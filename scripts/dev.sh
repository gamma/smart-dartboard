#!/usr/bin/env bash
set -euo pipefail

SDB_PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SDB_UVICORN="${SDB_PROJECT_DIR}/.venv/bin/uvicorn"

if [[ ! -x "${SDB_UVICORN}" ]]; then
  echo "Fehlende Entwicklungsumgebung: ${SDB_UVICORN}" >&2
  echo "Bitte zuerst den Schnellstart aus README.md ausführen." >&2
  exit 1
fi

cd "${SDB_PROJECT_DIR}"

export SDB_ENABLE_BLE="${SDB_ENABLE_BLE:-0}"
export SDB_ALLOW_TEST_EVENTS="${SDB_ALLOW_TEST_EVENTS:-1}"
export SDB_DEV_RELOAD=1

exec "${SDB_UVICORN}" app:app \
  --host "${SDB_HOST:-0.0.0.0}" \
  --port "${SDB_PORT:-8000}" \
  --reload \
  --reload-dir "${SDB_PROJECT_DIR}" \
  --reload-include "*.py" \
  --reload-include "*.html" \
  --reload-include "*.css" \
  --reload-include "*.js" \
  --reload-include "*.webp" \
  --reload-include "*.png" \
  --reload-include "*.svg"
