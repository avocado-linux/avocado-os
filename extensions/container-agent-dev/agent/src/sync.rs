//! Device-side sync handler for Container Dev Mode (design D9; task 6.3).
//!
//! On a [`crate::HostFrame::Sync`] the agent, IN ORDER:
//!   1. **Pulls** the new image through the device loopback proxy
//!      (`127.0.0.1:<port>`, task 6.2). The pull rides the proxy -> host bulk
//!      HTTPS path; it NEVER re-pulls over the control WebSocket (D9 splits
//!      bulk from control).
//!   2. **Restarts the container** via the engine AFTER the pull, so the new
//!      image takes effect. The read-only rootFS is untouched throughout.
//!   3. **Rewrites the active-image pointer** — a small JSON file recording the
//!      now-active `{image, tag, digest}` — on the **writable partition**
//!      (`/var/lib/avocado/container-dev/active-image.json`), only once the
//!      restart succeeded, so a failed restart never leaves the pointer ahead of
//!      what the device actually runs. The path is derived from
//!      [`WRITABLE_ROOT`], never a read-only rootFS path (`/usr`, `/etc`, the
//!      image rootfs).
//!   4. Updates [`crate::AgentState::running_digest`] so task 6.1's reconnect
//!      `Hello` reports it.
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
        if binary.contains("podman") {
            // Under the shipped systemd unit podman cannot work: it needs
            // CAP_SYS_ADMIN, namespaces, cgroup writes and a writable
            // /var/lib/containers, all of which the unit's sandbox denies. It
            // would fail at store init and, because on_sync's error is only
            // logged, present as syncs that silently do nothing. Say so once at
            // startup so the cause is visible.
            //
            // A warning rather than a refusal: the sandbox is the unit's, not
            // the binary's, and the agent run directly for debugging has no such
            // restriction.
            warn!(
                engine = %binary,
                "podman is not supported under the container-agent-dev systemd unit \
                 (its sandbox denies the privileges rootful podman needs); expect pulls \
                 to fail unless this agent is running outside that unit"
            );
        }
        Self { binary }
    }
}

impl Engine for CommandEngine {
    async fn pull(&self, reference: &str) -> Result<()> {
        run_engine(&self.binary, &["pull", reference]).await
    }

    async fn restart(&self, container: &str) -> Result<()> {
        run_engine(&self.binary, &["restart", container]).await
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

/// Handle a `Sync`: pull -> rewrite pointer -> restart -> record digest.
///
/// The order is load-bearing: a pull failure aborts before the pointer rewrite,
/// the restart, and the digest update, so a failed sync never claims a new
/// image is running and never mutates state. All writes target the writable
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

    // 2. Restart the container AFTER the pull so the new image takes effect.
    engine
        .restart(&cfg.container)
        .await
        .with_context(|| format!("restarting container {}", cfg.container))?;

    // 3. Record the now-running digest. This happens IMMEDIATELY after the
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
    match write_active_image(&cfg.writable_root, &active) {
        Ok(path) => {
            info!(pointer = %path.display(), "active-image pointer rewritten on writable partition")
        }
        Err(e) => warn!(
            error = %e,
            %digest,
            "container restarted but the active-image pointer could not be written; \
             the running digest is still reported correctly"
        ),
    }

    info!(%image, %tag, %digest, "sync complete: pulled, pointer rewritten, container restarted");
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
            2,
            "expected exactly pull then restart, got {recorded:?}"
        );
        assert!(
            recorded[0].starts_with("pull:"),
            "first call must be the pull: {recorded:?}"
        );
        assert!(
            recorded[1].starts_with("restart:"),
            "second call must be the restart: {recorded:?}"
        );
        assert_eq!(recorded[0], "pull:127.0.0.1:15151/my-app@sha256:new");
        assert_eq!(recorded[1], "restart:avocado-dev");
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

    #[test]
    fn loopback_reference_pins_digest_on_loopback_host() {
        let reference = loopback_reference(15151, "my-app", "sha256:abc");
        assert_eq!(reference, "127.0.0.1:15151/my-app@sha256:abc");
    }
}
