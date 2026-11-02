# Phase 56 Runbook: web3sec return relay and triage (DOGF-02, D-15, D-16)

Stage B runbook. The web3sec consumer's workspace is not in this repository and nothing about its
return can be inferred, so this runbook only prepares the relay, the paste slot and the triage.
Paste slots `W1` and `W2` at the end are read by plan 56-10. Both are empty in this commit.

## Purpose and decision

- **Requirement:** DOGF-02. The web3sec consumer's return against `v0.11.0`
  (`47-HANDOFF-web3sec.md` section 6) is received, pasted verbatim, and triaged into the findings
  log.
- **D-15:** triage starts only from a verbatim paste of the consumer's actual return. Nothing is
  reconstructed, summarised or invented.
- **D-16:** if no return has arrived when everything else is done, the maintainer decides what
  happens (see `## If no return has arrived`). The agent never picks.
- **Folded todo:** `.planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md`
  (owner `repo maintainer (relay to web3sec)`).

## Return contract

Source: `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/47-HANDOFF-web3sec.md`,
section "6. Return contract (what Paladin needs back)". The consumer reports four things:

1. The **consumer commit SHA** containing the pointer bump to the `v0.11.0` tag (`git -C paladin rev-parse HEAD`
   in the consumer must print `70657919beadbd341a72be2d0107080017a803f0`).
2. The **final D-2 choice**, and which web3sec types changed if you went lossless (`RoutineReport`, the outcome
   types).
3. **Gate output:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo check --workspace` and `cargo test --workspace` totals (passed / failed / ignored), reported against
   your latest pre-bump figure with the figure and its source named. The figures on record are **2,592 / 0 / 2**
   (the earlier figure) and **2,683 / 0 / 2** (the latest, see below).
4. **Any divergence** from this hand-off, including real `use` lines not covered by the muster, and any step of
   section 10.8 that did not read as written.

Baselines, so you can check which one the consumer used:

- **2,683 / 0 / 2** is the latest figure on record: web3sec's 2026-10-24 findings
  (`46-FEEDBACK-web3sec.md`, section 1) at `e28e43b5`.
- **2,592 / 0 / 2** is the earlier figure (the trial on `v0.10.1`, `49-HANDOFF-web3sec.md`,
  re-measured at Paladin `21426037`).
- The hand-off asks the consumer to name which figure it used. The folded todo still carries
  2,592 / 0 / 2; that is a stale figure, not a defect in either measurement.
- `MIGRATION.md` section 10.8 is byte-frozen (D-00c).

## Before you start

You need a channel to the web3sec maintainers. Nothing else is required of this checkout.

## Steps

1. Relay the request to the consumer, pointing them at section 6 of
   `47-HANDOFF-web3sec.md` (the four items above).
2. When the reply arrives, paste it verbatim into slot `W1`. No editing, no summarising, no
   reformatting. If it came as several messages, paste them in order with a blank line between.
3. Put the date received and the channel (and any note of your own) into slot `W2`.
4. A return that arrives after the phase closes is still pasted and triaged (D-16). It is then
   handled as a new finding batch against the next patch release.
5. Do not write an example, a sample or a placeholder return into `W1` for any reason. The slot
   stays empty until the real return exists.

## Redaction

The return is third-party text. If it contains a credential, token, private repository URL you do
not want published, or customer data, redact it by hand before pasting and say so in `W2`. Do not
paste a secret into this repository.

## Paste slots

### W1 — the consumer's return, verbatim

<!-- paste below this line; redacted -->
```
```

### W2 — received date, channel, maintainer note

<!-- paste below this line; redacted -->
```
```

## Triage checklist

For plan 56-10.

- [ ] Triage starts only from `W1` (D-15). If `W1` is empty, stop: go to `## If no return has
  arrived`.
- [ ] The SHA must equal `70657919beadbd341a72be2d0107080017a803f0`. A different SHA is its own DF.
- [ ] Record the D-2 choice, and which web3sec types changed if the consumer went lossless.
- [ ] Gate totals: compare each of `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo check --workspace` and `cargo test --workspace` against the figure the
  consumer named. A total that regresses against the consumer's named latest figure (2,683 / 0 /
  2) is its own DF.
- [ ] Each divergence is one DF with `**By:** web3sec`. That includes every real `use` line the
  muster missed (the muster is pinned by `tests/consumer_surface_muster.rs`) and every section 10.8
  step that did not read as written. Quote the consumer's wording inside a code fence in the DF
  (the findings-log checker ignores fenced text).
- [ ] Severity per D-04: S2 by default for anything that breaks the consumer's compile or tests;
  S3 for a muster gap with no behavioural effect.
- [ ] Disposition per D-05. A `MIGRATION.md` section 10.8 wording fix lands outside the
  byte-frozen section 10.8, in `MIGRATION.md` section 11 or `CHANGELOG.md` `[Unreleased]` (D-00c).
- [ ] Every S1 and S2 entry is appended `open → promote in v0.11.1` and goes to Stage C (D-06).
- [ ] Entries parse under `make check-findings-log`; the next free id is the one after the last
  `### DF-nn` in `.project/v0.11.1/02-dogfood-findings-log.md`.

## If no return has arrived

This section is for the maintainer. When everything else in the phase is done and `W1` is still
empty, choose one. The agent never picks either option, and never fills `W1`.

- **(a) Hold the tag.** The phase stays open. Record a blocker line in `.planning/STATE.md`
  naming DOGF-02 and the date.
- **(b) Re-home DOGF-02 with a named owner and a date.** Record it in all three places:
  1. the `.planning/REQUIREMENTS.md` DOGF-02 traceability row, status
     `Re-homed (owner: <name>, by: <YYYY-MM-DD>)`;
  2. a dated note under the DOGF-02 checklist line in the same file;
  3. the folded todo `2026-10-14-web3sec-single-bump-to-v0-11-0`: set its `owner:` and
     `recheck_by:` front matter and add a dated status note. The todo stays in `pending/`.
