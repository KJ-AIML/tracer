# Runtime Research Task Template

Use for **read-only** investigation of stock Grok / ACP behavior (W0-B style).

## Claim

```bash
# Prefer observe/review when no product writes are needed
npx github:KJ-AIML/heli-harness task claim <research-task-id> --mode observe --host <host>
# If notes must be written into an owned docs path, use write on a dedicated worktree
```

## Rules

- Do **not** fork Grok Build for research convenience  
- Do **not** hardcode machine-local absolute paths in committed docs  
- Label synthetic vs live evidence  
- Prefer scrubbed fixtures under `tests/fixtures/acp/`  
- Live authenticated runs are optional and not Gate 1 CI default  

## Report skeleton

```markdown
# Runtime Research — <topic>

**Task:** ...
**Mode:** observe|write
**Binary provenance:** stock path / version / SOURCE_REV if known

## Questions

1. ...

## Method

- Commands run
- Environment (OS, auth present? yes/no)

## Findings

| Claim | Evidence path | Live/synthetic |
|---|---|---|
| ... | ... | ... |

## Non-claims

- ...

## Implications for Tracer contracts

- Aligns with / tension vs W0-A docs: ...
```
