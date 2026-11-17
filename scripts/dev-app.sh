#!/usr/bin/env bash
# Starts the app in development mode with WebView2's debugging port open on 9333, so
# scripts/drive-app.mjs and Playwright can inspect it. Stops a previous run first.
#
#   scripts/dev-app.sh            # follows the Windows theme
#   LOVELYDL_THEME=light scripts/dev-app.sh
set -uo pipefail
cd "$(dirname "$0")/.."
taskkill //F //IM lovelydl.exe >/dev/null 2>&1 || true
pid=$(netstat -ano | grep -E ":5173 " | grep LISTENING | awk '{print $5}' | head -1)
if [ -n "$pid" ]; then taskkill //F //PID "$pid" >/dev/null 2>&1 || true; fi
mkdir -p .dev
export WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9333"
exec npx tauri dev > .dev/tauri-dev.log 2>&1
