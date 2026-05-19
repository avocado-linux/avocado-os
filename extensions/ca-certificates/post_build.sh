#!/bin/sh
# post_build hook for avocado-ext-ca-certificates.
#
# Runs after the ca-certificates package is installed into the extension
# sysroot but before the sysext/confext .raw images are sealed. Assembles
# /etc/ssl/certs/ca-certificates.crt from the cert list in
# /etc/ca-certificates.conf (the same source update-ca-certificates uses)
# so the bundle lands inside the confext .raw and is available at runtime
# without an ExecStartPre workaround.
#
# Env provided by the avocado CLI:
#   AVOCADO_BUILD_EXT_SYSROOT  $AVOCADO_EXT_SYSROOTS/avocado-ext-ca-certificates
#   AVOCADO_EXT_NAME           avocado-ext-ca-certificates
#   AVOCADO_TARGET             target machine
set -eu

sysroot="$AVOCADO_BUILD_EXT_SYSROOT"
conf="$sysroot/etc/ca-certificates.conf"
src_dir="$sysroot/usr/share/ca-certificates"
bundle_dir="$sysroot/etc/ssl/certs"
bundle="$bundle_dir/ca-certificates.crt"

if [ ! -d "$src_dir" ]; then
    echo "error: $src_dir not found; is ca-certificates installed in the sysroot?" >&2
    exit 1
fi

mkdir -p "$bundle_dir"
: > "$bundle"

added=0
if [ -f "$conf" ]; then
    # ca-certificates.conf format: one path per line, relative to
    # /usr/share/ca-certificates. '#' = comment, '!' prefix = disabled.
    while IFS= read -r line; do
        case "$line" in
            ''|'#'*|'!'*) continue ;;
        esac
        cert="$src_dir/$line"
        if [ -f "$cert" ]; then
            cat "$cert" >> "$bundle"
            added=$((added + 1))
        else
            echo "warning: listed cert not present: $line" >&2
        fi
    done < "$conf"
else
    # Fall back to globbing the default mozilla trust list if the conf
    # file is missing (e.g. ca-certificates postinst didn't fire under dnf
    # --installroot).
    echo "warning: $conf not found; falling back to mozilla glob" >&2
    for cert in "$src_dir"/mozilla/*.crt; do
        [ -f "$cert" ] || continue
        cat "$cert" >> "$bundle"
        added=$((added + 1))
    done
fi

if [ "$added" -eq 0 ]; then
    echo "error: no certificates were added to $bundle" >&2
    exit 1
fi

echo "post_build: wrote $bundle ($added certs)"
