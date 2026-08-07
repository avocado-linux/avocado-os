# hello — container dev loop worked example

A demo container that starts at boot, comes back after a reboot, and hot-reloads
when you push a new image. Development only; it is a payload, not a platform
component.

## Using it

Add the extension to your runtime, then declare the dev-mode wiring on the same
runtime:

```yaml
runtimes:
  dev:
    target: imx93-frdm
    extensions:
      - docker          # required: provides dockerd, the bridge modules, CA bundle
      - hello
      - container-agent-dev   # only if you want hot reload

    container_dev:
      images:
        - ref: peridionick/hello-flask:py311
          service: container-hello
```

Build the runtime and provision. The container is running at
`http://<device>:8080` on first boot, with no registry contact — the image is
baked into the `/var` partition.

## The `service:` field is not optional

It names the systemd unit that owns the container, and without it sync silently
does the wrong thing.

With `service:` set, the agent restarts `container-hello.service`, so `ExecStart`
re-runs `docker run` and re-resolves the tag to the newly pulled image.

Without it, the agent falls back to `<engine> restart <container>`, which
re-executes the existing container object. That object is bound to the image ID
it was created with and never re-resolves the tag — so the pull succeeds, the
tag succeeds, the restart succeeds, the new digest is recorded, and the device
goes on running the old image. Every layer reports success and nothing changed.
See the comment block in `container-agent-dev/agent/src/sync.rs`.

## Two sharp edges worth knowing

**`docker_images:` is honored only on a runtime build.** A standalone
`avocado ext build` of this extension parses the key, ignores it, and succeeds —
producing an extension with no image in it. The container then falls back to
pulling at start, which works only because the `docker` extension installs a CA
bundle.

**Units must live in `overlay/usr/lib/systemd/system/`.** `enable_services` looks
for the unit at exactly that path and creates the `.wants` symlink only if it
finds one; a unit placed in `overlay/etc/systemd/system/` produces an extension
that merges cleanly, presents the unit, and leaves it `disabled` forever. The
build prints a warning, but the build still succeeds — so it scrolls past.

## Not a production pattern

This runs a container from a hand-written unit shipped inside an extension.
Nothing above it knows what that unit runs: the fleet cannot ask what container a
device should be running, verify one is, or notice divergence. A container
updated via `dev sync` also persists across reboots — the pulled image stays in
`/var/lib/docker` under its tag — so a device can end up durably running an image
that exists in no build and no fleet record, and a reprovision reverts it
silently. Dev-to-prod graduation is out of scope for Container Dev Mode v1.
