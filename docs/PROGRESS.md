# Progress

For each milestone: what is done, how it was verified, what was not verified and why, and
what is next. A fresh session should read this file first, then PLAN.md and DECISIONS.md.

**Target:** Windows app with a macOS-style UI (changed from macOS on 2026-11-17, see
DECISIONS.md). **Dev host:** Windows 11 build 26300, x86_64.

## Toolchain on the dev host

| Tool | Version |
|---|---|
| rustc / cargo | 1.100.0 (MSVC) |
| node / npm | 26.12.1 / 11.20.0 |
| WebView2 runtime | 157.0.4341.46 |
| Visual Studio Build Tools | 2022 and 18 present |
| yt-dlp / ffmpeg / Deno on PATH | 2026.10.26 / 9.0.2 / 2.9.8 |

## M0 · Recon and plan — done

- Repo cloned (it was empty). GPL-3.0 licence text added. Local git identity set to the
  GitHub no-reply address.
- yt-dlp contract checked on the real binary: flags, preset aliases, the `--print`/`--quiet`
  trap, stdout/stderr split, info fields in progress templates, offline preset resolution,
  `-S` ordering. Verified by running real commands; results in DECISIONS.md.
- Tauri 2.12.6 facts checked in the crate source: Windows window effects (Mica, Acrylic via
  the DWM backdrop API on build 22523+), `scrollBarStyle: fluentOverlay`, `shadow` on
  undecorated windows, `set_overlay_icon`, `set_progress_bar`, tauri-specta rc.29 API,
  `tauri-winrt-notification` click callbacks.
- Tool sources checked through the GitHub API: asset names, sizes and SHA-256 digests for
  yt-dlp, Deno and ffmpeg.
- The brief's test video is gone; replacement chosen and probed.
- Not verified: nothing in M0 is left unverified.

## M1 · Engine — done

Built in `crates/engine` (no Tauri): model, state machine, settings, SQLite store with FTS5
history search, toolchain manager, argv builder, output parser, error classifier, process
runner (Job Objects on Windows), probe cache with exact preset resolution, and the engine
actor with the queue scheduler. `crates/fake-ytdlp` replays recorded sessions and records new
ones (`scripts/capture-fixture.sh`).

Verified:

| What | How |
|---|---|
| Real download with live progress | `cargo run -p engine --example dl -- https://www.youtube.com/watch?v=SkVqJ1SGeL0 --preset mp4-480`: 2 streams, merge, metadata, 9.3 MB file. Also `--preset m4a` (2.5 MB, no re-encode). |
| Offline end to end with the real tools | `tests/real_tools.rs`: a local HTTP server, yt-dlp's generic extractor, ffmpeg; skipped when yt-dlp or ffmpeg is not on PATH |
| argv builder | golden tests for the default and an everything-on request, every preset, explicit format picks, bad setting values dropped, URL always after `--` |
| Parser and progress | real progress lines, nulls, NaN/Infinity, fragments, byte-weighted and stage-i-of-n progress, never backwards |
| Error classifier | 28 fixtures: 19 captured from yt-dlp on this PC, 9 messages taken from yt-dlp's source where they cannot be produced on demand (bot check, geo block, members-only, live not started, rate limit, disk full) and marked as such |
| State machine | full 8×8 transition table |
| Store | order survives reopening, FTS5 prefix search over title/uploader/site/URL, diacritics folded, missing files detected, corrupt and newer databases handled |
| Engine (18 tests with fake-ytdlp) | download to file and history; stages video→audio→merge; 5 links at concurrency 2 (peak exactly 2); per-site limit; pause keeps `.part` and resume reruns the identical argv; cancel kills a hung process and deletes its files; crash; 7 error classes; one re-probe on a 403 with cached info; quit and relaunch resumes; queue order survives relaunch; exact preset resolution without a second probe; playlist with archive marking, own folder, index prefixes, one notification; video-or-playlist question; deep links never start downloads; missing tools; Copy Details is redacted |
| Tool installs (6 tests, local HTTP server) | SHA-256 checked from `SHA2-256SUMS` or GitHub's digest, atomic switch, previous kept, rollback, unhealthy binary refused, zip entries that escape the folder skipped, no checksum means no install |
| Lint | `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` clean |

Not verified, and why:

- Managed installs from the real GitHub (only against the local server so far). Exercised in
  M5 with the onboarding flow.
- Cookies from a browser: no supported browser on this PC has a cookie database (yt-dlp says
  so for Chrome, Edge and Firefox; those messages are classifier fixtures).

## Next

M2: Tauri shell, generated IPC types, mock mode, design tokens and base controls.
