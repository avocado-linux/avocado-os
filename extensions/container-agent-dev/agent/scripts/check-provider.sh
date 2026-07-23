#!/bin/bash
#
# check-provider.sh - enforce the ring-only rustls crypto provider for the
# Container Dev Mode device agent.
#
# aws-lc-rs is forbidden in this crate (assumption A9: the agent cross-compiles
# against the musl SDK, which clears only the `ring` stack - the same rule
# avocado-conn enforces). This guard fails the build if aws-lc-rs ever resolves
# into the dependency tree, and also asserts that `ring` IS present so a guard
# that cannot see the tree at all can never masquerade as a pass.
#
# Exit contract (three-way, deliberately NOT a bare `! cargo tree`):
#   0 - clean ring-only tree (aws-lc-rs absent, ring present)
#   1 - aws-lc-rs PRESENT (forbidden provider in the tree)
#   2 - guard error: the dependency tree could not be resolved (network, bad
#       manifest, unresolvable target, or ring itself missing). Never a pass.
#
# The check uses the default `cargo tree` edge set (includes dev-dependencies)
# so it is tree-wide: the agent Cargo.toml keeps aws-lc-rs "out of the tree
# entirely", and the rcgen dev-dependency is pinned to ring for exactly that.
#
# TARGET: when set (env var or first positional arg) the guard resolves against
# that target triple, matching the build's cross target. In the build hook this
# is mandatory (TARGET="$RUST_TARGET"). Run standalone with no TARGET, the guard
# checks the host triple as an ADVISORY check.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
manifest="$script_dir/../Cargo.toml"

target="${TARGET:-${1:-}}"

target_args=()
if [ -n "$target" ]; then
    target_args+=(--target "$target")
    scope="target $target"
else
    scope="host triple (advisory)"
fi

echo "check-provider: enforcing ring-only rustls provider ($scope)"

# Inverse-tree query for the forbidden provider. Capture status AND output so a
# non-zero exit does not abort under `set -e`. On success (aws-lc-rs present)
# stdout carries the offending inverse-tree path; on failure the captured text
# carries cargo's error on stderr.
set +e
awslc_out="$(cargo tree -i aws-lc-rs --manifest-path "$manifest" "${target_args[@]}" 2>&1)"
awslc_status=$?
set -e

if [ "$awslc_status" -eq 0 ]; then
    echo "check-provider: FAIL - forbidden crypto provider aws-lc-rs is in the tree:" >&2
    echo "$awslc_out" >&2
    exit 1
fi

if ! printf '%s\n' "$awslc_out" | grep -q 'did not match any packages'; then
    echo "check-provider: ERROR - guard could not resolve the dependency tree." >&2
    echo "check-provider: this is NOT a pass; refusing to certify the provider." >&2
    echo "$awslc_out" >&2
    exit 2
fi

# aws-lc-rs is provably absent. Backstop: `ring` MUST be present. A guard that
# cannot see ring cannot trust its aws-lc-rs result either.
set +e
cargo tree -i ring --manifest-path "$manifest" "${target_args[@]}" >/dev/null 2>&1
ring_status=$?
set -e

if [ "$ring_status" -ne 0 ]; then
    echo "check-provider: ERROR - ring is not present in the tree; guard cannot" >&2
    echo "check-provider: trust the aws-lc-rs result. Treating as a guard error." >&2
    exit 2
fi

echo "check-provider: PASS - aws-lc-rs absent, ring-only provider confirmed."
exit 0
