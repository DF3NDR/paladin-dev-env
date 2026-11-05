# Fixture register

```
| 10 | WR-77 | row inside a fence, ignored | closed | tbd | |
```

## Dispositions

| Phase | Id | Title | Disposition | Evidence | Owner (if waived) |
|-------|----|-------|-------------|----------|-------------------|
| 10 | WR-01 | first warning | closed | `first_warning_stays_fixed_test` (re-run: ok) | |
| 10 | CR-01 | the critical | waived | Accepted until the next minor; documented in docs/limits.md | owner-a |
| 40 | WR-01 | tenant warning | closed | commit 1a2b3c4d5e | |
| 40 | IN-01 | info one | closed | TENANT-04 | |
| 40 | IN-04 | info four | closed | crates/x/src/lib.rs | |

**Totals:** closed: 4, waived: 1, open: 0

```
never closed
