//! Device-side sync handler for Container Dev Mode (design D9; task 6.3).
//!
//! On a [`crate::HostFrame::Sync`] the agent, IN ORDER:
//!   1. **Pulls** the new image through the device loopback proxy
//!      (`127.0.0.1:<port>`, task 6.2). The pull rides the proxy -> host bulk
//!      HTTPS path; it NEVER re-pulls over the control WebSocket (D9 splits
//!      bulk from control).
//!   2. **Restarts the container** via the engine AFTER the pull, so the new
//!      image takes effect. The read-only rootFS is untouched throughout.
//!   3. **Records the now-running digest** in [`crate::AgentState::running_digest`]
//!      so task 6.1's reconnect `Hello` reports it. This lands BEFORE the pointer
//!      write so a full or unwritable `/var` cannot stop the agent reporting what
//!      it actually runs.
//!   4. **Rewrites the active-image pointer** — a small JSON file recording the
//!      now-active `{image, tag, digest}` — on the **writable partition**
//!      (`/var/lib/avocado/container-dev/active-image.json`), only once the
//!      restart succeeded, so a failed restart never leaves the pointer ahead of
//!      what the device actually runs. The path is derived from
//!      [`WRITABLE_ROOT`], never a read-only rootFS path (`/usr`, `/etc`, the
//!      image rootfs). This is the one step allowed to fail without failing the
//!      sync, so no step may assume the pointer file exists - it can be absent
//!      while the digest at step 3 is already set.
//!
//! The engine interaction is behind the [`Engine`] trait (mirroring task 6.2's
//! [`crate::proxy::Upstream`] pattern) so the ordering / pointer / digest logic
//! is unit-tested with a fake. The production [`CommandEngine`] shells out to
//! docker/podman via `tokio::process::Command` and is the only untested part —
//! no new TLS/HTTP deps are pulled in for the shell-out, keeping `aws-lc-rs`
//! out of the tree (A9).

use std::future::Future;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{info, warn};

use crate::AgentState;

/// The writable-partition root for on-device mutable state. Consistent with
/// `main.rs`'s bootstrap root (`DEFAULT_BOOTSTRAP_PATH` lives under this same
/// directory). NEVER a read-only rootFS path.
pub(crate) const WRITABLE_ROOT: &str = "/var/lib/avocado/container-dev";
/// The active-image pointer file name, written under [`WRITABLE_ROOT`].
const ACTIVE_IMAGE_FILE: &str = "active-image.json";
/// Default container engine binary when unset (docker; podman via the env var).
const DEFAULT_ENGINE: &str = "docker";
/// Default container name to restart when unset.
const DEFAULT_CONTAINER: &str = "avocado-dev";
/// Env var selecting the container engine binary (`docker` or `podman`).
const ENGINE_ENV: &str = "AVOCADO_CONTAINER_DEV_ENGINE";
/// Env var selecting the container name to restart on sync.
const CONTAINER_ENV: &str = "AVOCADO_CONTAINER_DEV_CONTAINER";
/// Env var overriding the writable state root (used by tests and staging).
const STATE_ROOT_ENV: &str = "AVOCADO_CONTAINER_DEV_STATE_ROOT";
/// Env var naming the systemd unit that OWNS the container, restarted instead of
/// the container itself when set.
///
/// `<engine> restart <container>` re-executes the container's existing config,
/// which pins the image by ID at create time - so a freshly pulled image for the
/// same tag is ignored and the sync silently no-ops while reporting success. The
/// unit that launched the container re-runs `<engine> run` on restart, which
/// re-resolves the tag and therefore adopts the new image. This mirrors the
/// `service:` field each entry under `container_dev.images` already declares.
const SERVICE_ENV: &str = "AVOCADO_CONTAINER_DEV_SERVICE";

// ---------------------------------------------------------------------------
// Active-image pointer.
// ---------------------------------------------------------------------------

/// The active-image pointer: which `{image, tag, digest}` the device now runs.
/// Serialized to `active-image.json` on the writable partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActiveImage {
    pub image: String,
    pub tag: String,
    pub digest: String,
}

/// The active-image pointer path, derived from `writable_root`. The join makes
/// it structurally impossible to land on a rootFS constant: the caller supplies
/// the root, and production always supplies [`WRITABLE_ROOT`].
fn active_image_path(writable_root: &Path) -> PathBuf {
    writable_root.join(ACTIVE_IMAGE_FILE)
}

/// Write the active-image pointer under `writable_root`, creating the directory
/// if needed. Returns the path written.
fn write_active_image(writable_root: &Path, active: &ActiveImage) -> Result<PathBuf> {
    let path = active_image_path(writable_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating writable state dir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(active).context("serializing active-image pointer")?;
    std::fs::write(&path, json)
        .with_context(|| format!("writing active-image pointer {}", path.display()))?;
    Ok(path)
}

/// The digest-pinned loopback registry reference the engine pulls. The device
/// engine auto-trusts `127.0.0.1`, so this rides the plain-HTTP loopback proxy
/// (task 6.2) which forwards to the host bulk HTTPS listener.
fn loopback_reference(port: u16, image: &str, digest: &str) -> String {
    format!("127.0.0.1:{port}/{image}@{digest}")
}

// ---------------------------------------------------------------------------
// Engine abstraction (injectable for tests).
// ---------------------------------------------------------------------------

/// The container engine: pull an image reference, restart a container. Async so
/// the production impl can shell out without blocking the runtime; abstracted so
/// the sync ordering / pointer / digest logic is unit-testable with a fake.
pub(crate) trait Engine: Send + Sync + 'static {
    fn pull(&self, reference: &str) -> impl Future<Output = Result<()>> + Send;
    /// Point `target` (the ref the service names) at `source` (what we just pulled).
    fn tag(&self, source: &str, target: &str) -> impl Future<Output = Result<()>> + Send;
    fn restart(&self, container: &str) -> impl Future<Output = Result<()>> + Send;
}

/// The real engine: shells out to the docker/podman CLI. This is the only
/// untested part of task 6.3 (it execs a subprocess); the pure [`on_sync`] core
/// above it is fully covered.
pub(crate) struct CommandEngine {
    binary: String,
}

impl CommandEngine {
    /// Resolve the engine binary from `$AVOCADO_CONTAINER_DEV_ENGINE`, defaulting
    /// to `docker`.
    pub(crate) fn from_env() -> Self {
        let binary = std::env::var(ENGINE_ENV)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_ENGINE.to_string());
        Self { binary }
    }
}

/// The engine binary the agent would use, resolved from the environment.
///
/// Split out so `main` can inspect it once at startup without constructing an
/// engine. [`CommandEngine::from_env`] runs per `Sync` frame, which is the wrong
/// place to warn from: nothing is logged at `systemctl start`, and then the same
/// warning repeats before every pull failure.
pub(crate) fn engine_binary_from_env() -> String {
    std::env::var(ENGINE_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_ENGINE.to_string())
}

/// Whether `binary` names an engine the shipped systemd unit cannot run.
///
/// Rootful podman needs CAP_SYS_ADMIN, namespace creation, cgroup writes and a
/// writable `/var/lib/containers`; the unit's sandbox denies all of them. Pure so
/// the classification is testable without an environment or a process.
pub(crate) fn is_unsupported_engine(binary: &str) -> bool {
    binary.contains("podman")
}

/// Warn once when the configured engine cannot work under the shipped unit.
///
/// A warning rather than a refusal: the restriction belongs to the unit, not the
/// binary, so an agent run directly for debugging is unaffected. Without it the
/// failure is invisible in the worst way - podman fails at store init, `on_sync`'s
/// error is only logged by its caller, and every Sync silently no-ops while the
/// agent stays connected reporting a stale digest.
pub(crate) fn warn_if_unsupported_engine(binary: &str) {
    if is_unsupported_engine(binary) {
        warn!(
            engine = %binary,
            "podman is not supported under the container-agent-dev systemd unit \
             (its sandbox denies the privileges rootful podman needs); expect pulls \
             to fail unless this agent is running outside that unit"
        );
    }
}

/// The systemd unit owning the container, from `$AVOCADO_CONTAINER_DEV_SERVICE`.
///
/// `None` keeps the container-restart path, so a device that does not launch its
/// container from a unit behaves exactly as before.
pub(crate) fn service_from_env() -> Option<String> {
    std::env::var(SERVICE_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl Engine for CommandEngine {
    async fn pull(&self, reference: &str) -> Result<()> {
        run_engine(&self.binary, &["pull", reference]).await
    }

    async fn tag(&self, source: &str, target: &str) -> Result<()> {
        run_engine(&self.binary, &["tag", source, target]).await
    }

    /// Restart the owning systemd unit when one is configured, else the container.
    ///
    /// See [`SERVICE_ENV`]: restarting the container re-executes its pinned image
    /// ID, so the pulled image would never actually run.
    async fn restart(&self, container: &str) -> Result<()> {
        match service_from_env() {
            Some(service) => {
                info!(service = %service, "restarting owning systemd unit to adopt the pulled image");
                run_engine("systemctl", &["restart", &service]).await
            }
            None => run_engine(&self.binary, &["restart", container]).await,
        }
    }
}

/// Run `<binary> <args...>` to completion, mapping a non-zero exit to an error.
async fn run_engine(binary: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(binary)
        .args(args)
        .status()
        .await
        .with_context(|| format!("spawning `{binary} {}`", args.join(" ")))?;
    ensure!(
        status.success(),
        "`{binary} {}` exited with {status}",
        args.join(" ")
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Sync config + core.
// ---------------------------------------------------------------------------

/// The device-side sync configuration: the loopback port the engine pulls from,
/// the container to restart, and the writable state root the pointer lands in.
pub(crate) struct SyncConfig {
    pub loopback_port: u16,
    pub container: String,
    pub writable_root: PathBuf,
}

impl SyncConfig {
    /// Resolve from env: loopback port shares `main.rs`'s resolver; the writable
    /// root defaults to [`WRITABLE_ROOT`] (`$AVOCADO_CONTAINER_DEV_STATE_ROOT`
    /// overrides for tests/staging); the container defaults to `avocado-dev`
    /// (`$AVOCADO_CONTAINER_DEV_CONTAINER` overrides).
    pub(crate) fn from_env() -> Self {
        let writable_root = std::env::var_os(STATE_ROOT_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(WRITABLE_ROOT));
        let container = std::env::var(CONTAINER_ENV)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_CONTAINER.to_string());
        Self {
            loopback_port: crate::loopback_port(),
            container,
            writable_root,
        }
    }
}

/// Handle a `Sync`: pull -> restart -> record digest -> rewrite pointer.
///
/// The order is load-bearing at both ends. A pull failure aborts before the
/// restart, the digest update and the pointer write, so a failed sync never
/// claims a new image is running and never mutates state. The digest is recorded
/// before the pointer write, so an unwritable `/var` costs the pointer file but
/// not a correct `Hello` - which makes the pointer write the only step that may
/// fail while the sync still returns `Ok`. All writes target the writable
/// partition; the read-only rootFS is never touched.
pub(crate) async fn on_sync<E: Engine>(
    engine: &E,
    cfg: &SyncConfig,
    image: &str,
    tag: &str,
    digest: &str,
    state: &AgentState,
) -> Result<()> {
    // 1. Pull through the device loopback proxy (never over the control WS).
    let reference = loopback_reference(cfg.loopback_port, image, digest);
    engine
        .pull(&reference)
        .await
        .with_context(|| format!("pulling {reference} through loopback proxy"))?;

    // 2. Point the ref the service names at what we just pulled.
    //
    //    The pull is digest-pinned, so it lands as an untagged image. The unit's
    //    ExecStart names `<image>:<tag>`, which on this device either does not
    //    exist (the host built it, so the target never had that tag) or still
    //    resolves to the PREVIOUS build. Either way the restart below would not
    //    run what was just fetched, while every step still reported success.
    let service_ref = format!("{image}:{tag}");
    engine
        .tag(&reference, &service_ref)
        .await
        .with_context(|| format!("tagging {reference} as {service_ref}"))?;

    // 3. Restart the container AFTER the pull and tag so the new image takes effect.
    engine
        .restart(&cfg.container)
        .await
        .with_context(|| format!("restarting container {}", cfg.container))?;

    // 4. Record the now-running digest. This happens IMMEDIATELY after the
    //    restart, before the pointer write, because at this instant the
    //    container really is running `digest` - that is an observed fact, not a
    //    plan. Deferring it behind the pointer write would let a pointer failure
    //    leave `Hello` reporting an image the device is demonstrably not running,
    //    which is the mirror image of the skew that moving the pointer after the
    //    restart fixed.
    *state
        .running_digest
        .lock()
        .expect("running_digest mutex poisoned") = digest.to_string();

    // 4. Rewrite the active-image pointer on the WRITABLE partition, only once
    //    the restart has actually succeeded. The pointer records what the device
    //    IS running, so writing it ahead of the restart would leave it claiming
    //    the new image while the container still ran the old one on any restart
    //    failure. Nothing consumes the pointer during startup, so it has no
    //    reason to precede the restart. The path is derived from
    //    cfg.writable_root, never a read-only rootFS path.
    //
    //    A failure here is reported, not propagated. The pull and the restart
    //    both succeeded, so the sync did happen; failing the whole operation
    //    over the bookkeeping record would tell the host the sync did not take
    //    effect when it did, and the host's only recovery - retry the same
    //    digest - cannot fix a full or unwritable /var anyway.
    let active = ActiveImage {
        image: image.to_string(),
        tag: tag.to_string(),
        digest: digest.to_string(),
    };
    // One terminal line per outcome, and only the Ok arm says "sync complete".
    // A single unconditional success line here claimed "pointer rewritten" on the
    // failure branch too, so grepping for the obvious success string reported a
    // current pointer when no file had been written - the warning above it was
    // right, but the success line is the one that reads as authoritative.
    match write_active_image(&cfg.writable_root, &active) {
        Ok(path) => info!(
            %image, %tag, %digest, pointer = %path.display(),
            "sync complete: pulled, container restarted, active-image pointer rewritten"
        ),
        Err(e) => warn!(
            error = %e,
            %image, %tag, %digest,
            "sync incomplete: pulled and container restarted, but the active-image \
             pointer could not be written; the running digest is reported from memory \
             only and will be lost on restart"
        ),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, Mutex};

    /// An in-process engine that records its ordered calls; optionally fails the
    /// pull to exercise the abort path. Never execs a real engine.
    struct FakeEngine {
        calls: Arc<Mutex<Vec<String>>>,
        fail_pull: bool,
        fail_restart: bool,
    }

    impl FakeEngine {
        fn new(calls: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                calls,
                fail_pull: false,
                fail_restart: false,
            }
        }

        fn failing_pull(calls: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                calls,
                fail_pull: true,
                fail_restart: false,
            }
        }

        fn failing_restart(calls: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                calls,
                fail_pull: false,
                fail_restart: true,
            }
        }
    }

    impl Engine for FakeEngine {
        async fn pull(&self, reference: &str) -> Result<()> {
            self.calls.lock().unwrap().push(format!("pull:{reference}"));
            if self.fail_pull {
                anyhow::bail!("simulated pull failure");
            }
            Ok(())
        }

        async fn tag(&self, source: &str, target: &str) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("tag:{source}->{target}"));
            Ok(())
        }

        async fn restart(&self, container: &str) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("restart:{container}"));
            if self.fail_restart {
                anyhow::bail!("simulated restart failure");
            }
            Ok(())
        }
    }

    fn test_cfg(writable_root: PathBuf) -> SyncConfig {
        SyncConfig {
            loopback_port: 15151,
            container: "avocado-dev".to_string(),
            writable_root,
        }
    }

    // Falsifier 1: a sync must restart the container AFTER the pull. This test
    // asserts BOTH happened and that pull strictly precedes restart, so an
    // implementation that skipped the restart (or reordered it) fails here.
    #[tokio::test]
    async fn sync_pulls_then_restarts_in_order() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let engine = FakeEngine::new(calls.clone());
        let tmp = tempfile::tempdir().unwrap();
        let cfg = test_cfg(tmp.path().to_path_buf());
        let state = crate::AgentState::new("dev-01".to_string());

        on_sync(&engine, &cfg, "my-app", "dev", "sha256:new", &state)
            .await
            .expect("sync succeeds");

        let recorded = calls.lock().unwrap().clone();
        assert_eq!(
            recorded.len(),
            3,
            "expected pull, tag, then restart, got {recorded:?}"
        );
        assert!(
            recorded[0].starts_with("pull:"),
            "first call must be the pull: {recorded:?}"
        );
        assert!(
            recorded[1].starts_with("tag:"),
            "the tag must land between the pull and the restart: {recorded:?}"
        );
        assert!(
            recorded[2].starts_with("restart:"),
            "the restart must come last: {recorded:?}"
        );
        assert_eq!(recorded[0], "pull:127.0.0.1:15151/my-app@sha256:new");
        assert_eq!(recorded[2], "restart:avocado-dev");
    }

    // The pull lands a digest-pinned image. Unless something points the ref the
    // service actually names at it, the restart re-runs whatever that ref already
    // meant - the old image, or nothing at all when the target never had the tag.
    #[tokio::test]
    async fn sync_tags_the_pulled_digest_before_restarting() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let engine = FakeEngine::new(calls.clone());
        let tmp = tempfile::tempdir().unwrap();
        let cfg = test_cfg(tmp.path().to_path_buf());
        let state = crate::AgentState::new("dev-01".to_string());

        on_sync(&engine, &cfg, "my-app", "dev", "sha256:new", &state)
            .await
            .expect("sync succeeds");

        let recorded = calls.lock().unwrap().clone();
        assert_eq!(
            recorded,
            vec![
                "pull:127.0.0.1:15151/my-app@sha256:new".to_string(),
                "tag:127.0.0.1:15151/my-app@sha256:new->my-app:dev".to_string(),
                "restart:avocado-dev".to_string(),
            ],
            "the pulled digest must be tagged as the service's ref BEFORE the restart: {recorded:?}"
        );
    }

    // Falsifier 2: the pointer lands on the writable partition, never a rootFS
    // path, and round-trips {image, tag, digest}.
    #[tokio::test]
    async fn sync_writes_pointer_under_writable_root_and_round_trips() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let engine = FakeEngine::new(calls);
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let cfg = test_cfg(root.clone());
        let state = crate::AgentState::new("dev-01".to_string());

        on_sync(&engine, &cfg, "my-app", "dev", "sha256:new", &state)
            .await
            .expect("sync succeeds");

        let pointer = active_image_path(&root);
        assert!(
            pointer.starts_with(&root),
            "pointer must live under the writable root: {}",
            pointer.display()
        );
        assert!(pointer.exists(), "pointer file must be written");

        let raw = std::fs::read_to_string(&pointer).unwrap();
        let back: ActiveImage = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            back,
            ActiveImage {
                image: "my-app".to_string(),
                tag: "dev".to_string(),
                digest: "sha256:new".to_string(),
            }
        );
    }

    // The pointer path is derived from the writable root, not a rootFS constant.
    // The production root is on the writable partition and matches main.rs's
    // bootstrap root.
    #[test]
    fn active_image_path_derives_from_writable_root_never_rootfs() {
        assert!(
            WRITABLE_ROOT.starts_with("/var/lib/avocado"),
            "writable root must be on the writable partition"
        );
        for ro in ["/usr", "/etc"] {
            assert!(
                !WRITABLE_ROOT.starts_with(ro),
                "writable root must not be under the read-only rootFS path {ro}"
            );
        }
        let path = active_image_path(Path::new(WRITABLE_ROOT));
        assert!(
            path.starts_with(WRITABLE_ROOT),
            "pointer path must be derived from the writable root"
        );
        assert_eq!(path.file_name().unwrap(), ACTIVE_IMAGE_FILE);
        // Same writable partition as main.rs's bootstrap root.
        assert!(
            Path::new(crate::DEFAULT_BOOTSTRAP_PATH).starts_with(WRITABLE_ROOT),
            "pointer root must match main.rs's bootstrap root partition"
        );
    }

    // Falsifier 3 (positive): after a successful sync the running digest is the
    // synced digest.
    #[tokio::test]
    async fn sync_updates_running_digest_on_success() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let engine = FakeEngine::new(calls);
        let tmp = tempfile::tempdir().unwrap();
        let cfg = test_cfg(tmp.path().to_path_buf());
        let state = crate::AgentState::new("dev-01".to_string());

        on_sync(&engine, &cfg, "my-app", "dev", "sha256:new", &state)
            .await
            .expect("sync succeeds");

        assert_eq!(*state.running_digest.lock().unwrap(), "sha256:new");
    }

    // Edge: a pull failure aborts before the restart, does not write the
    // pointer, and does not rewrite the running digest.
    #[tokio::test]
    async fn pull_failure_aborts_before_restart_and_keeps_digest() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let engine = FakeEngine::failing_pull(calls.clone());
        let tmp = tempfile::tempdir().unwrap();
        let cfg = test_cfg(tmp.path().to_path_buf());
        let state = crate::AgentState::new("dev-01".to_string());
        // A prior digest that must survive an aborted sync unchanged.
        *state.running_digest.lock().unwrap() = "sha256:old".to_string();

        let err = on_sync(&engine, &cfg, "my-app", "dev", "sha256:new", &state)
            .await
            .expect_err("pull failure must abort the sync");
        assert!(
            err.to_string().contains("loopback proxy"),
            "unexpected error: {err}"
        );

        let recorded = calls.lock().unwrap().clone();
        assert_eq!(
            recorded.len(),
            1,
            "only the pull should have been attempted: {recorded:?}"
        );
        assert!(recorded[0].starts_with("pull:"));
        assert!(
            !active_image_path(tmp.path()).exists(),
            "pointer must not be written when the pull fails"
        );
        assert_eq!(
            *state.running_digest.lock().unwrap(),
            "sha256:old",
            "running digest must be unchanged after an aborted sync"
        );
    }

    // Edge: a restart failure must not leave the pointer claiming an image the
    // device is not running. The pull succeeded so the bytes are on the device,
    // but the container still runs the OLD image - a pointer written ahead of
    // the restart would disagree with both reality and `running_digest`.
    #[tokio::test]
    async fn restart_failure_leaves_no_stale_pointer() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let engine = FakeEngine::failing_restart(calls.clone());
        let tmp = tempfile::tempdir().unwrap();
        let cfg = test_cfg(tmp.path().to_path_buf());
        let state = crate::AgentState::new("dev-01".to_string());
        *state.running_digest.lock().unwrap() = "sha256:old".to_string();

        let err = on_sync(&engine, &cfg, "my-app", "dev", "sha256:new", &state)
            .await
            .expect_err("restart failure must fail the sync");
        assert!(
            err.to_string().contains("restarting container"),
            "unexpected error: {err}"
        );

        let recorded = calls.lock().unwrap().clone();
        assert_eq!(
            recorded,
            vec![
                "pull:127.0.0.1:15151/my-app@sha256:new".to_string(),
                "tag:127.0.0.1:15151/my-app@sha256:new->my-app:dev".to_string(),
                "restart:avocado-dev".to_string(),
            ],
            "the pull and the restart attempt should both be recorded: {recorded:?}"
        );
        assert!(
            !active_image_path(tmp.path()).exists(),
            "pointer must not claim the new image when the restart failed"
        );
        assert_eq!(
            *state.running_digest.lock().unwrap(),
            "sha256:old",
            "running digest must still report the image actually running"
        );
    }

    // The mirror of `restart_failure_leaves_no_stale_pointer`: the restart
    // SUCCEEDS and the pointer write then fails (ENOSPC, an unwritable /var, a
    // state root pointing somewhere impossible). The container really is running
    // the new image at that point, so `running_digest` must say so - reporting
    // the old digest in the reconnect `Hello` would tell the host to re-sync an
    // image the device already runs, and would be a claim contradicted by the
    // device itself.
    #[tokio::test]
    async fn pointer_write_failure_after_a_successful_restart_still_reports_the_new_digest() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let engine = FakeEngine::new(calls.clone());
        let tmp = tempfile::tempdir().unwrap();

        // A writable_root whose parent is a FILE: create_dir_all cannot succeed,
        // so write_active_image fails while pull and restart do not.
        let blocker = tmp.path().join("not-a-dir");
        std::fs::write(&blocker, b"x").unwrap();
        let cfg = test_cfg(blocker.join("state"));

        let state = crate::AgentState::new("dev-01".to_string());
        *state.running_digest.lock().unwrap() = "sha256:old".to_string();

        on_sync(&engine, &cfg, "my-app", "dev", "sha256:new", &state)
            .await
            .expect("a pointer-write failure must not fail a sync that already took effect");

        let recorded = calls.lock().unwrap().clone();
        assert_eq!(
            recorded,
            vec![
                "pull:127.0.0.1:15151/my-app@sha256:new".to_string(),
                "tag:127.0.0.1:15151/my-app@sha256:new->my-app:dev".to_string(),
                "restart:avocado-dev".to_string(),
            ],
            "the pull and restart must both have run: {recorded:?}"
        );
        assert!(
            !active_image_path(&cfg.writable_root).exists(),
            "the pointer genuinely could not be written"
        );
        assert_eq!(
            *state.running_digest.lock().unwrap(),
            "sha256:new",
            "Hello must report the digest the container is actually running, \
             even though the pointer write failed"
        );
    }

    // The engine classification the startup warning keys on. Pure, so the
    // decision is testable without an environment or a spawned process - which
    // is the whole reason it was split out of `CommandEngine::from_env`, where it
    // ran once per Sync frame and so could not warn at startup at all.
    #[test]
    fn podman_is_classified_unsupported_and_docker_is_not() {
        assert!(is_unsupported_engine("podman"));
        assert!(
            is_unsupported_engine("/usr/bin/podman"),
            "an absolute path must classify the same as a bare name"
        );
        assert!(
            is_unsupported_engine("podman-remote"),
            "a podman variant is still podman"
        );

        assert!(!is_unsupported_engine("docker"));
        assert!(!is_unsupported_engine("/usr/bin/docker"));
        assert!(
            !is_unsupported_engine(DEFAULT_ENGINE),
            "the default engine must never be classified unsupported"
        );
    }

    #[test]
    fn loopback_reference_pins_digest_on_loopback_host() {
        let reference = loopback_reference(15151, "my-app", "sha256:abc");
        assert_eq!(reference, "127.0.0.1:15151/my-app@sha256:abc");
    }
}
