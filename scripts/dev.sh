#!/usr/bin/env bash
set -euo pipefail

SDB_PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SDB_UVICORN="${SDB_PROJECT_DIR}/.venv/bin/uvicorn"

SDB_BLE_MODE=""

show_help() {
  cat <<'EOF'
Verwendung: ./scripts/dev.sh [OPTION]

Optionen:
  --ble       Echtes BLE-Board verwenden (macOS nativ oder Linux)
  --no-ble    BLE deaktivieren und Testtreffer erlauben (Standard)
  -h, --help  Diese Hilfe anzeigen

Optionale Umgebung:
  SDB_DEVICE_NAME       BLE-Gerätename, Standard: SDB-BT
  SDB_DEVICE_ADDRESS    Feste Adresse; unter macOS eine CoreBluetooth-UUID
  SDB_ALLOW_TEST_EVENTS Mit --ble auf 1 setzen, um zusätzlich Testtreffer zu erlauben
  SDB_HOST, SDB_PORT    Entwicklungsserver-Adresse und -Port
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --ble)
      SDB_BLE_MODE="1"
      ;;
    --no-ble)
      SDB_BLE_MODE="0"
      ;;
    -h|--help)
      show_help
      exit 0
      ;;
    *)
      echo "Unbekannte Option: $1" >&2
      show_help >&2
      exit 2
      ;;
  esac
  shift
done

if [[ ! -x "${SDB_UVICORN}" ]]; then
  echo "Fehlende Entwicklungsumgebung: ${SDB_UVICORN}" >&2
  echo "Bitte zuerst den Schnellstart aus README.md ausführen." >&2
  exit 1
fi

cd "${SDB_PROJECT_DIR}"

if [[ -n "${SDB_BLE_MODE}" ]]; then
  export SDB_ENABLE_BLE="${SDB_BLE_MODE}"
else
  export SDB_ENABLE_BLE="${SDB_ENABLE_BLE:-0}"
fi

case "${SDB_ENABLE_BLE}" in
  1|true|TRUE|True)
    export SDB_ALLOW_TEST_EVENTS="${SDB_ALLOW_TEST_EVENTS:-0}"
    echo "Dev-Modus: BLE aktiv (${SDB_DEVICE_NAME:-SDB-BT}), Testtreffer ${SDB_ALLOW_TEST_EVENTS}."
    ;;
  *)
    export SDB_ALLOW_TEST_EVENTS=1
    echo "Dev-Modus: BLE deaktiviert, Testtreffer ${SDB_ALLOW_TEST_EVENTS}."
    ;;
esac
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
