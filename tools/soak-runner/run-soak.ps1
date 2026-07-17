# VS1-H3 soak runner (Windows PowerShell)
# Network: no. Credentials: no. Live Grok: no.
# Time-bounded cargo tests under tests/soak and tests/stress.

$ErrorActionPreference = "Stop"
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $RepoRoot

Write-Host "Repo root: $RepoRoot"
Write-Host "BRIDGE soak suite starting..."

$env:TRACER_SOAK_BURST_COUNT = if ($env:TRACER_SOAK_BURST_COUNT) { $env:TRACER_SOAK_BURST_COUNT } else { "600" }
# Do not set TRACER_SOAK_PERSIST_DELAY_MS globally; soak02 sets it in-process.

Write-Host "Running tests/soak (serial-friendly)..."
cargo test -p tracer-vs1-soak -- --nocapture --test-threads=1
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Running tests/stress..."
cargo test -p tracer-vs1-stress -- --nocapture --test-threads=1
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Soak + stress PASS"
