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

use anyhow::{Context, Result, ensure};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::time::sleep;
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
async fn handle_host_frame(frame: HostFrame, state: &AgentState) {
    match frame {
        HostFrame::Sync { image, tag, digest } => {
            info!(%image, %tag, %digest, "received sync; pulling + restarting");
            // The engine shell-out and env-resolved config are the thin,
            // untested production glue; the ordering/pointer/digest core lives
            // in the fully-tested `sync::on_sync`.
            let engine = sync::CommandEngine::from_env();
            let cfg = sync::SyncConfig::from_env();
            if let Err(e) = sync::on_sync(&engine, &cfg, &image, &tag, &digest, state).await {
                warn!(error = %e, %image, %tag, %digest, "sync failed");
            }
        }
    }
}

/// Dial the control WS, send `Hello`, and serve frames until the link closes.
///
/// Sets `connected` to `true` once the WS handshake succeeds, so the caller can
/// reset the backoff on a link that actually established.
async fn connect_and_serve(
    bootstrap: &Bootstrap,
    state: &AgentState,
    connected: &AtomicBool,
) -> Result<()> {
    ensure!(
        !bootstrap.ws_endpoint.is_empty(),
        "bootstrap ws_endpoint is empty; the host must emit it (tracked follow-up)"
    );

    let tls_config = build_tls_config(&bootstrap.ca_cert_pem)?;
    let connector = TlsConnector::from(Arc::new(tls_config));

    let tcp = TcpStream::connect(&bootstrap.ws_endpoint)
        .await
        .with_context(|| format!("connecting TCP to {}", bootstrap.ws_endpoint))?;

    let host = endpoint_host(&bootstrap.ws_endpoint).to_string();
    let server_name = ServerName::try_from(host).context("invalid TLS server name")?;
    let tls = connector
        .connect(server_name, tcp)
        .await
        .context("TLS handshake to control WS")?;

    let mut request = format!("wss://{}/", bootstrap.ws_endpoint)
        .into_client_request()
        .context("building WS upgrade request")?;
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", bootstrap.read_token))
            .context("building Authorization header")?,
    );

    let (ws, _response) = client_async(request, tls)
        .await
        .context("WebSocket upgrade to control WS")?;
    connected.store(true, Ordering::SeqCst);
    info!(endpoint = %bootstrap.ws_endpoint, "control WS connected");

    let (mut sink, mut stream) = ws.split();

    let hello = state.hello();
    sink.send(Message::Text(
        serde_json::to_string(&hello)
            .context("serializing Hello")?
            .into(),
    ))
    .await
    .context("sending Hello")?;

    // Surface a pending re-bootstrap (raised by the loopback proxy on a bulk
    // 401) to the host. There is no token-renewal endpoint; the operator must
    // re-run `avocado container dev up` to mint a fresh token.
    if state.rebootstrap.is_raised() {
        let status = DeviceFrame::Status(Status {
            device_id: state.device_id.clone(),
            state: "needs_rebootstrap".to_string(),
            detail: Some(
                "bulk pull rejected the read/control token; re-run `avocado container dev up`"
                    .to_string(),
            ),
        });
        sink.send(Message::Text(
            serde_json::to_string(&status)
                .context("serializing re-bootstrap status")?
                .into(),
        ))
        .await
        .context("sending re-bootstrap status")?;
    }

    while let Some(msg) = stream.next().await {
        let msg = msg.context("reading control WS frame")?;
        match msg {
            Message::Text(text) => match serde_json::from_str::<HostFrame>(text.as_str()) {
                Ok(frame) => handle_host_frame(frame, state).await,
                Err(e) => warn!(error = %e, raw = %text, "ignoring malformed host frame"),
            },
            Message::Close(_) => {
                info!("control WS closed by host");
                break;
            }
            Message::Ping(_) | Message::Pong(_) => {}
            Message::Binary(_) | Message::Frame(_) => {
                warn!("ignoring unexpected non-text control frame");
            }
        }
    }

    Ok(())
}

/// The reconnect loop: connect, serve, and on any close/error back off and
/// redial, re-sending `Hello` with the current running digest.
async fn run(bootstrap: Bootstrap, state: Arc<AgentState>) -> Result<()> {
    let mut backoff = Backoff::default();
    loop {
        let connected = AtomicBool::new(false);
        match connect_and_serve(&bootstrap, &state, &connected).await {
            Ok(()) => info!("control WS session ended; reconnecting"),
            Err(e) => warn!(error = %e, "control WS session failed; reconnecting"),
        }
        if connected.load(Ordering::SeqCst) {
            backoff.reset();
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
    match proxy::HttpsUpstream::from_bootstrap(&bootstrap) {
        Ok(upstream) => {
            let port = loopback_port();
            let upstream = Arc::new(upstream);
            let rebootstrap = state.rebootstrap.clone();
            tokio::spawn(async move {
                if let Err(e) = proxy::serve_loopback(port, upstream, rebootstrap).await {
                    warn!(error = %e, "loopback registry proxy exited");
                }
            });
        }
        Err(e) => warn!(error = %e, "loopback registry proxy not started"),
    }

    run(bootstrap, state).await
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
}
