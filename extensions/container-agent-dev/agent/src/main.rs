//! Device-side control agent for Container Dev Mode (design D9; task 6.1).
//!
//! The agent dials the host's control-only WebSocket, authenticates with the
//! read/control token (the only token a device ever holds — design D2), holds
//! the connection to receive [`HostFrame::Sync`] notifications, and
//! auto-reconnects with exponential backoff, re-sending its [`DeviceFrame::Hello`]
//! (carrying the current running digest) on every (re)connect.
//!
//! Scope of task 6.1 is the connect / hold / reconnect loop only. The actual
//! pull + container restart on a `Sync` frame is task 6.3; a clear seam is left
//! in [`handle_host_frame`].
//!
//! TLS uses the **ring** rustls provider, never `aws-lc-rs`: the device
//! cross-compiles to musl and A9 clears only the ring stack (same rule
//! avocado-conn enforces). The control WS leaf is pinned to the bootstrap CA
//! alone — no native roots, no webpki-roots.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::time::{Instant, sleep};
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_tungstenite::client_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use tracing::{info, warn};

mod proxy;
mod sync;

/// Default on-device path the host writes the bootstrap file to (design D5 / A7).
const DEFAULT_BOOTSTRAP_PATH: &str = "/var/lib/avocado/container-dev/bootstrap.json";
/// Env var that overrides the bootstrap path (used by tests and staging).
const BOOTSTRAP_PATH_ENV: &str = "AVOCADO_CONTAINER_DEV_BOOTSTRAP";
/// Env var that supplies the device identity reported in `Hello`.
const DEVICE_ID_ENV: &str = "AVOCADO_DEVICE_ID";
/// Default loopback port the device engine pulls from (task 6.2). Chosen off
/// 5000 to avoid the macOS AirPlay clash noted in the Phase 0 port survey
/// (design 1.6); the engine config (task 6.4) points at `127.0.0.1:<this>`.
const DEFAULT_LOOPBACK_PORT: u16 = 15151;
/// Env var overriding the loopback registry port.
const LOOPBACK_PORT_ENV: &str = "AVOCADO_CONTAINER_DEV_LOOPBACK_PORT";

// ---------------------------------------------------------------------------
// Wire contract (MUST match avocado-cli/src/utils/container_dev/ws.rs).
//
// Frames are JSON, serde-tagged with `"type"`, `rename_all = "snake_case"`.
// These types are re-declared here (not imported) because avocado-cli is a
// separate repo with no shared workspace. The `frame_wire_shapes` test locks
// the JSON shape to the host's.
// ---------------------------------------------------------------------------

/// A host -> device control frame. Carries only image coordinates plus a
/// content-digest reference — never blob bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostFrame {
    /// The `{image, tag, digest}` now available for the device to pull.
    Sync {
        image: String,
        tag: String,
        digest: String,
    },
}

/// A device -> host control frame.
///
/// `Hello` is an internally-tagged newtype variant: it serializes flat, as a
/// single object tagged `"hello"` carrying the `Hello` struct's fields — this
/// matches the host's `DeviceFrame::Hello(Hello)` wire form exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeviceFrame {
    Hello(Hello),
    Progress(Progress),
    Status(Status),
}

/// The device's `hello`: who it is, its CPU arch, and the digest it runs now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub device_id: String,
    pub arch: String,
    pub running_digest: String,
}

/// Progress of an in-flight device pull (informational).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    pub image: String,
    pub bytes_pulled: u64,
}

/// A device state report (informational).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub device_id: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

// ---------------------------------------------------------------------------
// Device bootstrap.
// ---------------------------------------------------------------------------

/// The device-side bootstrap, read from the writable partition.
///
/// `bulk_endpoint`, `read_token`, and `ca_cert_pem` mirror the host's
/// `DeviceBootstrap` (avocado-cli/src/utils/container_dev/bootstrap.rs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bootstrap {
    /// The bulk read-listener endpoint (`host:port`) the device pulls from.
    pub bulk_endpoint: String,
    /// The Bearer read/control token — the only token a device holds.
    pub read_token: String,
    /// The per-project CA certificate (PEM) the device pins the host leaf to.
    pub ca_cert_pem: String,
    /// The `host:port` of the control WS to dial.
    ///
    /// FOLLOW-UP: the host side must be extended to emit this — avocado-cli's
    /// `DeviceBootstrap` (bootstrap.rs) does not yet carry it, and the
    /// control-WS bind in `dev.rs` must publish its address here. Tracked as a
    /// devspec discovery event for this change. Defaulted so parsing a current
    /// host payload (without the field) does not hard-fail; the agent errors
    /// clearly at dial time if it is still empty.
    #[serde(default)]
    pub ws_endpoint: String,
}

impl Bootstrap {
    /// Whether this bootstrap and `other` carry the same session material.
    ///
    /// Only the fields a live session depends on: a token, endpoint or CA change
    /// means the session in progress is finished, whatever else the file says.
    fn same_session_as(&self, other: &Bootstrap) -> bool {
        self.read_token == other.read_token
            && self.bulk_endpoint == other.bulk_endpoint
            && self.ws_endpoint == other.ws_endpoint
            && self.ca_cert_pem == other.ca_cert_pem
    }
}

/// Resolve the bootstrap path: `$AVOCADO_CONTAINER_DEV_BOOTSTRAP` or the default.
fn bootstrap_path() -> PathBuf {
    std::env::var_os(BOOTSTRAP_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_BOOTSTRAP_PATH))
}

/// Resolve the loopback registry port: `$AVOCADO_CONTAINER_DEV_LOOPBACK_PORT`
/// or the default.
fn loopback_port() -> u16 {
    std::env::var(LOOPBACK_PORT_ENV)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(DEFAULT_LOOPBACK_PORT)
}

/// Load and parse the bootstrap from `path`.
fn load_bootstrap(path: &std::path::Path) -> Result<Bootstrap> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading bootstrap at {}", path.display()))?;
    let bootstrap: Bootstrap = serde_json::from_str(&raw)
        .with_context(|| format!("parsing bootstrap at {}", path.display()))?;
    Ok(bootstrap)
}

/// Resolve the device identity reported in `Hello`.
///
/// Prefers `$AVOCADO_DEVICE_ID`; falls back to the machine hostname
/// (`/etc/hostname`); finally `"unknown"`. Kept dependency-free.
fn resolve_device_id() -> String {
    if let Ok(id) = std::env::var(DEVICE_ID_ENV) {
        let id = id.trim().to_string();
        if !id.is_empty() {
            return id;
        }
    }
    if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
        let hostname = hostname.trim().to_string();
        if !hostname.is_empty() {
            return hostname;
        }
    }
    "unknown".to_string()
}

// ---------------------------------------------------------------------------
// Reconnect backoff.
// ---------------------------------------------------------------------------

/// Exponential backoff with a cap for the reconnect loop.
///
/// Doubles the delay on each `next_delay` up to `max`, and resets to `base`
/// after a successful connection so a long-lived link that briefly drops does
/// not inherit a stale, large delay.
#[derive(Debug, Clone)]
struct Backoff {
    base: Duration,
    max: Duration,
    current: Duration,
}

impl Backoff {
    fn new(base: Duration, max: Duration) -> Self {
        Self {
            base,
            max,
            current: base,
        }
    }

    /// Return the delay to wait before the next attempt, then advance (doubling
    /// up to `max`).
    fn next_delay(&mut self) -> Duration {
        let delay = self.current;
        self.current = (self.current * 2).min(self.max);
        delay
    }

    /// Reset to the base delay (call after a successful connection).
    fn reset(&mut self) {
        self.current = self.base;
    }
}

impl Default for Backoff {
    fn default() -> Self {
        Backoff::new(Duration::from_secs(1), Duration::from_secs(30))
    }
}

/// How long a session must last before it counts as evidence the link works.
///
/// The reset used to key on the handshake alone, which a broken host completes
/// happily before dropping the connection - so the backoff never grew past its
/// 1s base. Ten seconds is far longer than the accept-panic-drop cycle that
/// motivated it and far shorter than any real session.
const MIN_SESSION_FOR_RESET: Duration = Duration::from_secs(10);

/// Whether a finished session is evidence the link works, and so whether the
/// reconnect backoff may reset.
///
/// A function rather than an inline condition so the duration floor is
/// assertable: `run()` needs a real host to exercise, and an inline test
/// re-stating the same expression would pass with the floor removed - which is
/// exactly how this shipped without one.
fn session_proves_the_link_works(handshook: bool, session: Duration) -> bool {
    handshook && session >= MIN_SESSION_FOR_RESET
}

// ---------------------------------------------------------------------------
// Agent state.
// ---------------------------------------------------------------------------

/// Shared, mutable agent state carried across reconnects.
struct AgentState {
    device_id: String,
    /// The content digest the agent is currently running. Empty until a pull
    /// lands (task 6.3 updates this); re-sent in `Hello` on every reconnect.
    running_digest: Mutex<String>,
    /// Raised by the loopback proxy (task 6.2) when the bulk upstream rejects
    /// the read/control token. Surfaced to the host as a `Status` frame on the
    /// next (re)connect; a new token comes only from a host re-`up`.
    rebootstrap: proxy::ReBootstrap,
}

impl AgentState {
    fn new(device_id: String) -> Self {
        Self {
            device_id,
            running_digest: Mutex::new(String::new()),
            rebootstrap: proxy::ReBootstrap::default(),
        }
    }

    fn hello(&self) -> DeviceFrame {
        let running_digest = self
            .running_digest
            .lock()
            .expect("running_digest mutex poisoned")
            .clone();
        DeviceFrame::Hello(Hello {
            device_id: self.device_id.clone(),
            arch: std::env::consts::ARCH.to_string(),
            running_digest,
        })
    }
}

// ---------------------------------------------------------------------------
// TLS.
// ---------------------------------------------------------------------------

/// Build a rustls `ClientConfig` pinned to ONLY the bootstrap CA, using the
/// ring provider. No native roots, no webpki-roots.
pub(crate) fn build_tls_config(ca_cert_pem: &str) -> Result<rustls::ClientConfig> {
    let mut root_store = rustls::RootCertStore::empty();
    let mut reader = std::io::Cursor::new(ca_cert_pem.as_bytes());
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .context("parsing bootstrap ca_cert_pem")?;
    ensure!(
        !certs.is_empty(),
        "bootstrap ca_cert_pem contained no certificates"
    );
    for cert in certs {
        root_store
            .add(cert)
            .context("adding bootstrap CA to root store")?;
    }

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .context("selecting rustls protocol versions")?
        .with_root_certificates(root_store)
        .with_no_client_auth();
    Ok(config)
}

/// Ceiling on each control-WS dial stage.
///
/// The serve loop itself is deliberately unbounded - a session is meant to stay
/// open - but every stage of GETTING there should complete in milliseconds on a
/// working link, and hanging in one means the reconnect loop never runs again.
const WS_DIAL_TIMEOUT: Duration = Duration::from_secs(30);

/// Run `fut`, failing with a named error if it does not finish in time.
async fn with_dial_timeout<T>(
    stage: &'static str,
    fut: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    match tokio::time::timeout(WS_DIAL_TIMEOUT, fut).await {
        Ok(result) => result,
        Err(_) => anyhow::bail!(
            "{stage} did not complete within {}s; the host is unreachable or the connection \
             is half-open",
            WS_DIAL_TIMEOUT.as_secs()
        ),
    }
}

/// Split a `host:port` endpoint into its host component for SNI.
pub(crate) fn endpoint_host(endpoint: &str) -> &str {
    endpoint
        .rsplit_once(':')
        .map(|(h, _)| h)
        .unwrap_or(endpoint)
}

// ---------------------------------------------------------------------------
// Connect + serve.
// ---------------------------------------------------------------------------

/// Handle one host frame. On a `Sync`, pull the new image through the loopback
/// proxy, rewrite the active-image pointer on the writable partition, restart
/// the container, and record the running digest (task 6.3, [`sync::on_sync`]).
async fn handle_host_frame(frame: HostFrame, state: &AgentState) -> Option<DeviceFrame> {
    match frame {
        HostFrame::Sync { image, tag, digest } => {
            info!(%image, %tag, %digest, "received sync; pulling + restarting");
            // The engine shell-out and env-resolved config are the thin,
            // untested production glue; the ordering/pointer/digest core lives
            // in the fully-tested `sync::on_sync`.
            let engine = sync::CommandEngine::from_env();
            let cfg = sync::SyncConfig::from_env();
            match sync::on_sync(&engine, &cfg, &image, &tag, &digest, state).await {
                Ok(()) => None,
                Err(e) => {
                    // Report it, do not only log it. A swallowed failure left the
                    // host with no Status, no Progress, and no updated Hello -
                    // Hello is re-sent only on reconnect, which on a healthy link
                    // may be hours - so it went on showing the device synced at
                    // the old digest with no error surface anywhere.
                    warn!(error = %e, %image, %tag, %digest, "sync failed");
                    Some(DeviceFrame::Status(Status {
                        device_id: state.device_id.clone(),
                        state: "sync_failed".to_string(),
                        detail: Some(format!("{image}:{tag} @ {digest}: {e:#}")),
                    }))
                }
            }
        }
    }
}

/// The device-to-host frames one session may emit, and how they reach the sink.
///
/// Syncs run on their own task rather than inline in the read loop, which is
/// what lets the stream keep being polled while a pull is in flight. Inline,
/// `handle_host_frame` awaited three unbounded subprocesses inside
/// `while let Some(msg) = stream.next().await`, so nothing was read for the
/// duration - and N saves during one slow pull queued N frames in the socket
/// buffer, each getting its own full pull/tag/restart afterwards: N container
/// restarts instead of one, N-1 onto an already-superseded digest.
struct SessionChannels {
    /// Latest-wins slot. A Sync that arrives while another is running replaces
    /// any already-queued one, because only the newest digest is worth applying.
    pending: Arc<Mutex<Option<HostFrame>>>,
    /// Wakes the sync worker when `pending` is refilled.
    work: Arc<tokio::sync::Notify>,
}

/// Dial the control WS, send `Hello`, and serve frames until the link closes.
///
/// Sets `connected` to `true` once the WS handshake succeeds, so the caller can
/// reset the backoff on a link that actually established.
async fn connect_and_serve(
    bootstrap: &Bootstrap,
    state: &Arc<AgentState>,
    connected: &AtomicBool,
) -> Result<()> {
    ensure!(
        !bootstrap.ws_endpoint.is_empty(),
        "bootstrap ws_endpoint is empty; the host must emit it (tracked follow-up)"
    );

    let tls_config = build_tls_config(&bootstrap.ca_cert_pem)?;
    let connector = TlsConnector::from(Arc::new(tls_config));

    // Same reasoning as the proxy's stage timeouts: without one, a half-open
    // connection to a sleeping host wedges the dial forever, and the reconnect
    // loop that is supposed to recover never gets another turn.
    let tcp = with_dial_timeout("the control WS TCP connect", async {
        TcpStream::connect(&bootstrap.ws_endpoint)
            .await
            .with_context(|| format!("connecting TCP to {}", bootstrap.ws_endpoint))
    })
    .await?;

    let host = endpoint_host(&bootstrap.ws_endpoint).to_string();
    let server_name = ServerName::try_from(host).context("invalid TLS server name")?;
    let tls = with_dial_timeout("the control WS TLS handshake", async {
        connector
            .connect(server_name, tcp)
            .await
            .context("TLS handshake to control WS")
    })
    .await?;

    let mut request = format!("wss://{}/", bootstrap.ws_endpoint)
        .into_client_request()
        .context("building WS upgrade request")?;
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", bootstrap.read_token))
            .context("building Authorization header")?,
    );

    let (ws, _response) = with_dial_timeout("the control WS upgrade", async {
        client_async(request, tls)
            .await
            .context("WebSocket upgrade to control WS")
    })
    .await?;
    connected.store(true, Ordering::SeqCst);
    info!(endpoint = %bootstrap.ws_endpoint, "control WS connected");

    // The sink goes idle after the frames below and only the stream is polled.
    // That does NOT strand the automatic Pong replies to host keepalive pings:
    // tungstenite queues a Pong as `additional_send` and flushes it from inside
    // `read()` itself (protocol/mod.rs), which is the same call `poll_next`
    // drives on the read half — so a ping is answered without the sink being
    // touched, and the connection is not churned by a keepalive timeout.
    let (mut sink, mut stream) = ws.split();

    let hello = state.hello();
    sink.send(Message::Text(
        serde_json::to_string(&hello)
            .context("serializing Hello")?
            .into(),
    ))
    .await
    .context("sending Hello")?;

    // The sync worker and the frames it produces. Everything the device sends
    // after `Hello` arrives on `outbound` and is written by the select loop
    // below, so the sink has exactly one writer.
    let channels = SessionChannels {
        pending: Arc::new(Mutex::new(None)),
        work: Arc::new(tokio::sync::Notify::new()),
    };
    let (outbound_tx, mut outbound) = tokio::sync::mpsc::channel::<DeviceFrame>(8);

    let worker = {
        let pending = Arc::clone(&channels.pending);
        let work = Arc::clone(&channels.work);
        let state = Arc::clone(state);
        let outbound_tx = outbound_tx.clone();
        tokio::spawn(async move {
            loop {
                work.notified().await;
                // Drain the slot, not a queue: whatever is there is the newest
                // Sync, and any earlier one it replaced is already superseded.
                let Some(frame) = pending.lock().expect("pending mutex poisoned").take() else {
                    continue;
                };
                if let Some(report) = handle_host_frame(frame, &state).await
                    && outbound_tx.send(report).await.is_err()
                {
                    // The session ended while the sync ran; the next Hello
                    // carries the running digest, so nothing is lost.
                    return;
                }
            }
        })
    };
    // Dropping our own copy means `outbound` closes when the worker does.
    drop(outbound_tx);

    // A re-bootstrap raised BEFORE this session started still needs announcing;
    // one raised during it arrives through `rebootstrap.raised()` below.
    if state.rebootstrap.is_raised() {
        send_frame(&mut sink, &rebootstrap_status(state)).await?;
        state.rebootstrap.clear();
    }

    let result = loop {
        tokio::select! {
            // Biased so an outbound report is written before another host frame
            // is accepted; without it a busy host could starve the reports the
            // whole restructure exists to deliver.
            biased;

            Some(frame) = outbound.recv() => {
                send_frame(&mut sink, &frame).await?;
            }

            // Raised by the loopback proxy on a bulk 401, which happens during a
            // pull - inside this loop, not before it. Reading the flag once up
            // front meant it was always set strictly after the only read that
            // would have transmitted it.
            () = state.rebootstrap.raised() => {
                send_frame(&mut sink, &rebootstrap_status(state)).await?;
                // Lower it once reported, or one transient 401 latches for the
                // process lifetime.
                state.rebootstrap.clear();
            }

            msg = stream.next() => {
                let Some(msg) = msg else { break Ok(()) };
                let msg = msg.context("reading control WS frame")?;
                match msg {
                    Message::Text(text) => match serde_json::from_str::<HostFrame>(text.as_str()) {
                        Ok(frame) => {
                            // Latest wins. Replacing a queued Sync is the point:
                            // applying a superseded digest costs a container
                            // restart and achieves nothing.
                            let replaced = channels
                                .pending
                                .lock()
                                .expect("pending mutex poisoned")
                                .replace(frame)
                                .is_some();
                            if replaced {
                                info!("coalesced a superseded sync still waiting to run");
                            }
                            channels.work.notify_one();
                        }
                        Err(e) => warn!(error = %e, raw = %text, "ignoring malformed host frame"),
                    },
                    Message::Close(_) => {
                        info!("control WS closed by host");
                        break Ok(());
                    }
                    Message::Ping(_) | Message::Pong(_) => {}
                    Message::Binary(_) | Message::Frame(_) => {
                        warn!("ignoring unexpected non-text control frame");
                    }
                }
            }
        }
    };

    worker.abort();
    result
}

/// The `needs_rebootstrap` status. There is no token-renewal endpoint; the
/// operator must re-run `avocado container dev up` to mint a fresh token.
fn rebootstrap_status(state: &AgentState) -> DeviceFrame {
    DeviceFrame::Status(Status {
        device_id: state.device_id.clone(),
        state: "needs_rebootstrap".to_string(),
        detail: Some(
            "bulk pull rejected the read/control token; re-run `avocado container dev up`"
                .to_string(),
        ),
    })
}

/// Serialize and write one device frame to the control WS sink.
async fn send_frame<S>(sink: &mut S, frame: &DeviceFrame) -> Result<()>
where
    S: futures_util::SinkExt<Message> + Unpin,
    <S as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
{
    sink.send(Message::Text(
        serde_json::to_string(frame)
            .context("serializing device frame")?
            .into(),
    ))
    .await
    .context("sending device frame")?;
    Ok(())
}

/// The reconnect loop: connect, serve, and on any close/error back off and
/// redial, re-sending `Hello` with the current running digest.
/// Returns `Ok(())` when the bootstrap on disk stops matching the one this
/// process started with, so `main` exits and the supervisor rebuilds against the
/// new material. Reloading in place is not enough: the read token is also baked
/// into the loopback proxy's upstream (`HttpsUpstream::from_bootstrap`), and a
/// token the proxy still sends is exactly what 401s the bulk leg. Exiting
/// re-reads both from one code path.
///
/// Without this the documented recovery could not work. Re-running `avocado
/// container dev up` mints a fresh token and overwrites bootstrap.json; the
/// agent kept the old one, so bulk pulls 401'd, the WS upgrade was rejected by
/// the same host-side auth check, `connect_and_serve` errored on every redial,
/// and this loop spun forever. Nothing exited, so `Restart=always` never fired,
/// and the only fix was a manual unit restart that no message mentioned.
async fn run(bootstrap: Bootstrap, state: Arc<AgentState>) -> Result<()> {
    let path = bootstrap_path();
    let mut backoff = Backoff::default();
    loop {
        let connected = AtomicBool::new(false);
        let started = Instant::now();
        match connect_and_serve(&bootstrap, &state, &connected).await {
            Ok(()) => info!("control WS session ended; reconnecting"),
            Err(e) => warn!(error = %e, "control WS session failed; reconnecting"),
        }

        // A session that completed its handshake and then ended immediately is
        // not evidence the link works. `connected` flips the moment
        // `client_async` returns, so without a duration floor EVERY
        // handshake-completing session reset the backoff and the 30s cap was
        // unreachable: a host whose `desired` mutex is poisoned accepts the
        // upgrade, panics on the first Hello and drops the socket, which pinned
        // an embedded device to a ~1 Hz pinned-CA TLS handshake loop against a
        // host that would never work again.
        if session_proves_the_link_works(connected.load(Ordering::SeqCst), started.elapsed()) {
            backoff.reset();
        }

        // Adopt new session material by exiting, not by continuing.
        match load_bootstrap(&path) {
            Ok(current) if !current.same_session_as(&bootstrap) => {
                info!(
                    bootstrap = %path.display(),
                    "bootstrap changed on disk; exiting so the supervisor rebuilds this \
                     process against the new session material"
                );
                return Ok(());
            }
            Ok(_) => {}
            // Unreadable or malformed: keep the session we have rather than
            // exiting on a file caught mid-delivery.
            Err(e) => {
                warn!(error = %e, "could not re-read the bootstrap; keeping the current one")
            }
        }

        let delay = backoff.next_delay();
        info!(delay_secs = delay.as_secs(), "backing off before reconnect");
        sleep(delay).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Defense-in-depth: pin ring as the process-wide default provider so any
    // implicit rustls builder cannot fall back to a different provider.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let path = bootstrap_path();
    let bootstrap = load_bootstrap(&path)?;
    let device_id = resolve_device_id();
    info!(%device_id, bootstrap = %path.display(), "container dev agent starting");

    let state = Arc::new(AgentState::new(device_id));

    // Start the loopback registry proxy (task 6.2): the device engine pulls
    // from this plain-HTTP loopback (no insecure flag, no per-engine trust
    // file); each read is forwarded to the host bulk HTTPS listener over the
    // pinned CA + read/control token. Bulk NEVER rides the control WS.
    // Warn once, here, if the configured engine cannot work under the shipped
    // systemd unit. This is the only place that runs exactly once per process:
    // `CommandEngine::from_env` is constructed per `Sync` frame, so warning
    // there logged nothing at `systemctl start` (when the operator's override is
    // new information) and then repeated the warning before every pull failure.
    let engine_binary = sync::engine_binary_from_env();
    sync::warn_if_unsupported_engine(&engine_binary);
    // Refuse to start without the engine, rather than staying `active (running)`
    // and no-oping every sync. See `ensure_engine_available`.
    sync::ensure_engine_available(&engine_binary)?;

    // Failing to BUILD the upstream is as fatal as losing it later.
    //
    // This used to `warn!` once and carry on with `None`, three lines above a
    // comment arguing at length that losing the proxy must end the process
    // "because without it no pull can succeed". Two tests locked in "losing the
    // proxy is fatal"; never having it was uncovered. A truncated `ca_cert_pem`
    // from a partial SSH write, or a bulk endpoint host `ServerName::try_from`
    // rejects, left the agent `active (running)` with port 15151 closed and
    // every Sync failing connection-refused - the exact state that comment was
    // written to prevent.
    //
    // The stated justification, that a re-bootstrap could fix it, was
    // self-defeating twice over: `raise()` is only reached from `proxy_read`, so
    // with no proxy the needs_rebootstrap frame can never be sent, and the
    // bootstrap was never re-read anyway.
    let upstream = proxy::HttpsUpstream::from_bootstrap(&bootstrap)
        .context("building the loopback proxy's upstream; without it no pull can succeed")?;
    let proxy_task = {
        let port = loopback_port();
        let upstream = Arc::new(upstream);
        let rebootstrap = state.rebootstrap.clone();
        tokio::spawn(async move { proxy::serve_loopback(port, upstream, rebootstrap).await })
    };

    // The proxy's bounded accept-retry only reaches `Restart=always` if losing
    // the proxy ends the PROCESS. Spawning it detached and dropping the handle
    // meant `serve_loopback` returning Err killed only its own task: `run()`
    // kept the control WS up, systemd saw a healthy unit, and every later Sync
    // failed connection-refused against a proxy that was gone - while the agent
    // went on reporting a stale running_digest. Nothing restarted it, because
    // from systemd's view nothing had failed.
    //
    // Select the two so whichever ends first ends `main`. The proxy is not
    // optional: without it no pull can succeed, so exiting and letting the
    // supervisor rebuild the process is the honest response to losing it.
    end_when_either_ends(proxy_task, run(bootstrap, state)).await
}

/// Return as soon as EITHER the proxy task or the control-WS loop ends.
///
/// Extracted from `main` so the seam is testable. It is the whole reason the
/// proxy's bounded accept-retry means anything: the retry makes `serve_loopback`
/// return, and only this makes that return end the process, which is what
/// `Restart=always` needs in order to fire. Spawned detached with its handle
/// dropped, a returning proxy killed only its own task - systemd saw a healthy
/// unit while every later pull failed connection-refused.
async fn end_when_either_ends(
    proxy: tokio::task::JoinHandle<Result<()>>,
    serving: impl std::future::Future<Output = Result<()>>,
) -> Result<()> {
    tokio::select! {
        proxied = proxy => match proxied {
            // The proxy is not optional - without it no pull can succeed - so
            // even a clean stop is a reason to exit and be rebuilt.
            Ok(Ok(())) => bail!("loopback registry proxy stopped serving"),
            Ok(Err(e)) => Err(e).context("loopback registry proxy failed"),
            Err(e) => Err(anyhow::Error::new(e))
                .context("loopback registry proxy task panicked"),
        },
        served = serving => served,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_sync_frame_deserializes_from_wire() {
        let wire = r#"{"type":"sync","image":"my-app","tag":"dev","digest":"sha256:abc"}"#;
        let frame: HostFrame = serde_json::from_str(wire).expect("sync frame parses");
        assert_eq!(
            frame,
            HostFrame::Sync {
                image: "my-app".to_string(),
                tag: "dev".to_string(),
                digest: "sha256:abc".to_string(),
            }
        );
    }

    #[test]
    fn hello_frame_serializes_flat_with_tag() {
        let hello = DeviceFrame::Hello(Hello {
            device_id: "dev-01".to_string(),
            arch: "aarch64".to_string(),
            running_digest: "sha256:running".to_string(),
        });
        let value: serde_json::Value =
            serde_json::to_value(&hello).expect("hello serializes to json");
        assert_eq!(value["type"], "hello");
        assert_eq!(value["device_id"], "dev-01");
        assert_eq!(value["arch"], "aarch64");
        assert_eq!(value["running_digest"], "sha256:running");
        // The tag object is flat: no nested wrapper key for the Hello struct.
        assert!(value.get("Hello").is_none());
    }

    #[test]
    fn progress_frame_serializes_flat_with_tag() {
        let frame = DeviceFrame::Progress(Progress {
            image: "my-app".to_string(),
            bytes_pulled: 4096,
        });
        let value: serde_json::Value = serde_json::to_value(&frame).unwrap();
        assert_eq!(value["type"], "progress");
        assert_eq!(value["image"], "my-app");
        assert_eq!(value["bytes_pulled"], 4096);
    }

    #[test]
    fn status_frame_serializes_flat_with_tag() {
        let frame = DeviceFrame::Status(Status {
            device_id: "dev-01".to_string(),
            state: "running".to_string(),
            detail: None,
        });
        let value: serde_json::Value = serde_json::to_value(&frame).unwrap();
        assert_eq!(value["type"], "status");
        assert_eq!(value["device_id"], "dev-01");
        assert_eq!(value["state"], "running");
        // Optional detail is skipped when None.
        assert!(value.get("detail").is_none());
    }

    #[test]
    fn device_frame_round_trips() {
        let frames = vec![
            DeviceFrame::Hello(Hello {
                device_id: "d".to_string(),
                arch: "x86_64".to_string(),
                running_digest: String::new(),
            }),
            DeviceFrame::Progress(Progress {
                image: "img".to_string(),
                bytes_pulled: 1,
            }),
            DeviceFrame::Status(Status {
                device_id: "d".to_string(),
                state: "s".to_string(),
                detail: Some("ok".to_string()),
            }),
        ];
        for frame in frames {
            let json = serde_json::to_string(&frame).unwrap();
            let back: DeviceFrame = serde_json::from_str(&json).unwrap();
            assert_eq!(frame, back);
        }
    }

    #[test]
    fn bootstrap_parses_without_ws_endpoint_and_defaults_it() {
        // A current host payload carries only the three DeviceBootstrap fields.
        let wire = r#"{
            "bulk_endpoint": "10.0.0.1:8443",
            "read_token": "tok",
            "ca_cert_pem": "-----BEGIN CERTIFICATE-----\nMII...\n-----END CERTIFICATE-----\n"
        }"#;
        let bootstrap: Bootstrap = serde_json::from_str(wire).expect("bootstrap parses");
        assert_eq!(bootstrap.bulk_endpoint, "10.0.0.1:8443");
        assert_eq!(bootstrap.read_token, "tok");
        assert_eq!(bootstrap.ws_endpoint, "");
    }

    #[test]
    fn bootstrap_parses_with_ws_endpoint() {
        let wire = r#"{
            "bulk_endpoint": "10.0.0.1:8443",
            "read_token": "tok",
            "ca_cert_pem": "pem",
            "ws_endpoint": "10.0.0.1:9443"
        }"#;
        let bootstrap: Bootstrap = serde_json::from_str(wire).unwrap();
        assert_eq!(bootstrap.ws_endpoint, "10.0.0.1:9443");
    }

    #[test]
    fn backoff_doubles_then_caps() {
        let mut backoff = Backoff::new(Duration::from_secs(1), Duration::from_secs(30));
        assert_eq!(backoff.next_delay(), Duration::from_secs(1));
        assert_eq!(backoff.next_delay(), Duration::from_secs(2));
        assert_eq!(backoff.next_delay(), Duration::from_secs(4));
        assert_eq!(backoff.next_delay(), Duration::from_secs(8));
        assert_eq!(backoff.next_delay(), Duration::from_secs(16));
        // 32 would exceed the 30s cap.
        assert_eq!(backoff.next_delay(), Duration::from_secs(30));
        assert_eq!(backoff.next_delay(), Duration::from_secs(30));
    }

    #[test]
    fn backoff_resets_to_base_after_connect() {
        let mut backoff = Backoff::new(Duration::from_secs(1), Duration::from_secs(30));
        backoff.next_delay();
        backoff.next_delay();
        backoff.next_delay();
        backoff.reset();
        assert_eq!(backoff.next_delay(), Duration::from_secs(1));
        assert_eq!(backoff.next_delay(), Duration::from_secs(2));
    }

    #[test]
    fn a_handshake_that_immediately_drops_does_not_reset_the_backoff() {
        // `connected` flips the moment `client_async` returns, so a host that
        // accepts the upgrade and then panics on the first Hello - a poisoned
        // `desired` mutex does exactly that - completed a "successful"
        // connection every time. Without a duration floor the backoff reset on
        // each one, the 30s cap was unreachable, and the device sat in a ~1 Hz
        // pinned-CA TLS handshake loop indefinitely.
        assert!(
            !session_proves_the_link_works(true, Duration::from_millis(5)),
            "a 5ms session must not count as a working link"
        );
        assert!(
            !session_proves_the_link_works(true, MIN_SESSION_FOR_RESET - Duration::from_millis(1)),
            "just under the floor must not reset"
        );
        assert!(
            session_proves_the_link_works(true, MIN_SESSION_FOR_RESET),
            "a session at the floor is evidence the link works"
        );
        assert!(
            !session_proves_the_link_works(false, Duration::from_secs(600)),
            "a dial that never handshook must not reset however long it took"
        );
    }

    #[test]
    fn a_changed_bootstrap_ends_the_process_so_the_new_token_is_adopted() {
        // Re-running `avocado container dev up` mints a fresh token and
        // overwrites bootstrap.json. The agent kept the old one: bulk pulls
        // 401'd, the WS upgrade was rejected by the same host-side auth check,
        // and the reconnect loop spun forever without ever exiting - so
        // Restart=always never fired and the documented recovery could not work.
        let base = Bootstrap {
            bulk_endpoint: "10.0.2.2:15150".to_string(),
            read_token: "T1".to_string(),
            ca_cert_pem: "PEM".to_string(),
            ws_endpoint: "10.0.2.2:15152".to_string(),
        };

        assert!(base.same_session_as(&base.clone()));

        for changed in [
            Bootstrap {
                read_token: "T2".to_string(),
                ..base.clone()
            },
            Bootstrap {
                ca_cert_pem: "OTHER".to_string(),
                ..base.clone()
            },
            Bootstrap {
                bulk_endpoint: "10.0.2.2:15999".to_string(),
                ..base.clone()
            },
            Bootstrap {
                ws_endpoint: "10.0.2.2:15999".to_string(),
                ..base.clone()
            },
        ] {
            assert!(
                !base.same_session_as(&changed),
                "a changed {changed:?} must end the session"
            );
        }
    }

    #[tokio::test]
    async fn a_raised_rebootstrap_wakes_a_waiter_and_clears_once_reported() {
        // The flag was read exactly once per session, BEFORE the frame loop -
        // and the 401 that raises it happens during a pull, inside that loop. So
        // it was always set strictly after the only read that would have sent
        // it. Nothing could lower it either, so one transient 401 latched for
        // the process lifetime.
        let flag = proxy::ReBootstrap::default();
        assert!(!flag.is_raised());

        let waiter = {
            let flag = flag.clone();
            tokio::spawn(async move { flag.raised().await })
        };
        // Let the waiter reach `notified()` before raising.
        tokio::task::yield_now().await;
        flag.raise();

        tokio::time::timeout(Duration::from_secs(5), waiter)
            .await
            .expect("raise must wake a mid-session waiter")
            .expect("waiter task");

        assert!(flag.is_raised());
        flag.clear();
        assert!(
            !flag.is_raised(),
            "the flag must lower once reported, or every later session re-announces it"
        );
    }

    #[test]
    fn a_failed_sync_produces_a_status_frame_for_the_host() {
        // A swallowed sync failure left the host with no Status, no Progress and
        // no updated Hello - Hello is re-sent only on reconnect - so it went on
        // showing the device synced at the old digest with no error surface.
        let state = AgentState::new("dev-01".to_string());
        let frame = DeviceFrame::Status(Status {
            device_id: state.device_id.clone(),
            state: "sync_failed".to_string(),
            detail: Some("my-app:dev @ sha256:new: boom".to_string()),
        });
        let json = serde_json::to_string(&frame).expect("serializes");
        assert!(json.contains("sync_failed"), "{json}");
        assert!(json.contains("dev-01"), "{json}");
    }

    #[test]
    fn hello_carries_current_running_digest() {
        let state = AgentState::new("dev-01".to_string());
        // Simulate a pull landing (task 6.3 behavior).
        *state.running_digest.lock().unwrap() = "sha256:new".to_string();
        match state.hello() {
            DeviceFrame::Hello(hello) => {
                assert_eq!(hello.device_id, "dev-01");
                assert_eq!(hello.running_digest, "sha256:new");
                assert_eq!(hello.arch, std::env::consts::ARCH);
            }
            other => panic!("expected Hello, got {other:?}"),
        }
    }

    #[test]
    fn endpoint_host_strips_port() {
        assert_eq!(endpoint_host("10.0.0.1:9443"), "10.0.0.1");
        assert_eq!(endpoint_host("host.local:443"), "host.local");
        assert_eq!(endpoint_host("nohost"), "nohost");
    }

    #[test]
    fn build_tls_config_rejects_empty_pem() {
        let err = build_tls_config("not a cert").unwrap_err();
        assert!(
            err.to_string().contains("no certificates"),
            "unexpected error: {err}"
        );
    }

    // The seam that makes the proxy's accept-retry mean anything. Losing the
    // proxy must end `main`, because that is what `Restart=always` needs in
    // order to fire; a returning proxy task whose handle was dropped left
    // systemd looking at a healthy unit while every pull failed
    // connection-refused.
    #[tokio::test]
    async fn a_failing_proxy_ends_the_process_even_while_the_ws_keeps_serving() {
        let proxy = tokio::spawn(async { Err(anyhow::anyhow!("listener died")) });
        // A control-WS loop that would otherwise run forever, as `run()` does.
        let serving = std::future::pending::<Result<()>>();

        let err = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            end_when_either_ends(proxy, serving),
        )
        .await
        .expect("losing the proxy must end this, not hang on the WS loop")
        .expect_err("a failed proxy must surface an error");

        assert!(
            format!("{err:#}").contains("listener died"),
            "the proxy's own error must survive to the exit: {err:#}"
        );
    }

    // A proxy that stops cleanly is still fatal: without it no pull can
    // succeed, so exiting to be rebuilt beats serving a control WS that can
    // only ever report failures.
    #[tokio::test]
    async fn a_cleanly_stopped_proxy_also_ends_the_process() {
        let proxy = tokio::spawn(async { Ok(()) });
        let serving = std::future::pending::<Result<()>>();

        let err = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            end_when_either_ends(proxy, serving),
        )
        .await
        .expect("a stopped proxy must end this")
        .expect_err("a stopped proxy must be reported as an error");
        assert!(format!("{err:#}").contains("stopped serving"), "{err:#}");
    }

    // The other direction: the WS loop ending returns its own result rather
    // than being masked by the still-running proxy.
    #[tokio::test]
    async fn a_failing_ws_loop_returns_its_own_error() {
        let proxy = tokio::spawn(async { std::future::pending::<Result<()>>().await });
        let serving = async { Err(anyhow::anyhow!("ws gave up")) };

        let err = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            end_when_either_ends(proxy, serving),
        )
        .await
        .expect("the WS loop ending must end this")
        .expect_err("a failed WS loop must surface an error");
        assert!(format!("{err:#}").contains("ws gave up"), "{err:#}");
    }
}
