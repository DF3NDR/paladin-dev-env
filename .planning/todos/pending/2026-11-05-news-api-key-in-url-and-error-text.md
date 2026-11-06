---
created: 2026-11-05T00:00:00Z
title: "News API key travels in the URL and reqwest error text can echo it (D-52-03-01)"
area: content-security
severity: major
files:
  - crates/paladin-content/src/adapters/input/news_api_fetcher.rs
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-09)
---

## Problem

News API key travels in the URL and reqwest error text can echo it (D-52-03-01).

Carried into the v0.11.1 pre-tag audit register as R-09; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read crates/paladin-content/src/adapters/input/news_api_fetcher.rs on 9eba21d9b: :66 builds the URL with &apiKey={}, :54 builds the client with reqwest::blocking::Client::new() (follows redirects), :299 a test pins apiKey= in the URL

## What to do

security hardening: key in an X-Api-Key header, URL stripped from the mapped error, redirect::Policy::none().

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 57-REGISTER-EVIDENCE.md Orphaned deferrals; 52 deferred-items.md D-52-03-01; 55 deferred-items.md Deferred by CONTEXT; STATE Blockers/Concerns
- Register row: R-09 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
