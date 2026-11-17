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

## Next

M1: the engine crate and fake-ytdlp.
