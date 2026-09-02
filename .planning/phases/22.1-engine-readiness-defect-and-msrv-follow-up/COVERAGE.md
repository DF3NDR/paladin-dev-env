# API Coverage — Phase 22.1

No external API integration: the phase's four items are an in-process scheduler fix
(`Frontier`/`compute_next_vanguard`), a graph-fingerprint hashing change, a workspace MSRV floor
and lockfile change, and a read-only capture of this repository's own CI-run evidence via the `gh`
CLI — no external API, SDK or service capability surface is built, wrapped or consumed as a
deliverable.

The detector fired on a single incidental phrase inside a PLAN threat-model table
("local shell -> GitHub API (`gh`, GH_TOKEN)"), which describes an existing read-only credential
boundary used to *read* workflow-run metadata, not an integration this phase delivers. The
orchestrator's pre-draft run of the same detector over the ROADMAP section and CONTEXT.md returned
`detected: false`.
