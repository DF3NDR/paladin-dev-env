# Request to web3sec: the v0.11.0 return report

**From:** Paladin (`DF3NDR/paladin-dev-env`) maintainer, Phase 56 (v0.11.1 dogfood findings)
**To:** the Web3 Security Paladin ("web3sec") consumer agent
**Date:** 2026-11-04
**Attach:** `47-HANDOFF-web3sec.md` (the 2026-10-26 handoff; its §6 is the contract this request fills)

## Why you are receiving this

The maintainer reports that your bump to Paladin `v0.11.0` is complete and succeeded. Paladin never
received the return report §6 of the handoff asks for, and Paladin's v0.11.1 release gate (SHIP-10)
admits DOGF-02 only against a recorded return. Nothing is wrong on your side; this is bookkeeping
the handoff asked for and nobody carried back.

## What to produce

One Markdown reply, under the four headings below, in this order. Fill each from your repository
state; do not estimate. If a figure is not recoverable, say `not captured` under that heading
rather than reconstructing it. Paste no credentials (no API keys, tokens, `.env` contents).

### 1. Consumer commit SHA

The web3sec commit that carries the pointer bump to the `v0.11.0` tag, the adaptation and the
`Cargo.lock` together (the single-bump rule). In that checkout:

```sh
git rev-parse HEAD
git -C paladin rev-parse HEAD          # must print 70657919beadbd341a72be2d0107080017a803f0
git -C paladin describe --tags --exact-match   # must print v0.11.0
```

Paste the three outputs.

### 2. D-2 choice

`lossy` or `lossless`. If lossless, list the web3sec types that changed (`RoutineReport`, the
outcome types, any others).

### 3. Gate output, at that commit

```sh
cargo fmt --check; echo "fmt exit=$?"
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3; echo "clippy exit=${PIPESTATUS[0]}"
cargo check --workspace 2>&1 | tail -2; echo "check exit=${PIPESTATUS[0]}"
cargo test --workspace 2>&1 | grep -E "^test result:" | awk '{p+=$4; f+=$6; i+=$8} END {print "passed="p" failed="f" ignored="i}'
```

Paste the four results. Then state your **pre-bump baseline** and its source. The two figures
Paladin has on record are `2,592 / 0 / 2` (your 2026-10-18/19 hand-off) and `2,683 / 0 / 2`
(your 2026-10-24 findings at `e28e43b5`). Say which one you compared against, or give a third
with its commit.

### 4. Divergences

Every place the handoff or Paladin's `MIGRATION.md` §10.8 ("Library consumers: v0.10.1 → v0.11.0")
did not read as written: a `use` line the muster did not cover, a step that needed a different
command, a type or signature that differed from the guide, a behaviour change you had to code
around. One bullet each, verbatim where there was an error message. If there were none, write
`none`.

## Delivery

Return the reply to the Paladin maintainer in chat. The maintainer relays it into Paladin's
findings log; each divergence becomes one log entry, triaged against Paladin's current tree.
A clean return (`none` divergences, gates green) closes DOGF-02 as met.
