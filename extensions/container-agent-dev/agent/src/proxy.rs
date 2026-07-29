//! Device loopback registry proxy (design D3/D9; task 6.2).
//!
//! The device container engine (docker/podman) pulls from a **plain-HTTP
//! loopback** registry this module serves on `127.0.0.1:<port>`. Engines
//! auto-trust a loopback registry, so the device needs NO insecure-registry
//! flag and NO per-engine trust/cert file — that is how the two "NO ..."
//! guarantees are met: by serving loopback HTTP we need zero engine trust
//! config at all.
//!
//! Each engine read (`GET`/`HEAD` on `/v2/…/manifests/…` and
//! `/v2/…/blobs/…`, plus the `GET /v2/` ping) is forwarded to the host's
//! dedicated **bulk HTTPS listener** (D9) over a rustls client pinned to ONLY
//! the bootstrap CA (reusing [`crate::build_tls_config`]), carrying the
//! read/control token as `Authorization: Bearer …`. Blob bytes ride THIS
//! HTTPS path, never the control WebSocket (D9 splits bulk from control).
//!
//! Fail-closed pre-stream (M-A / H-2 / M-3): the UPSTREAM status is inspected
//! BEFORE any engine-facing `200` is opened. On an upstream `2xx` we open the
//! `2xx` and stream the body through. On an upstream `401`/`403` we do NOT open
//! a `200`: we return a clean gateway error that carries neither the upstream
//! status semantics nor its `WWW-Authenticate` header, and we raise a shared
//! "needs re-bootstrap" flag the agent surfaces (the agent does not loop
//! silently). There is NO token refresh/renewal endpoint — a new token comes
//! only from a host re-`up` re-bootstrap. The mid-stream 401 case is PREVENTED
//! by the host's drain-based grace-overlap (D5), not signalled: OCI/HTTP have
//! no "terminal, do not retry" wire signal, so if the drain ceiling is ever
//! exceeded the agent's mid-stream behavior is best-effort (the stream simply
//! cuts), explicitly NOT a load-bearing terminal signal. The upstream
//! `WWW-Authenticate` is never proxied back to the engine under any status.
//!
//! # Trust assumptions
//!
//! **The loopback listener performs no device-side authorization.** Any local
//! process that can reach `127.0.0.1:<port>` gets its `GET`/`HEAD` forwarded
//! upstream under the agent's `Authorization: Bearer <read_token>`, so the read
//! token is effectively shared with every process on the device. That is
//! deliberate, and it is the same trust boundary the "no engine trust config"
//! guarantee buys: the engine authenticates to this proxy by being local, and
//! adding a device-side gate would need a credential the engine would then have
//! to be configured with — reintroducing exactly the per-engine trust file the
//! design exists to avoid. What bounds the exposure is that the socket is
//! loopback-only (never `0.0.0.0`), the method allowlist is GET/HEAD so a local
//! process can only read, and the whole extension is dev-only and gated on a
//! bootstrap the host must deliver. A device running untrusted local workloads
//! is outside this threat model; the mitigation there is not to install the dev
//! extension.
//!
//! **Each upstream request opens a fresh TCP + TLS + HTTP/1 connection.** There
//! is no connection pool, so pulling a multi-layer image pays one full TLS
//! handshake per manifest and per blob. That is accepted rather than optimized:
//! a pool would need idle-connection eviction and reuse-after-error handling on
//! a dev-only path whose cost is a handful of handshakes per sync. Revisit it if
//! this proxy ever carries a production pull.

use std::convert::Infallible;
use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Incoming;
use hyper::header::HeaderValue;
use hyper::header::{AUTHORIZATION, HOST, RANGE};
use hyper::service::service_fn;
use hyper::{HeaderMap, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::pki_types::ServerName;
use tracing::{info, warn};

use crate::{Bootstrap, build_tls_config, endpoint_host};

/// The engine-facing response body: boxed so both streamed upstream bodies and
/// small static error bodies share one type.
type RespBody = BoxBody<Bytes, std::io::Error>;

/// Upstream headers copied through on a successful (`2xx`) read. Deliberately a
/// strict allowlist: `www-authenticate` (and every other header) is dropped
/// unless it is on this list, so an upstream auth header can never reach the
/// engine even on a `2xx`.
const PASSTHROUGH_HEADERS: &[&str] = &[
    "content-type",
    "content-length",
    "content-range",
    "accept-ranges",
    "docker-content-digest",
    "etag",
    "last-modified",
];

/// The plain-text body returned to the engine when the bulk upstream rejects
/// the read/control token. It carries no upstream status or header semantics.
const REBOOTSTRAP_MSG: &str =
    "bulk upstream rejected the read/control token; re-run `avocado container dev up`";

/// A source of inbound connections.
///
/// Exists so the accept-error retry policy can be tested; production always
/// uses the [`TcpListener`] impl below.
pub(crate) trait Accept: Send {
    type Conn: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin + 'static;

    fn accept(&self) -> impl Future<Output = std::io::Result<Self::Conn>> + Send;
}

impl Accept for TcpListener {
    type Conn = tokio::net::TcpStream;

    async fn accept(&self) -> std::io::Result<Self::Conn> {
        TcpListener::accept(self)
            .await
            .map(|(stream, _peer)| stream)
    }
}

/// Pause after a failed `accept()` before trying again, so a transient error
/// does not turn the serve loop into a busy spin.
const ACCEPT_ERROR_BACKOFF: Duration = Duration::from_millis(100);

/// How many consecutive `accept()` failures end the serve loop.
///
/// A transient failure should not kill the proxy, but a permanently bad
/// listener must not be retried forever: the task would never return, so the
/// process would never exit and `Restart=always` would have nothing to restart.
/// Returning after a run of failures hands recovery back to the supervisor.
const ACCEPT_ERROR_LIMIT: u32 = 32;

// ---------------------------------------------------------------------------
// Re-bootstrap status flag.
// ---------------------------------------------------------------------------

/// A shared, clonable "needs re-bootstrap" flag.
///
/// Raised when the bulk upstream rejects the read/control token (a stale token
/// after a host re-`up`). The agent surfaces it (task 6.1's WS loop emits a
/// `Status` frame); it is never a silent retry loop, and there is no renewal
/// endpoint — the token is refreshed only by a host re-`up` re-bootstrap.
#[derive(Clone, Default)]
pub(crate) struct ReBootstrap(Arc<AtomicBool>);

impl ReBootstrap {
    /// Raise the flag. Idempotent.
    pub(crate) fn raise(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether a re-bootstrap has been requested.
    pub(crate) fn is_raised(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

// ---------------------------------------------------------------------------
// Upstream abstraction (injectable for tests).
// ---------------------------------------------------------------------------

/// One upstream read response: the status is available BEFORE the body is
/// consumed, which is what lets the proxy fail closed pre-stream.
pub(crate) struct UpstreamResponse {
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
    pub(crate) body: RespBody,
}

/// The bulk-read upstream. Abstracted so the proxy logic is unit-testable with
/// an in-process fake, while [`HttpsUpstream`] carries the real pinned-CA TLS
/// leg used on the device.
pub(crate) trait Upstream: Send + Sync + 'static {
    fn fetch(
        &self,
        method: Method,
        path: String,
        range: Option<HeaderValue>,
    ) -> impl Future<Output = Result<UpstreamResponse>> + Send;
}

/// The real upstream: a fresh TLS connection per request to the host bulk
/// listener, pinned to the bootstrap CA, carrying the Bearer read/control
/// token. TLS is driven by `tokio-rustls` (ring) directly — never a
/// higher-level client that could drag in a second crypto provider.
pub(crate) struct HttpsUpstream {
    connector: TlsConnector,
    server_name: ServerName<'static>,
    /// `host:port` of the bulk listener — used for the TCP connect and the
    /// `Host` header.
    authority: String,
    read_token: String,
}

impl HttpsUpstream {
    /// Build the upstream from the bootstrap: pins the CA and captures the bulk
    /// endpoint + read/control token. Fails if the CA PEM is unusable or the
    /// endpoint host is not a valid TLS server name.
    pub(crate) fn from_bootstrap(bootstrap: &Bootstrap) -> Result<Self> {
        let tls_config = build_tls_config(&bootstrap.ca_cert_pem)?;
        let connector = TlsConnector::from(Arc::new(tls_config));
        let host = endpoint_host(&bootstrap.bulk_endpoint).to_string();
        let server_name = ServerName::try_from(host).context("invalid bulk TLS server name")?;
        Ok(Self {
            connector,
            server_name,
            authority: bootstrap.bulk_endpoint.clone(),
            read_token: bootstrap.read_token.clone(),
        })
    }
}

impl Upstream for HttpsUpstream {
    async fn fetch(
        &self,
        method: Method,
        path: String,
        range: Option<HeaderValue>,
    ) -> Result<UpstreamResponse> {
        let tcp = TcpStream::connect(&self.authority)
            .await
            .with_context(|| format!("connecting to bulk listener {}", self.authority))?;
        let tls = self
            .connector
            .connect(self.server_name.clone(), tcp)
            .await
            .context("bulk TLS handshake (pinned CA)")?;

        let io = TokioIo::new(tls);
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
            .await
            .context("bulk HTTP/1 handshake")?;
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                warn!(error = %e, "bulk upstream connection closed with error");
            }
        });

        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header(HOST, &self.authority)
            .header(AUTHORIZATION, format!("Bearer {}", self.read_token));
        if let Some(range) = range {
            builder = builder.header(RANGE, range);
        }
        let request = builder
            .body(Empty::<Bytes>::new())
            .context("building upstream request")?;

        let response = sender
            .send_request(request)
            .await
            .context("sending upstream request")?;

        let status = response.status();
        let headers = response.headers().clone();
        let body = response.into_body().map_err(std::io::Error::other).boxed();
        Ok(UpstreamResponse {
            status,
            headers,
            body,
        })
    }
}

// ---------------------------------------------------------------------------
// Proxy logic.
// ---------------------------------------------------------------------------

/// Build a small static-body engine response (used for every gateway error).
fn gateway_error(status: StatusCode, message: &'static str) -> Response<RespBody> {
    let body = Full::new(Bytes::from_static(message.as_bytes()))
        .map_err(|never| match never {})
        .boxed();
    Response::builder()
        .status(status)
        .header(hyper::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(body)
        .expect("static gateway error response is always valid")
}

/// Core read proxy. Operates on already-extracted request parts so it is
/// unit-testable without constructing a hyper `Incoming` body.
///
/// Fail-closed pre-stream: the upstream status is fully classified before any
/// engine-facing `2xx` (and its body) is opened.
pub(crate) async fn proxy_read<U: Upstream>(
    method: Method,
    path: String,
    range: Option<HeaderValue>,
    upstream: &U,
    rebootstrap: &ReBootstrap,
) -> Response<RespBody> {
    // The device only ever reads. Anything else never reaches the host.
    if method != Method::GET && method != Method::HEAD {
        return gateway_error(
            StatusCode::METHOD_NOT_ALLOWED,
            "only GET and HEAD are proxied",
        );
    }

    match upstream.fetch(method, path, range).await {
        Ok(upstream) if upstream.status.is_success() => {
            // Only now, with a confirmed 2xx, do we open the engine response
            // and stream the body through. Headers are a strict allowlist —
            // `WWW-Authenticate` is never among them.
            let mut builder = Response::builder().status(upstream.status);
            for name in PASSTHROUGH_HEADERS {
                if let Some(value) = upstream.headers.get(*name) {
                    builder = builder.header(*name, value);
                }
            }
            builder
                .body(upstream.body)
                .expect("engine success response is always valid")
        }
        Ok(upstream)
            if upstream.status == StatusCode::UNAUTHORIZED
                || upstream.status == StatusCode::FORBIDDEN =>
        {
            // Fail-closed: no 200 was opened, so no body has streamed. Raise
            // the re-bootstrap status and return a CLEAN gateway error that
            // leaks neither the upstream 401/403 nor its WWW-Authenticate.
            rebootstrap.raise();
            warn!(
                upstream_status = %upstream.status,
                "bulk upstream rejected the read/control token; raising re-bootstrap status"
            );
            gateway_error(StatusCode::BAD_GATEWAY, REBOOTSTRAP_MSG)
        }
        Ok(upstream) => {
            // Any other non-2xx is treated as transient. Surface a gateway
            // error the engine may retry; do NOT raise re-bootstrap.
            warn!(upstream_status = %upstream.status, "bulk upstream returned a transient error");
            gateway_error(StatusCode::BAD_GATEWAY, "bulk upstream returned an error")
        }
        Err(e) => {
            warn!(error = %e, "bulk upstream request failed");
            gateway_error(StatusCode::BAD_GATEWAY, "bulk upstream unreachable")
        }
    }
}

// ---------------------------------------------------------------------------
// Loopback listener.
// ---------------------------------------------------------------------------

/// Bind the loopback registry socket. Always `127.0.0.1` (never `0.0.0.0`): the
/// registry is device-local only, which is what keeps it trust-free for the
/// engine and unreachable from the LAN.
pub(crate) async fn bind_loopback(port: u16) -> Result<TcpListener> {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding loopback registry on {addr}"))
}

/// Serve the loopback registry over `listener`, forwarding every read to
/// `upstream`. Runs until the listener errors.
/// The accept loop, over anything that can accept connections.
///
/// Generic over [`Accept`] rather than taking a `TcpListener` so the retry
/// policy is reachable from a test: forcing repeated `accept()` failures on a
/// real listener means corrupting its fd, which is not portable. Mirrors the
/// [`Upstream`] seam this module already uses for the same reason.
pub(crate) async fn serve_on<L: Accept, U: Upstream>(
    listener: L,
    upstream: Arc<U>,
    rebootstrap: ReBootstrap,
) -> Result<()> {
    // Consecutive accept failures. Reset by any successful accept, so a run has
    // to be genuinely unbroken to reach the limit.
    let mut consecutive_errors: u32 = 0;
    loop {
        let stream = match listener.accept().await {
            Ok(stream) => {
                consecutive_errors = 0;
                stream
            }
            // A single accept error (a peer that vanished between the SYN and
            // the accept, a momentary EMFILE) says nothing about the listener,
            // and killing the proxy over one would leave every later Sync pull
            // failing connection-refused. Retry those.
            //
            // But retrying forever is its own failure: on a permanently bad
            // listener the task never returns, so the process never exits and
            // `Restart=always` has nothing to restart - the proxy is dead and
            // the supervisor cannot tell. Give up after an unbroken run and let
            // the error propagate so systemd can do its job.
            Err(e) => {
                consecutive_errors += 1;
                if consecutive_errors >= ACCEPT_ERROR_LIMIT {
                    return Err(e).with_context(|| {
                        format!("accepting loopback connections failed {consecutive_errors} times in a row")
                    });
                }
                warn!(
                    error = %e,
                    consecutive_errors,
                    "accepting loopback connection failed; retrying"
                );
                sleep(ACCEPT_ERROR_BACKOFF).await;
                continue;
            }
        };
        let upstream = upstream.clone();
        let rebootstrap = rebootstrap.clone();
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req: Request<Incoming>| {
                let upstream = upstream.clone();
                let rebootstrap = rebootstrap.clone();
                async move {
                    let method = req.method().clone();
                    let path = req
                        .uri()
                        .path_and_query()
                        .map(|pq| pq.as_str().to_owned())
                        .unwrap_or_else(|| req.uri().path().to_owned());
                    let range = req.headers().get(RANGE).cloned();
                    Ok::<_, Infallible>(
                        proxy_read(method, path, range, upstream.as_ref(), &rebootstrap).await,
                    )
                }
            });
            if let Err(e) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service)
                .await
            {
                warn!(error = %e, "loopback registry connection error");
            }
        });
    }
}

/// Bind `127.0.0.1:port` and serve the loopback registry. The real device
/// entry point (called from `main`).
pub(crate) async fn serve_loopback<U: Upstream>(
    port: u16,
    upstream: Arc<U>,
    rebootstrap: ReBootstrap,
) -> Result<()> {
    let listener = bind_loopback(port).await?;
    match listener.local_addr() {
        Ok(addr) => info!(%addr, "loopback registry proxy listening"),
        Err(_) => info!("loopback registry proxy listening"),
    }
    serve_on(listener, upstream, rebootstrap).await
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    /// A listener whose `accept()` fails a scripted number of times.
    ///
    /// `fail_forever` models a listener that has gone permanently bad - the case
    /// the retry bound exists for. `failing_then_recovering` models a transient run
    /// followed by recovery, so the counter's reset can be observed.
    #[derive(Clone)]
    struct ScriptedListener {
        attempts: Arc<AtomicUsize>,
        fail_forever: bool,
        succeed_after: usize,
    }

    impl ScriptedListener {
        fn failing_forever() -> Self {
            Self {
                attempts: Arc::new(AtomicUsize::new(0)),
                fail_forever: true,
                succeed_after: 0,
            }
        }

        fn failing_then_recovering(succeed_after: usize) -> Self {
            Self {
                attempts: Arc::new(AtomicUsize::new(0)),
                fail_forever: false,
                succeed_after,
            }
        }

        fn attempts(&self) -> usize {
            self.attempts.load(Ordering::SeqCst)
        }
    }

    impl Accept for ScriptedListener {
        type Conn = tokio::io::DuplexStream;

        async fn accept(&self) -> std::io::Result<Self::Conn> {
            let n = self.attempts.fetch_add(1, Ordering::SeqCst);
            if self.fail_forever || n < self.succeed_after {
                return Err(std::io::Error::other("scripted accept failure"));
            }
            if n == self.succeed_after {
                // One success in the middle: enough to reset the counter.
                let (a, _b) = tokio::io::duplex(64);
                return Ok(a);
            }
            if n <= self.succeed_after * 2 {
                // A SECOND failing run after the success. Both runs are one
                // short of the limit, so the loop survives only if the success
                // reset the counter - cumulatively they are well past it.
                return Err(std::io::Error::other("scripted accept failure"));
            }
            // Then park, the way a real listener waits for the next peer. A
            // listener that kept returning instantly would spin the loop.
            std::future::pending().await
        }
    }

    /// A listener that fails forever must end the loop rather than spin, so the
    /// process exits and `Restart=always` can recover it. Before the bound was
    /// added this returned never, which is what made the supervisor useless.
    #[tokio::test(start_paused = true)]
    async fn a_permanently_failing_listener_ends_the_serve_loop() {
        let listener = ScriptedListener::failing_forever();
        let upstream = Arc::new(ok_upstream());
        let rebootstrap = ReBootstrap::default();

        let started = tokio::time::Instant::now();
        let err = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            serve_on(listener, upstream, rebootstrap),
        )
        .await
        .expect("the loop must terminate rather than spin forever")
        .expect_err("a permanently failing listener must surface an error");

        assert!(
            err.to_string().contains("in a row"),
            "the error should name the consecutive-failure run: {err}"
        );

        // The backoff is the only thing standing between a sustained accept
        // error and a busy spin, and without this it had no coverage at all -
        // deleting the sleep left every proxy test green. Time is virtual here
        // (`start_paused`), so this asserts the sleep actually elapsed without
        // the suite paying for it.
        let elapsed = started.elapsed();
        let want = ACCEPT_ERROR_BACKOFF * (ACCEPT_ERROR_LIMIT - 1);
        assert!(
            elapsed >= want,
            "each retry must back off: expected at least {want:?} across \
             {ACCEPT_ERROR_LIMIT} attempts, only {elapsed:?} elapsed"
        );
    }

    /// The counter must reset on a successful accept, or a long-lived proxy that
    /// sees scattered transient errors would eventually shut itself down for no
    /// reason. Fails if the reset is removed.
    #[tokio::test(start_paused = true)]
    async fn a_successful_accept_resets_the_error_counter() {
        // More total failures than the limit, but never a consecutive run that
        // reaches it: two batches either side of a success.
        let listener = ScriptedListener::failing_then_recovering(ACCEPT_ERROR_LIMIT as usize - 1);
        let observed = listener.clone();
        let upstream = Arc::new(ok_upstream());

        let handle = tokio::spawn(serve_on(listener, upstream, ReBootstrap::default()));

        // Wait for both failing runs plus the success between them to elapse.
        let want = 2 * (ACCEPT_ERROR_LIMIT as usize - 1) + 1;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while observed.attempts() < want && !handle.is_finished() {
            assert!(std::time::Instant::now() < deadline, "test timed out");
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }

        assert!(
            !handle.is_finished(),
            "the loop gave up after {} accepts: a success between two runs of \
             {} failures must reset the counter, since neither run reaches the \
             limit of {ACCEPT_ERROR_LIMIT} on its own",
            observed.attempts(),
            ACCEPT_ERROR_LIMIT - 1,
        );
        handle.abort();
    }

    /// A minimal upstream for the accept-loop tests, which never reach it.
    fn ok_upstream() -> FakeUpstream {
        FakeUpstream {
            status: StatusCode::OK,
            headers: Vec::new(),
            body: "",
        }
    }

    use http_body_util::BodyExt;
    use hyper::body::Incoming;
    use hyper::service::service_fn;
    use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};
    use tokio::net::TcpStream;
    use tokio_rustls::TlsAcceptor;
    use tokio_rustls::rustls::pki_types::{
        CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName,
    };

    // --- helpers -----------------------------------------------------------

    fn full(bytes: impl Into<Bytes>) -> RespBody {
        Full::new(bytes.into())
            .map_err(|never| match never {})
            .boxed()
    }

    /// A CA plus a leaf it signed for `127.0.0.1`.
    struct Ca {
        ca_pem: String,
        leaf_cert: CertificateDer<'static>,
        leaf_key: PrivateKeyDer<'static>,
    }

    fn make_ca() -> Ca {
        let mut ca_params = CertificateParams::new(Vec::new()).expect("ca params");
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let ca_key = KeyPair::generate().expect("ca key");
        let ca_cert = ca_params.self_signed(&ca_key).expect("self-sign ca");

        let leaf_params =
            CertificateParams::new(vec!["127.0.0.1".to_string()]).expect("leaf params");
        let leaf_key = KeyPair::generate().expect("leaf key");
        let leaf_cert = leaf_params
            .signed_by(&leaf_key, &ca_cert, &ca_key)
            .expect("sign leaf");

        Ca {
            ca_pem: ca_cert.pem(),
            leaf_cert: leaf_cert.der().clone(),
            leaf_key: PrivateKeyDer::from(PrivatePkcs8KeyDer::from(leaf_key.serialize_der())),
        }
    }

    /// Start an in-process TLS "host bulk listener" presenting `cert`/`key`.
    /// A path containing `unauth` returns 401 + `WWW-Authenticate`; anything
    /// else returns 200 with a known body.
    async fn start_tls_upstream(
        cert: CertificateDer<'static>,
        key: PrivateKeyDer<'static>,
    ) -> SocketAddr {
        let provider = Arc::new(tokio_rustls::rustls::crypto::ring::default_provider());
        let config = tokio_rustls::rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("server protocol versions")
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .expect("server single cert");
        let acceptor = TlsAcceptor::from(Arc::new(config));

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind tls upstream");
        let addr = listener.local_addr().expect("upstream addr");

        tokio::spawn(async move {
            loop {
                let (tcp, _) = match listener.accept().await {
                    Ok(pair) => pair,
                    Err(_) => return,
                };
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    let tls = match acceptor.accept(tcp).await {
                        Ok(tls) => tls,
                        Err(_) => return,
                    };
                    let io = TokioIo::new(tls);
                    let service = service_fn(|req: Request<Incoming>| async move {
                        let resp = if req.uri().path().contains("unauth") {
                            Response::builder()
                                .status(StatusCode::UNAUTHORIZED)
                                .header("www-authenticate", "Bearer realm=\"https://evil/token\"")
                                .body(full("denied"))
                                .unwrap()
                        } else {
                            Response::builder()
                                .status(StatusCode::OK)
                                .header("content-type", "application/octet-stream")
                                .header("docker-content-digest", "sha256:deadbeef")
                                .body(full("BLOBDATA"))
                                .unwrap()
                        };
                        Ok::<_, Infallible>(resp)
                    });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, service)
                        .await;
                });
            }
        });

        addr
    }

    /// A plain-HTTP (no TLS) GET against the loopback registry, standing in for
    /// the device engine. Returns status, headers, and the collected body.
    async fn plain_get(addr: SocketAddr, path: &str) -> (StatusCode, HeaderMap, Bytes) {
        let tcp = TcpStream::connect(addr).await.expect("connect loopback");
        let io = TokioIo::new(tcp);
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
            .await
            .expect("loopback handshake");
        tokio::spawn(async move {
            let _ = conn.await;
        });
        let request = Request::builder()
            .method(Method::GET)
            .uri(path)
            .header(HOST, addr.to_string())
            .body(Empty::<Bytes>::new())
            .expect("loopback request");
        let response = sender
            .send_request(request)
            .await
            .expect("loopback response");
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect")
            .to_bytes();
        (status, headers, body)
    }

    /// A fake upstream returning a canned response, for the pure proxy-logic
    /// tests that need no network.
    struct FakeUpstream {
        status: StatusCode,
        headers: Vec<(&'static str, &'static str)>,
        body: &'static str,
    }

    impl Upstream for FakeUpstream {
        async fn fetch(
            &self,
            _method: Method,
            _path: String,
            _range: Option<HeaderValue>,
        ) -> Result<UpstreamResponse> {
            let mut headers = HeaderMap::new();
            for (name, value) in &self.headers {
                headers.insert(
                    hyper::header::HeaderName::from_static(name),
                    HeaderValue::from_static(value),
                );
            }
            Ok(UpstreamResponse {
                status: self.status,
                headers,
                body: full(self.body),
            })
        }
    }

    async fn collect(resp: Response<RespBody>) -> (StatusCode, HeaderMap, Bytes) {
        let status = resp.status();
        let headers = resp.headers().clone();
        let body = resp
            .into_body()
            .collect()
            .await
            .expect("collect")
            .to_bytes();
        (status, headers, body)
    }

    // --- proxy-logic tests -------------------------------------------------

    #[tokio::test]
    async fn proxy_forwards_upstream_200_body_verbatim() {
        let upstream = FakeUpstream {
            status: StatusCode::OK,
            headers: vec![
                ("content-type", "application/octet-stream"),
                ("docker-content-digest", "sha256:abc"),
            ],
            body: "LAYER-BYTES",
        };
        let rebootstrap = ReBootstrap::default();
        let resp = proxy_read(
            Method::GET,
            "/v2/app/blobs/sha256:abc".to_string(),
            None,
            &upstream,
            &rebootstrap,
        )
        .await;
        let (status, headers, body) = collect(resp).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"LAYER-BYTES");
        assert_eq!(
            headers.get("content-type").unwrap(),
            "application/octet-stream"
        );
        assert_eq!(headers.get("docker-content-digest").unwrap(), "sha256:abc");
        assert!(!rebootstrap.is_raised());
    }

    #[tokio::test]
    async fn proxy_401_returns_clean_error_no_www_authenticate_and_raises_rebootstrap() {
        let upstream = FakeUpstream {
            status: StatusCode::UNAUTHORIZED,
            headers: vec![("www-authenticate", "Bearer realm=\"https://evil/token\"")],
            body: "denied",
        };
        let rebootstrap = ReBootstrap::default();
        let resp = proxy_read(
            Method::GET,
            "/v2/app/blobs/sha256:abc".to_string(),
            None,
            &upstream,
            &rebootstrap,
        )
        .await;
        let (status, headers, body) = collect(resp).await;

        // Engine never sees a 401 — it gets a clean gateway error.
        assert_ne!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        // The upstream WWW-Authenticate is never proxied back.
        assert!(headers.get("www-authenticate").is_none());
        // The clean body carries no upstream auth semantics.
        assert_eq!(&body[..], REBOOTSTRAP_MSG.as_bytes());
        // The re-bootstrap status is raised for the agent to surface.
        assert!(rebootstrap.is_raised());
    }

    #[tokio::test]
    async fn proxy_transient_5xx_is_gateway_error_without_rebootstrap() {
        let upstream = FakeUpstream {
            status: StatusCode::SERVICE_UNAVAILABLE,
            headers: vec![],
            body: "later",
        };
        let rebootstrap = ReBootstrap::default();
        let resp = proxy_read(
            Method::GET,
            "/v2/app/manifests/dev".to_string(),
            None,
            &upstream,
            &rebootstrap,
        )
        .await;
        let (status, _headers, _body) = collect(resp).await;

        assert_eq!(status, StatusCode::BAD_GATEWAY);
        // A transient error is retryable, not a terminal re-bootstrap signal.
        assert!(!rebootstrap.is_raised());
    }

    // --- pinned-CA tests ---------------------------------------------------

    #[tokio::test]
    async fn pinned_ca_accepts_matching_leaf_and_streams_200() {
        let ca = make_ca();
        let addr = start_tls_upstream(ca.leaf_cert.clone(), ca.leaf_key.clone_key()).await;

        let bootstrap = Bootstrap {
            bulk_endpoint: addr.to_string(),
            read_token: "read-token".to_string(),
            ca_cert_pem: ca.ca_pem.clone(),
            ws_endpoint: String::new(),
        };
        let upstream = HttpsUpstream::from_bootstrap(&bootstrap).expect("build upstream");

        let resp = upstream
            .fetch(Method::GET, "/v2/app/blobs/sha256:abc".to_string(), None)
            .await
            .expect("pinned CA accepts its own leaf");
        assert!(resp.status.is_success());
        let body = resp.body.collect().await.expect("collect").to_bytes();
        assert_eq!(&body[..], b"BLOBDATA");
    }

    #[tokio::test]
    async fn pinned_ca_rejects_leaf_from_a_different_ca() {
        // Server presents a leaf signed by CA "B"...
        let server_ca = make_ca();
        let addr =
            start_tls_upstream(server_ca.leaf_cert.clone(), server_ca.leaf_key.clone_key()).await;

        // ...but the client pins a DIFFERENT CA "A".
        let client_ca = make_ca();
        let bootstrap = Bootstrap {
            bulk_endpoint: addr.to_string(),
            read_token: "read-token".to_string(),
            ca_cert_pem: client_ca.ca_pem.clone(),
            ws_endpoint: String::new(),
        };
        let upstream = HttpsUpstream::from_bootstrap(&bootstrap).expect("build upstream");

        let result = upstream
            .fetch(Method::GET, "/v2/app/blobs/sha256:abc".to_string(), None)
            .await;
        assert!(
            result.is_err(),
            "handshake must fail: a host leaf not signed by the bootstrap CA is rejected"
        );
    }

    // --- end-to-end loopback test ------------------------------------------

    #[tokio::test]
    async fn loopback_is_plain_http_and_proxies_over_tls_upstream() {
        let ca = make_ca();
        let upstream_addr = start_tls_upstream(ca.leaf_cert.clone(), ca.leaf_key.clone_key()).await;

        let bootstrap = Bootstrap {
            bulk_endpoint: upstream_addr.to_string(),
            read_token: "read-token".to_string(),
            ca_cert_pem: ca.ca_pem.clone(),
            ws_endpoint: String::new(),
        };
        let upstream = Arc::new(HttpsUpstream::from_bootstrap(&bootstrap).expect("build upstream"));
        let rebootstrap = ReBootstrap::default();

        let listener = bind_loopback(0).await.expect("bind loopback");
        let loopback_addr = listener.local_addr().expect("loopback addr");
        // The loopback socket must be 127.0.0.1, never 0.0.0.0.
        assert!(loopback_addr.ip().is_loopback());

        tokio::spawn(serve_on(listener, upstream, rebootstrap.clone()));

        // The engine speaks PLAIN HTTP to the loopback with zero TLS/trust
        // config; the proxy forwards over the pinned-CA HTTPS upstream.
        let (status, headers, body) = plain_get(loopback_addr, "/v2/app/blobs/sha256:abc").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"BLOBDATA");
        assert_eq!(
            headers.get("docker-content-digest").unwrap(),
            "sha256:deadbeef"
        );
        assert!(!rebootstrap.is_raised());

        // An upstream 401 surfaces as a clean gateway error with no
        // WWW-Authenticate, and raises the re-bootstrap status.
        let (status, headers, _body) = plain_get(loopback_addr, "/v2/app/blobs/unauth").await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert!(headers.get("www-authenticate").is_none());
        assert!(rebootstrap.is_raised());
    }

    #[tokio::test]
    async fn bind_loopback_binds_localhost_not_wildcard() {
        let listener = bind_loopback(0).await.expect("bind loopback");
        let addr = listener.local_addr().expect("addr");
        assert!(addr.ip().is_loopback());
        assert_eq!(addr.ip(), std::net::IpAddr::V4(Ipv4Addr::LOCALHOST));
    }

    // Guard for ServerName import usage in non-test builds is unnecessary; keep
    // the import honest by asserting a loopback IP parses as a server name.
    #[test]
    fn loopback_ip_is_a_valid_server_name() {
        assert!(ServerName::try_from("127.0.0.1").is_ok());
    }
}
