#!/bin/bash
# Extract public API surface using cargo-public-api
# Requires cargo-public-api: cargo install cargo-public-api
#
# Output is piped through a canonicalising filter before it is written to the
# baseline: rustdoc's synthesised auto-trait `impl` bounds
# (Send/Sync/Unpin/Freeze/UnsafeUnpin) have no guaranteed order across
# toolchain versions, so an un-normalised baseline records a property of the
# machine that generated it (which nightly happened to run) rather than a
# property of the API. The filter only reorders adjacent marker-bound runs;
# every other token on every line is passed through byte-identically.
#
# Toolchain pin (PUBLIC_API_TOOLCHAIN). That filter cannot absorb every nightly
# difference: on 2026-09-21 a new nightly (rustc bba531001 2026-09-20) started
# rendering derived return types as `-> Self` instead of the fully qualified
# path, and the floating nightly CI installed turned 606 baseline lines red
# with no public item added, removed or changed. The nightly is therefore
# chosen through ONE variable. CI sets it to a dated nightly in the
# api-surface job of .github/workflows/ci.yml and installs that same name;
# unset, it is the local `nightly`, exactly as before.
#
# cargo-public-api 0.52.0 has no --toolchain flag. It reads RUSTUP_TOOLCHAIN,
# which `cargo +<toolchain>` sets, honours it when it is a nightly, and
# otherwise warns and SILENTLY switches to plain `nightly`. A non-nightly value
# would look applied and not be, so it is refused here instead.
#
# To move the pin: set PUBLIC_API_TOOLCHAIN in ci.yml to the new dated nightly,
# regenerate the baseline with that same toolchain
#   PUBLIC_API_TOOLCHAIN=nightly-YYYY-MM-DD ./scripts/extract-public-api.sh .project/current-exports.txt
# and commit both together.
set -euo pipefail

OUTPUT_FILE="${1:-.project/current-exports.txt}"

PUBLIC_API_TOOLCHAIN="${PUBLIC_API_TOOLCHAIN:-nightly}"
if ! [[ "${PUBLIC_API_TOOLCHAIN}" =~ ^nightly(-[0-9]{4}-[0-9]{2}-[0-9]{2})?$ ]]; then
    echo "❌ PUBLIC_API_TOOLCHAIN='${PUBLIC_API_TOOLCHAIN}' is not 'nightly' or 'nightly-YYYY-MM-DD'." >&2
    echo "   cargo-public-api silently falls back to plain 'nightly' for any other value," >&2
    echo "   so the pin would look applied and not be. Refusing to extract." >&2
    exit 2
fi

echo "Extracting public API surface using cargo-public-api (toolchain: ${PUBLIC_API_TOOLCHAIN})..."

# Check if cargo-public-api is installed
if ! command -v cargo-public-api &> /dev/null; then
    echo "❌ cargo-public-api not found. Installing..."
    cargo install cargo-public-api || {
        echo "❌ Failed to install cargo-public-api"
        echo "   This tool requires OpenSSL 3.0.0+"
        echo "   If in devcontainer, rebuild with: Ctrl+Shift+P -> 'Dev Containers: Rebuild Container'"
        exit 1
    }
fi

# Generate simplified public API list
echo "# Public API Surface - Generated $(date -u +"%Y-%m-%d %H:%M:%S UTC")" > "$OUTPUT_FILE"
echo "# This file tracks all publicly exported items from the paladin crate" >> "$OUTPUT_FILE"
echo "# Generated using cargo-public-api v$(cargo public-api --version | awk '{print $2}')" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Extract public API in simplified format, canonicalising auto-trait marker
# bound ordering before it lands in the baseline.
# Note: cargo public-api may emit warnings to stderr, but still succeeds
cargo "+${PUBLIC_API_TOOLCHAIN}" public-api --simplified 2>/dev/null | python3 scripts/normalize-api-bounds.py >> "$OUTPUT_FILE" || {
    echo "❌ Failed to generate API surface"
    echo "   Check that the toolchain is installed: rustup toolchain install ${PUBLIC_API_TOOLCHAIN}"
    exit 1
}

# Count total items
TOTAL=$(grep -c "^pub " "$OUTPUT_FILE" || echo "0")
echo "" >> "$OUTPUT_FILE"
echo "## Summary" >> "$OUTPUT_FILE"
echo "Total public items: $TOTAL" >> "$OUTPUT_FILE"

echo "✅ API surface extracted to $OUTPUT_FILE ($TOTAL items)"
