---
completed: 2026-11-05
created: 2026-11-03T00:00:00Z
title: Install `cargo-llvm-cov` 0.8.7 in the devcontainer image
area: devcontainer
severity: minor
files:
  - .devcontainer/Dockerfile.dev
  - .devcontainer/README.md
owner: repo maintainer
dispositioned_by: Phase 56 plan 56-09 (DF-20)
---

## Problem

Found by the maintainer's coverage walk (DF-20, C1/C6). `.devcontainer/Dockerfile.dev:112` adds the
`llvm-tools-preview` rustup component, but no line installs `cargo-llvm-cov` itself, and
`post-create.sh` does not either. After a devcontainer rebuild `cargo llvm-cov --version` answers
`error: no such command: llvm-cov`. `.devcontainer/README.md:10` nonetheless lists "llvm-cov" among the
cargo extensions the container provides. A hand install under `/usr/local/cargo/bin` is lost on rebuild
(only `/usr/local/cargo/registry` is a persisted volume in `.devcontainer/docker-compose.yml`).

Plan 56-09 could not make this edit: `.devcontainer/Dockerfile.dev` is outside that plan's
`files_modified`. The testing guide now tells a devcontainer reader to check `cargo llvm-cov --version`
and run the pinned install, which covers the gap until the image is fixed.

## Solution

Add one line next to the other pinned tool installs in `.devcontainer/Dockerfile.dev`, with a comment
saying it mirrors CI's pin (`tool: cargo-llvm-cov@0.8.7` in the `coverage` job of
`.github/workflows/ci.yml`) and drives `make coverage`:

```
RUN cargo install --locked --version 0.8.7 cargo-llvm-cov
```

Keep the pin in step with `ci.yml` and `docs/src/contributing/testing-guide.md`. After the next image
rebuild, `.devcontainer/README.md:10` is true as written; confirm with `cargo llvm-cov --version`.
The image build is the proof, so this needs a rebuild and is not checkable from a sandbox with no
container runtime.

## Resolved 2026-11-05 (Phase 58)

Plan 58-04 added `RUN cargo install --locked --version 0.8.7 cargo-llvm-cov` to `.devcontainer/Dockerfile.dev`, equal to the
`cargo-llvm-cov@0.8.7` pin in `ci.yml` (commit `86213aee2` chore(58-04)). Register row R-59 (`fixed`), closed by plan 58-06. The
image build is the final proof and needs a container runtime; none is available in the sandbox, so the line is checked by
reading the Dockerfile, not by a rebuild.
