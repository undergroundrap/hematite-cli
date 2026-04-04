#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if command -v pwsh >/dev/null 2>&1; then
  exec pwsh -NoProfile -ExecutionPolicy Bypass -File "$ROOT_DIR/clean.ps1" "$@"
fi

if command -v powershell >/dev/null 2>&1; then
  exec powershell -NoProfile -ExecutionPolicy Bypass -File "$ROOT_DIR/clean.ps1" "$@"
fi

deep=0
if [[ "${1:-}" == "--deep" ]]; then
  deep=1
fi

cd "$ROOT_DIR"

rm -rf \
  .hematite/ghost/* \
  .hematite/scratch/* \
  .hematite_logs/* \
  .hematite_scratch/* \
  tmp/* \
  .hematite/reports/* 2>/dev/null || true

rm -f \
  .hematite/ghost/ledger.txt \
  .hematite/session.json \
  .hematite/last_request.json \
  .hematite/vein.db-shm \
  .hematite/vein.db-wal \
  hematite_memory.db-shm \
  hematite_memory.db-wal \
  error.log \
  error_log.txt \
  our_errors.txt \
  error_lines.txt \
  build_errors.txt \
  build_errors.txt.txt \
  build_errors.txt.json \
  errors.txt \
  errors.txt.json \
  errors.json \
  errors.json.txt \
  cargo_errors*.txt 2>/dev/null || true

if [[ "$deep" -eq 1 ]]; then
  rm -rf target onnx_lib 2>/dev/null || true
fi

echo "Hematite cleanup complete."
