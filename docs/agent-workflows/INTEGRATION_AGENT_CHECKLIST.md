# Integration Agent Checklist

For Stage / Gate integration owners merging parallel Wave modules.

## Before merge

- [ ] Gate 0 still PASS; readiness matrix module status reviewed
- [ ] Each module has a handoff report with commit SHAs
- [ ] Path ownership: no unapproved cross-module edits
- [ ] `heli conflicts` clean or conflicts explicitly resolved
- [ ] Shared manifest requests collected (root `Cargo.toml`, workspace members, etc.)
- [ ] Contract freeze respected — changes go through CONTRACT_CHANGE_REQUEST.md

## Merge discipline

- [ ] Prefer fast-forward or explicit integration branch
- [ ] Do not rewrite module history without owner agreement
- [ ] Preserve both reports if content conflicts; stop auto-resolution of design disputes
- [ ] Re-run module verification after conflict resolution

## After merge

- [ ] Integration report path updated under `docs/integration/`
- [ ] Traceability matrix rows updated if requirement mapping changed
- [ ] Wave readiness notes for next launch wave
- [ ] No remote publish unless release policy + user authorization allow it

## Conflict triage order (master plan)

1. Stop automatic conflict resolution  
2. Identify ownership  
3. Preserve both task reports  
4. Ask the integration owner to resolve  
5. Rerun module and integration checks  
6. Record the decision  
