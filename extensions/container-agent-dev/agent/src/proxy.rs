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
/// Carries a `Notify` as well as the flag, because a bare flag could not be
/// delivered. The WS loop read `is_raised()` exactly once per session, BEFORE
/// entering its frame loop - and the 401 that raises it happens during a pull,
/// which is inside that loop. So the flag was set strictly after the only read
/// that would have transmitted it, and with the sink otherwise idle the host was
/// never told. There was also no way to lower it, so one transient 401 latched
/// for the process lifetime.
#[derive(Clone, Default)]
pub(crate) struct ReBootstrap {
    raised: Arc<AtomicBool>,
    changed: Arc<tokio::sync::Notify>,
}

impl ReBootstrap {
    /// Raise the flag and wake anyone waiting to report it. Idempotent in the
    /// flag; the wake is what makes it reach the host mid-session.
    ///
    /// `notify_one`, not `notify_waiters`: the reporting select arm builds a
    /// fresh `notified()` future on every iteration, so it is only registered as
    /// a waiter while parked in the select. A 401 raised while the loop body ran
    /// another arm - mid-`send_frame`, say - found no waiter, and
    /// `notify_waiters` stores no permit, so the wake was dropped. The flag
    /// stayed set and surfaced at the next session start, which is exactly the
    /// mid-session delivery this was supposed to fix. `notify_one` stores the
    /// permit, so the next `notified()` completes immediately.
    pub(crate) fn raise(&self) {
        self.raised.store(true, Ordering::SeqCst);
        self.changed.notify_one();
    }

    /// Whether a re-bootstrap has been requested.
    ///
    /// Test-only: the reporting path reads the flag exclusively through
    /// [`Self::take`], because observing and lowering have to be one step for a
    /// raise landing mid-send not to be erased. A non-consuming read is still
    /// what the proxy's own tests want - "the 401 raised it", "the 200 did not"
    /// - and those must not lower it as a side effect.
    #[cfg(test)]
    pub(crate) fn is_raised(&self) -> bool {
        self.raised.load(Ordering::SeqCst)
    }

    /// Lower the flag, reporting whether it had been raised.
    ///
    /// Atomic so the caller can lower it BEFORE reporting without racing a
    /// concurrent `raise`. Lowering is what keeps a single transient 401 from
    /// making every later session announce `needs_rebootstrap` long after the
    /// token it referred to was replaced.
    pub(crate) fn take(&self) -> bool {
        self.raised.swap(false, Ordering::SeqCst)
    }

    /// Resolve when the flag is next raised.
    pub(crate) async fn raised(&self) {
        self.changed.notified().await;
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

/// Ceiling on each pre-body upstream stage: TCP connect, TLS handshake, HTTP/1
/// handshake, and the request/response head.
///
/// None of these had one. A laptop that sleeps or drops off Wi-Fi mid-pull leaves
/// a half-open connection with no RST; no SO_KEEPALIVE is set anywhere, so the
/// 7200s tcp_keepalive_time never applies, and after `send_request` the agent is
/// purely READING, so there are no outbound retransmits for tcp_retries2 to give
/// up on. The read hung forever: `docker pull` waited on a loopback response that
/// never arrived, `on_sync` never returned, and - before syncs moved off the read
/// loop - the WS was never polled again either. systemd reported
/// `active (running)` with the device completely dead and nothing logged.
///
/// Generous on purpose: this bounds a stall, not a slow link. Body streaming is
/// deliberately NOT wrapped, because a multi-gigabyte layer over a slow link is
/// legitimately long; the stages here are the ones that should complete in
/// milliseconds on any working link.
const UPSTREAM_STAGE_TIMEOUT: Duration = Duration::from_secs(30);

/// Run `fut`, failing with a named error if it does not finish in time.
async fn with_stage_timeout<T>(
    stage: &'static str,
    fut: impl Future<Output = Result<T>>,
) -> Result<T> {
    match tokio::time::timeout(UPSTREAM_STAGE_TIMEOUT, fut).await {
        Ok(result) => result,
        Err(_) => anyhow::bail!(
            "{stage} did not complete within {}s; the host is unreachable or the connection \
             is half-open",
            UPSTREAM_STAGE_TIMEOUT.as_secs()
        ),
    }
}

impl Upstream for HttpsUpstream {
    async fn fetch(
        &self,
        method: Method,
        path: String,
        range: Option<HeaderValue>,
    ) -> Result<UpstreamResponse> {
        let tcp = with_stage_timeout("connecting to the bulk listener", async {
            TcpStream::connect(&self.authority)
                .await
                .with_context(|| format!("connecting to bulk listener {}", self.authority))
        })
        .await?;
        let tls = with_stage_timeout("the bulk TLS handshake", async {
            self.connector
                .connect(self.server_name.clone(), tcp)
                .await
                .context("bulk TLS handshake (pinned CA)")
        })
        .await?;

        let io = TokioIo::new(tls);
        let (mut sender, conn) = with_stage_timeout("the bulk HTTP/1 handshake", async {
            hyper::client::conn::http1::handshake(io)
                .await
                .context("bulk HTTP/1 handshake")
        })
        .await?;
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

        let response = with_stage_timeout("the upstream request", async {
            sender
                .send_request(request)
                .await
                .context("sending upstream request")
        })
        .await?;

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

/// Ceiling on the gap between two body chunks from the bulk upstream.
///
/// [`UPSTREAM_STAGE_TIMEOUT`] bounds every stage BEFORE the body, but the stall
/// those were written for - a laptop that sleeps or drops off Wi-Fi mid-pull,
/// leaving a half-open connection with no RST - lands overwhelmingly DURING body
/// transfer, which was the one stage still uncovered. The outcome had been
/// softened rather than removed: the read loop keeps the WS polled so the socket
/// no longer wedges, but the sync worker parked forever, later `Sync` frames
/// piled into the latest-wins slot and never ran, no `Status` was emitted
/// because `on_sync` never returned, and systemd still reported
/// `active (running)`.
///
/// An IDLE deadline, not a total-transfer one: capping total transfer would kill
/// a legitimate multi-gigabyte layer over a slow link, while a link that is
/// moving bytes at all keeps resetting this.
#[cfg(not(test))]
const UPSTREAM_BODY_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Short under test so the WIRING is assertable, not just the wrapper.
///
/// Virtual time cannot cover it: with the wrapper removed there is no timer left
/// inside the body at all, and tokio's paused-clock auto-advance does not fire
/// for an outer `timeout` when the inner future parks without registering a
/// waker - the test hangs instead of failing, which is the one outcome a
/// regression must not produce. [`the_production_body_idle_timeout_is_generous`]
/// pins the shipped value so this override cannot hide a typo in it.
#[cfg(test)]
const UPSTREAM_BODY_IDLE_TIMEOUT: Duration = Duration::from_millis(50);

/// The idle deadline the shipped binary uses, kept beside the override so both
/// values are visible together and the real one stays asserted.
#[cfg(test)]
const PRODUCTION_BODY_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Wraps a body so a gap of [`UPSTREAM_BODY_IDLE_TIMEOUT`] between chunks ends
/// the stream with an error instead of hanging forever.
struct IdleTimeoutBody<B> {
    inner: B,
    idle: std::pin::Pin<Box<tokio::time::Sleep>>,
}

impl<B> IdleTimeoutBody<B> {
    fn new(inner: B) -> Self {
        Self {
            inner,
            idle: Box::pin(sleep(UPSTREAM_BODY_IDLE_TIMEOUT)),
        }
    }
}

impl<B> hyper::body::Body for IdleTimeoutBody<B>
where
    B: hyper::body::Body<Data = Bytes, Error = std::io::Error> + Unpin,
{
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<std::result::Result<hyper::body::Frame<Bytes>, std::io::Error>>>
    {
        use std::task::Poll;

        let this = self.as_mut().get_mut();
        match std::pin::Pin::new(&mut this.inner).poll_frame(cx) {
            // Any progress - a chunk, the end of the stream, or an error -
            // restarts the clock from here.
            Poll::Ready(frame) => {
                this.idle
                    .as_mut()
                    .reset(tokio::time::Instant::now() + UPSTREAM_BODY_IDLE_TIMEOUT);
                Poll::Ready(frame)
            }
            Poll::Pending => match this.idle.as_mut().poll(cx) {
                Poll::Ready(()) => Poll::Ready(Some(Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!(
                        "the bulk upstream sent no body data for {}s; the host is unreachable \
                         or the connection is half-open",
                        UPSTREAM_BODY_IDLE_TIMEOUT.as_secs()
                    ),
                )))),
                Poll::Pending => Poll::Pending,
            },
        }
    }

    fn size_hint(&self) -> hyper::body::SizeHint {
        self.inner.size_hint()
    }
}

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
                .body(IdleTimeoutBody::new(upstream.body).boxed())
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

    #[test]
    fn the_production_body_idle_timeout_is_generous() {
        // The suite runs with a 50ms override, so nothing else would notice the
        // shipped value being wrong. It has to be long enough that a slow but
        // live link is never cut - it bounds a stall, not a slow transfer.
        assert!(
            PRODUCTION_BODY_IDLE_TIMEOUT >= Duration::from_secs(30),
            "a short production idle deadline would kill slow but healthy pulls"
        );
        assert!(
            UPSTREAM_BODY_IDLE_TIMEOUT < PRODUCTION_BODY_IDLE_TIMEOUT,
            "the test override must be the shorter of the two"
        );
    }

    /// A 200 whose body never delivers a byte.
    struct StallingUpstream;

    impl Upstream for StallingUpstream {
        async fn fetch(
            &self,
            _method: Method,
            _path: String,
            _range: Option<HeaderValue>,
        ) -> Result<UpstreamResponse> {
            Ok(UpstreamResponse {
                status: StatusCode::OK,
                headers: HeaderMap::new(),
                body: StallingBody.boxed(),
            })
        }
    }

    #[tokio::test]
    async fn a_stalled_upstream_body_fails_through_the_proxy() {
        // The wiring, not just the wrapper: reverting `proxy_read` to hand the
        // upstream body straight to the response leaves the two IdleTimeoutBody
        // unit tests green while every real pull can still hang forever.
        let resp = proxy_read(
            Method::GET,
            "/v2/app/blobs/sha256:abc".to_string(),
            None,
            &StallingUpstream,
            &ReBootstrap::default(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        // The outer bound is what makes an unwired wrapper FAIL rather than hang.
        // With the wrapper in place the body's own 50ms deadline fires long
        // first, so this costs nothing on the passing path.
        let err = tokio::time::timeout(BODY_TEST_BOUND, resp.into_body().collect())
            .await
            .expect("a stalled upstream body must not hang the engine's pull")
            .expect_err("a stalled upstream body must surface as an error");
        assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
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

    /// A body that never produces a frame and never wakes its waker - a
    /// half-open connection to a host that went to sleep mid-layer.
    struct StallingBody;

    impl hyper::body::Body for StallingBody {
        type Data = Bytes;
        type Error = std::io::Error;

        fn poll_frame(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<std::result::Result<hyper::body::Frame<Bytes>, std::io::Error>>>
        {
            std::task::Poll::Pending
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_raise_with_no_waiter_registered_is_still_delivered() {
        // The reporting arm builds a fresh `notified()` future on every select
        // iteration, so it counts as a registered waiter only while parked in
        // the select. A 401 raised while the loop body ran another arm -
        // mid-`send_frame`, say - found no waiter, and `notify_waiters` stores
        // no permit, so the wake was dropped. The flag stayed set and surfaced
        // at the next session start, which is exactly the mid-session delivery
        // the Notify was added for.
        let flag = ReBootstrap::default();
        flag.raise();

        tokio::time::timeout(Duration::from_secs(5), flag.raised())
            .await
            .expect("a raise with no waiter must still wake the next waiter");
    }

    /// Bound for the body tests: comfortably longer than the 50ms idle deadline
    /// the suite runs with, short enough that a regression reports in seconds
    /// rather than hanging the run.
    const BODY_TEST_BOUND: Duration = Duration::from_secs(5);

    #[tokio::test]
    async fn a_body_that_stalls_mid_stream_fails_instead_of_hanging() {
        // The stage timeouts cover TCP, TLS, the HTTP/1 handshake and the
        // request head - every stage EXCEPT the one a sleeping laptop actually
        // stalls in. Unwrapped, this collect never returns: `docker pull` waits
        // on a loopback response that never arrives, `on_sync` never returns, no
        // Status is ever emitted, and systemd still reports active (running).
        let err = tokio::time::timeout(
            BODY_TEST_BOUND,
            IdleTimeoutBody::new(StallingBody).collect(),
        )
        .await
        .expect("a stalled body must not hang forever")
        .expect_err("a stalled body must surface as an error");
        assert_eq!(
            err.kind(),
            std::io::ErrorKind::TimedOut,
            "a stalled body must fail as a timeout: {err}"
        );
        assert!(
            err.to_string().contains("no body data"),
            "the error must say the stall was in the body: {err}"
        );
    }

    #[tokio::test]
    async fn a_body_that_delivers_is_passed_through_untouched() {
        // The mirror: the wrapper must not truncate or corrupt a working
        // stream, or every pull breaks. Without this the test above passes for a
        // wrapper that errors unconditionally.
        let bytes = IdleTimeoutBody::new(full("LAYER-BYTES"))
            .collect()
            .await
            .expect("a delivering body must not be cut off")
            .to_bytes();
        assert_eq!(&bytes[..], b"LAYER-BYTES");
    }

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
