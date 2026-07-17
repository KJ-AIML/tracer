# Module Handoff Report Format

Every Wave 1 module agent should leave a completion report using this structure (path may vary: `docs/modules/w1-x/W1-X_COMPLETION_REPORT.md` or module-specific docs path).

```markdown
# W1-X Completion Report

**Task id:** `tracer-w1-...`
**Work item:** W1-X
**Branch:** `agent/...`
**Base SHA:** <gate/main base>
**Head SHA:** <final local commit>
**Session id:** `heli-ses-...`
**Host:** <grok-build|...>
**Target:** tracer

## Summary

<what shipped in one paragraph>

## Files changed

| Path | Action | Notes |
|---|---|---|
| ... | added/updated | ... |

## Validation

| Command | Result |
|---|---|
| `cargo test --manifest-path ...` | pass / fail |
| `heli session status` | lease active / released |

## Owned path compliance

- Owned only: <list>
- Explicitly not touched: <list>

## Assumptions

- ...

## Risks / follow-ups

- ...

## Integration notes

- Merge order relative to other modules
- Shared manifest requests (if any)
- Contract change requests (link)

## Lease

- Released: yes/no
- Push: **no** (unless user authorized)
```

## Minimum evidence bar

- Commands actually run (not planned)
- Commit SHAs for local commits
- No claim of live host plugin enforcement without SessionStart evidence
