#!/usr/bin/env bash
# Build the extra Debian artifact on cargo-dist's Linux global-artifact runner.
set -euo pipefail

cargo install cargo-deb --version 3.8.0 --locked
cargo deb --locked --profile dist --output target/debian/wirelint_amd64.deb

package=target/debian/wirelint_amd64.deb
staging=$(mktemp -d)
trap 'rm -rf -- "$staging"' EXIT
dpkg-deb --extract "$package" "$staging"

expected=$(cargo run --quiet --locked --profile dist -- --version)
actual=$("$staging/usr/bin/wirelint" --version)
if [[ "$actual" != "$expected" ]]; then
    echo "Debian package version mismatch: expected '$expected', got '$actual'" >&2
    exit 1
fi
"$staging/usr/bin/wirelint" --help
for file in LICENSE THIRD_PARTY_NOTICES.md; do
    if [[ ! -f "$staging/usr/share/doc/wirelint/$file" ]]; then
        echo "Debian package is missing $file" >&2
        exit 1
    fi
done

(cd target/debian && sha256sum wirelint_amd64.deb > wirelint_amd64.deb.sha256)
