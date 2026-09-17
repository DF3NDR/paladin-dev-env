#!/usr/bin/env bash
# check-all-examples.sh
#
# Rewritten (D-14) to mirror the `.github/workflows/ci.yml` "Example Muster"
# job's feature-split invocations exactly, in the same order, so a green
# local run and a green CI run mean the same thing.
#
# The previous version of this script ran a per-file `cargo check`, naming
# one example and enabling every workspace feature at once. Enabling every
# feature always includes whatever feature a target is gated on, so that
# form silently satisfies every `required-features` gate in Cargo.toml and
# can therefore never reproduce the exact gap CI's feature split exists to
# catch: a bare `cargo build --examples` selector SILENTLY SKIPS any target
# whose required-features are unmet (exit 0, no warning, no error). This
# script never checks a single example with every feature enabled; each
# gated target gets its own explicit `cargo build --example ... --features
# "..."` invocation instead, matching `Cargo.toml`'s `[[example]]`
# `required-features` lists verbatim.
#
# This is intentionally NOT wired into the pre-push hook: several full
# example builds are too slow for a push hook (D-14). It is invoked via
# `make check-examples`.
#
# Usage:  ./scripts/check-all-examples.sh
# Exit:   0 if every invocation succeeds and every expected binary was produced.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$WORKSPACE_ROOT"

CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${CYAN}==========================================${NC}"
echo -e "${CYAN}Example Muster (local mirror of CI)${NC}"
echo -e "${CYAN}==========================================${NC}"
echo ""

# --- 1. Default features: every auto-discovered target (no required-features) -
echo -e "${CYAN}[1/7] Build examples (default features -- auto-discovered targets)${NC}"
cargo build --examples --offline

# --- 2. vision: vision_analysis, vision_battalion ------------------------------
echo -e "${CYAN}[2/7] Build examples (vision -- vision_analysis, vision_battalion)${NC}"
cargo build --example vision_analysis --example vision_battalion --features "vision,llm-openai" --offline

# --- 3. content-processing: document_processing --------------------------------
echo -e "${CYAN}[3/7] Build examples (content-processing -- document_processing)${NC}"
cargo build --example document_processing --features "content-processing" --offline

# --- 4. web-server: http_service_host, webhook_receiver ------------------------
echo -e "${CYAN}[4/7] Build examples (web-server -- http_service_host, webhook_receiver)${NC}"
cargo build --example http_service_host --example webhook_receiver --features "web-server" --offline

# --- 5. web-server,dev-ui: platform_api_client ----------------------------------
echo -e "${CYAN}[5/7] Build examples (web-server,dev-ui -- platform_api_client)${NC}"
cargo build --example platform_api_client --features "web-server,dev-ui" --offline

# --- 6. redis-cache: node_result_cache ------------------------------------------
echo -e "${CYAN}[6/7] Build examples (redis-cache -- node_result_cache)${NC}"
cargo build --example node_result_cache --features "redis-cache" --offline

# --- 7. otel: observability_otel_export -----------------------------------------
echo -e "${CYAN}[7/7] Build examples (otel -- observability_otel_export)${NC}"
cargo build --example observability_otel_export --features "otel" --offline

# --- Binary-count assertion ------------------------------------------------------
# Derives the expected count from the .rs files under examples/ (self-correcting),
# and matches on basenames rather than raw `ls` since cargo also emits `.d`
# dependency files and hash-suffixed duplicates into target/debug/examples.
echo ""
echo -e "${CYAN}Asserting every example binary was produced...${NC}"
EXPECTED=$(find examples -name '*.rs' | wc -l)
FOUND=0
MISSING=""
for f in examples/*.rs; do
    name=$(basename "$f" .rs)
    if [ -x "target/debug/examples/$name" ] && [ ! -d "target/debug/examples/$name" ]; then
        FOUND=$((FOUND + 1))
    else
        MISSING="$MISSING $name"
    fi
done
echo "Expected: $EXPECTED example binaries; found: $FOUND"
if [ "$FOUND" -ne "$EXPECTED" ]; then
    echo -e "${RED}Missing:$MISSING${NC}" >&2
    exit 1
fi

echo ""
echo -e "${GREEN}All $EXPECTED example binaries present.${NC}"
