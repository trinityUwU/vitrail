#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_DIR="$ROOT_DIR/logs"
PID_FILE="$LOG_DIR/vitrail.pid"

mkdir -p "$LOG_DIR"
: > "$LOG_DIR/vitrail.log"

if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
  echo "Vitrail tourne déjà (pid $(cat "$PID_FILE"))."
  exit 0
fi

if [ ! -d "$ROOT_DIR/src-tauri" ] || [ ! -f "$ROOT_DIR/src-tauri/Cargo.toml" ]; then
  echo "EPIC 0 non fait : pas encore de scaffold Tauri (src-tauri/Cargo.toml absent)."
  echo "Voir docs/EPICS.md — EPIC 0.1."
  exit 1
fi

cd "$ROOT_DIR"
nohup bun run tauri dev >> "$LOG_DIR/vitrail.log" 2>&1 &
echo $! > "$PID_FILE"
echo "Vitrail démarré (pid $(cat "$PID_FILE")). Logs: $LOG_DIR/vitrail.log"
