#!/usr/bin/env bash
# Records a real yt-dlp session as a fixture for the engine's offline tests.
#
#   scripts/capture-fixture.sh <fixture-name> <url> [dl example options…]
#
# Example:
#   scripts/capture-fixture.sh video_audio "https://www.youtube.com/watch?v=SkVqJ1SGeL0" --preset mp4-480
#
# The engine runs fake-ytdlp in record mode in place of yt-dlp. fake-ytdlp runs the real
# yt-dlp from PATH, passes its output through, and saves a scrubbed copy (no signed links,
# no addresses, no user name) under crates/engine/tests/fixtures/sessions/<fixture-name>/.
set -euo pipefail
name="$1"; url="$2"; shift 2
root="$(cd "$(dirname "$0")/.." && pwd)"
real="$(command -v yt-dlp)"
# Two builds: `--example` would otherwise leave the fake-ytdlp binary unbuilt.
cargo build -q -p fake-ytdlp --manifest-path "$root/Cargo.toml"
cargo build -q -p engine --example dl --manifest-path "$root/Cargo.toml"
fake="$root/target/debug/fake-ytdlp"
if [ -f "$fake.exe" ]; then fake="$fake.exe"; fi
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
# Git Bash paths have to be turned into Windows paths before a Windows program sees them.
native() { if command -v cygpath >/dev/null; then cygpath -w "$1"; else printf '%s' "$1"; fi; }
export FAKE_YTDLP_RECORD="$(native "$root/crates/engine/tests/fixtures/sessions")"
export FAKE_YTDLP_FIXTURE="$name"
export FAKE_YTDLP_REAL="$(native "$real")"
export LOVELYDL_YTDLP="$(native "$fake")"
export LOVELYDL_DEV_DIR="$(native "$work")"
"$root/target/debug/examples/dl" "$url" --dir "$work/out" "$@" || true
echo
echo "Saved:"
ls -la "$root/crates/engine/tests/fixtures/sessions/$name"
