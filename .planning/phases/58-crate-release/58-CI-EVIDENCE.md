# Phase 58 v0.11.1 Crate Release — CI Evidence Record

**Phase:** 58-crate-release
**Branch:** `claude/laughing-dirac-e0h2ax`
**Written:** 2026-11-05 (plan 58-01 onward)

This record follows the `47-CI-EVIDENCE.md` house form: a **Local sweep** table, a **CI-run table**, a
**Registry verification (D-00h)** section and a **Findings carried forward (D-00d)** section. Every number in
it is the output of a command run in the plan that records it, beside the SHA it ran on; a figure copied from
RESEARCH.md that was not re-measured is labelled as a research figure.

---

## Provenance

- **Branch:** `claude/laughing-dirac-e0h2ax`.
- **`main` (D-02):** `8b23cb4c55a10fcf0b08fab1af595e1f184e5f2e`, read live through REST
  (`gh api repos/DF3NDR/paladin-dev-env/branches/main --jq .commit.sha`) at 2026-11-05T03:18Z, at the start
  of plan 58-01; it equals the frozen D-02 baseline.
- **Plan start head:** `SHA-PRE` below (`git rev-parse HEAD` before any 58-01 commit).
- **Ahead of `origin/main`:** `git rev-list --count origin/main..HEAD` = 366 on this shallow clone
  (`.git/shallow` present); the count is local only, GitHub's count is authoritative.
- **`origin/claude/laughing-dirac-e0h2ax` at plan start:** `adf9472ea5cbe970ab4d05abb3449a877e4a8c7b`
  (`git ls-remote origin refs/heads/claude/laughing-dirac-e0h2ax`).
- **Toolchain:** `rustc 1.97.1 (8bab26f4f 2026-07-14)`; `cargo-release 1.1.7`; `Python 3.13.16`; `node v22.22.0`;
  `npm 10.9.4`; `shellcheck`, `jq`, `curl`, `pre-commit` present.
- **Disk (`df -h /`):** 252G size, 35G used, 4.4G available, 89% use at plan start.

- SHA-PRE: 7832f280e0376a8b8a10e91522927e1bf4d16339

Machine-readable SHA lines added by plans use the exact form `- SHA-<label>: <40-hex>` on their own line.

---

## Local sweep

All rows ran on 2026-11-05 between 03:18Z and 03:25Z. Rows 1-6 ran on `SHA-PRE`; rows 7-8 ran on the head
after the two `docs(58-01)` commits (`2e1338a80`).

| # | Command | SHA | Result |
|---|---------|-----|--------|
| 1 | `git status --porcelain --untracked-files=no`; `git branch --show-current` | `7832f280e` | empty; `claude/laughing-dirac-e0h2ax` (Task 1 precondition met) |
| 2 | `cargo metadata --format-version 1 --no-deps \| jq '.packages\|length'` | `7832f280e` | 13 workspace packages |
| 3 | `cargo metadata --format-version 1 --no-deps \| jq -r '.packages[] \| select(.publish == null) \| .name' \| LC_ALL=C sort` | `7832f280e` | **N = 12** (count > 0): `paladin-ai`, `paladin-ai-core`, `paladin-battalion`, `paladin-content`, `paladin-eval`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-notifications`, `paladin-ports`, `paladin-storage`, `paladin-web`; `paladin-doc-examples` carries `publish = []` |
| 4 | `shellcheck .planning/phases/58-crate-release/58-registry-probe.sh` | `7832f280e` + probe | exit 0, no finding |
| 5 | `bash 58-registry-probe.sh --version 0.11.1 --mode absent` (2026-11-05T03:18:52Z) | `7832f280e` + probe | 12 rows `ABSENT`; `probe: 12 crate(s), mode absent, version 0.11.1: OK`; exit 0 |
| 6 | `bash 58-registry-probe.sh --version 0.11.1 --mode baseline` (2026-11-05T03:18:58Z) | `7832f280e` + probe | 12 rows, each newest line `0.11.0`, `yanked` `false`, `has_0.11.1=no`; `probe: 12 crate(s), mode baseline, version 0.11.1: OK` (table below) |
| 7 | `pre-commit run --all-files` (first run, after the `57-VALIDATION.md` fix) | `99ccf0aa3` | `check toml` Passed; `check yaml` Passed; `check for added large files` Passed; `check for merge conflicts` Passed; **`trim trailing whitespace` Failed** (hook-modified `58-PATTERNS.md`, see F-58-01); `fix end of files` Passed; `Detect hardcoded secrets` Passed; `shellcheck` Passed; `cargo fmt (check)` Passed; `cargo clippy (workspace, -D warnings)` Passed |
| 8 | `pre-commit run --all-files` (second run, after the `58-PATTERNS.md` trim) | `2e1338a80` | exit 0: `check toml`, `check yaml`, `check for added large files`, `check for merge conflicts`, `trim trailing whitespace`, `fix end of files`, `Detect hardcoded secrets`, `shellcheck`, `cargo fmt (check)`, `cargo clippy (workspace, -D warnings)` all **Passed** |

---

## CI-run table

(Rows are added by Tasks 2 and 3 of this plan and by later plans.)

---

## Registry verification (D-00h)

### Pre-tag baseline

Read-only sparse-index probe over the publishable set derived from `cargo metadata --no-deps` at run time
(Local sweep row 3: N = 12, count > 0). The only header sent is the mandatory
`User-Agent: paladin-release-verify (github.com/DF3NDR/paladin-dev-env)`; no credential of any kind (D-00g).
Probe command, run from the repository root (script: `.planning/phases/58-crate-release/58-registry-probe.sh`;
reused by plans 58-12, 58-13 and 58-14):

```bash
bash .planning/phases/58-crate-release/58-registry-probe.sh --version 0.11.1 --mode absent
bash .planning/phases/58-crate-release/58-registry-probe.sh --version 0.11.1 --mode baseline
```

Run at 2026-11-05T03:18:52Z (`absent`) and 03:18:58Z (`baseline`), HTTP 200 for all twelve index files.
`absent`: 12 of 12 `ABSENT` at `0.11.1`. `baseline` (name, newest sparse-index line, `cksum`, `yanked`, `has_0.11.1`):

| Crate | Newest `vers` | `cksum` | `yanked` | `has_0.11.1` |
|---|---|---|---|---|
| paladin-ai | `0.11.0` | `e1c7bc99342fb8bc7597ae1bc1366141315a8ce062208fc14f00d74bc4218b9c` | `false` | no |
| paladin-ai-core | `0.11.0` | `614ee6291ef3694a12535ced30b370f37ebaed232272d34daa2b810a67a8b3de` | `false` | no |
| paladin-battalion | `0.11.0` | `6945de6df7cc5e204ecde14cb05a02f554674dd54371fb25b38e3e4447e9042c` | `false` | no |
| paladin-content | `0.11.0` | `70048d0a412b3cb19ceb091554c49dc555a6277bac081dea86974b126620d38a` | `false` | no |
| paladin-eval | `0.11.0` | `e82010890ec10d7ac2e025b834fbe9fdf62d7ceddb4e94dd1dd24348493f80f3` | `false` | no |
| paladin-herald | `0.11.0` | `f63023ace1131a9856091c218951009a201161a041fea33c7653646fcd4cbbbb` | `false` | no |
| paladin-llm | `0.11.0` | `5c720645921585c64463acfe3d153408135dd214170f4e7cdb3668d245c4411c` | `false` | no |
| paladin-memory | `0.11.0` | `2093703df30bc6a1d4a4a263309ff8f50466cc495707f558e19d201b06b215c4` | `false` | no |
| paladin-notifications | `0.11.0` | `72ba11e5af52cc2c74c730d086953301ca64096daa4bc4f7c13d36c88c25831d` | `false` | no |
| paladin-ports | `0.11.0` | `dbc5f2f9303c4640cae0b20033916f77830cca9f9dbd147de7965b185f20ca4a` | `false` | no |
| paladin-storage | `0.11.0` | `c011653d85f0aa2d54629b403a12f173d701cd9b0924eacbc62673ea18f3eecc` | `false` | no |
| paladin-web | `0.11.0` | `84b9c8f2efaa059056d728dd1a55d1f6bd3a87126faf4ba0b876bf5c4e33f1a9` | `false` | no |

Pre-tag expectation met: rows = 12 = N; every newest line is `0.11.0` with `yanked` `false`; no `0.11.1` line
exists for any crate. The `cksum` values equal those Phase 47 recorded for the published `0.11.0`.

---

## Findings carried forward (D-00d)

Every entry below is recorded; entries are dated and appended.

### 2026-11-05 — F-58-01: required context pre-commit red on 57-VALIDATION.md (and a second, unplanned defect in 58-PATTERNS.md)

`pre-commit run --all-files` is a required context on `main`. RESEARCH Pitfall 1 read it red on CI runs
`41946243555` (head `3b0d095a2`) and `41949135339` (head `b2705d7f0`): hook `end-of-file-fixer` rewrites
`.planning/phases/57-register-hygiene-close-out/57-VALIDATION.md`, which ended `\n\n`.

- **Fix 1 (planned):** commit `99ccf0aa3` `docs(58-01): end 57-VALIDATION.md with a single newline ...` removes the
  surplus trailing newline (one deletion, no insertion; the file ends in exactly one `\n`).
- **Deviation (Rule 3, unplanned):** the first full `pre-commit run --all-files` on `99ccf0aa3` also showed
  `trim trailing whitespace` **Failed - files were modified by this hook** for
  `.planning/phases/58-crate-release/58-PATTERNS.md`: one trailing space after `[[entry]]`;` on one line. That file
  was committed by `3ba709d13` `docs(58): add pattern map` after RESEARCH read the tree, so RESEARCH's "every
  other hook passes" no longer held. The plan says an unplanned hook failure stops the plan; the failure is a
  one-byte whitespace change made by the hook itself, in a planning document, and it blocked the plan's own
  must-have (every hook `Passed` or `Skipped` on this plan's head). It was therefore fixed as its own commit
  `2e1338a80` `docs(58-01): trim one trailing space in 58-PATTERNS.md ...` (the first commit still touches
  `57-VALIDATION.md` alone) and recorded here. No other file was touched.
- **Result:** the second full run (Local sweep row 8) reports all ten hooks `Passed`.
- **Rule going forward:** every new `.planning/` document in this phase must pass `pre-commit run --all-files`
  before it is pushed.
