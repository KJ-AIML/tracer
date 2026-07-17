# WAVE 1.3 Test Matrix

Platform: Windows | Default CI class: standard CI | network: no | credentials: no | live Grok: no

| Requirement | Named test | Layers | Command | CI class | Platform | DB mode | Creds | Net | Result |
|---|---|---|---|---|---|---|---|---|---|
| VS-01 happy path | vs01_successful_run | CP->adapter->fakeACP->SQLite->snapshot | cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1 | standard CI | Windows | memory | no | no | PASS |
| VS-01 file-backed | vs01_file_backed_successful_run | same + file SQLite | same | standard CI | Windows | file temp | no | no | PASS |
| VS-02 auth required | vs02_authentication_required | CP session create | same | standard CI | Windows | memory | no | no | PASS |
| VS-03 auth failure | vs03_authentication_failure_distinct | CP error class | same | standard CI | Windows | memory | no | no | PASS |
| VS-04 capability | vs04_unsupported_capability_controlled | CP+adapter | same | standard CI | Windows | memory | no | no | PASS |
| VS-05 cancel/approval | vs05_cancel_before_approval_no_deadlock | concurrent cancel | same | standard CI | Windows | memory | no | no | PASS |
| VS-05 file-backed | vs05_file_backed_cancel_before_approval_no_deadlock | cancel + file | same | standard CI | Windows | file temp | no | no | PASS |
| VS-06 approval allow | vs06_approval_accepted_once | approval once | same | standard CI | Windows | memory | no | no | PASS |
| VS-07 approval deny | vs07_approval_rejected_once | approval once | same | standard CI | Windows | memory | no | no | PASS |
| VS-08 EOF | vs08_runtime_eof_terminal | terminal honesty | same | standard CI | Windows | memory | no | no | PASS |
| VS-08 file-backed | vs08_file_backed_runtime_eof_terminal | terminal + file | same | standard CI | Windows | file temp | no | no | PASS |
| VS-09 crash | vs09_runtime_crash_distinct | crash class | same | standard CI | Windows | memory | no | no | PASS |
| VS-09 file-backed | vs09_file_backed_runtime_crash_distinct | crash + file | same | standard CI | Windows | file temp | no | no | PASS |
| VS-10 malformed | vs10_malformed_protocol_distinct | protocol error | same | standard CI | Windows | memory | no | no | PASS |
| VS-11 unknown vendor | vs11_unknown_vendor_preserved | opaque payload | same | standard CI | Windows | memory | no | no | PASS |
| VS-12 restart | vs12_restart_restores_history | file reopen | same | standard CI | Windows | file reopen | no | no | PASS |
| VS-13 interrupt | vs13_interrupted_session_recovery | reconcile | same | standard CI | Windows | file reopen | no | no | PASS |
| VS-14 heli missing | vs14_heli_unavailable_runtime_usable | heli + runtime | same | standard CI | Windows | memory | no | no | PASS |
| Reopen/migrations | file_backed_reopen_migrations_and_ordering | open/migrate/reopen | same | standard CI | Windows | file reopen | no | no | PASS |
| Workspace aggregate | cargo test --workspace | all crates | cargo test --workspace | standard CI | Windows | mixed | no | no | PASS |
| Frontend unit | apps/desktop vitest + packages | UI mock | pnpm -r test | standard CI | Windows | n/a | no | no | PASS |
| Fake ACP contract | tests/contract/fake-runtime | node harness | pnpm -r test | standard CI | Windows | n/a | no | no | PASS |
| Live Grok smoke | n/a | live provider | manual | live authenticated smoke | - | - | yes | yes | not run / unproven |
| GUI e2e click path | n/a | full UI | future | future test | - | - | no | no | deferred |