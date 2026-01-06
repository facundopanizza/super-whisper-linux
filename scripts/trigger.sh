#!/usr/bin/env bash
# SuperWhisper Linux trigger script for Hyprland
# Usage: trigger.sh [command]
# Commands: toggle (default), start, stop, cancel, status

SOCKET="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/super-whisper.sock"
COMMAND="${1:-toggle}"

if [[ ! -S "$SOCKET" ]]; then
    notify-send "SuperWhisper" "App not running" -u critical 2>/dev/null
    exit 1
fi

echo "$COMMAND" | socat - UNIX-CONNECT:"$SOCKET" 2>/dev/null
