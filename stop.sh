#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_FILE="$ROOT_DIR/logs/vitrail.pid"

if [ ! -f "$PID_FILE" ]; then
  echo "Vitrail n'est pas lancé (pas de PID file)."
  exit 0
fi

PID="$(cat "$PID_FILE")"
if kill -0 "$PID" 2>/dev/null; then
  kill "$PID"
  echo "Vitrail arrêté (pid $PID)."
else
  echo "Aucun process actif pour le pid $PID (résidu de PID file)."
fi
rm -f "$PID_FILE"
