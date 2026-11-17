# Decisions

Short dated entries: the decision, the alternatives, the reason. Newest last.

## 2026-11-17 · Target changed from macOS to Windows

- **What happened.** The brief describes a macOS app. The session runs on a Windows 11 PC,
  and when that was raised the owner said: build a Windows app with the same UI as macOS,
  smooth and modern.
- **Decision.** lovelyDL v1 is a Windows app (Windows 10 1809+, best on Windows 11) with a
  macOS-style interface. The product scope in the brief stands. The platform layer changes:

| Brief (macOS) | lovelyDL on Windows |
|---|---|
| NSVisualEffect / Liquid Glass sidebar | Acrylic or Mica through Tauri `windowEffects` (real system material) |
| Overlay title bar, native traffic lights | Frameless window, drawn traffic-light controls at the top left |
| Global menu bar, native context menus | In-window macOS-style menus, a toolbar "More" menu, shortcuts with Ctrl |
| SF Symbols | Lucide icons (ISC licence); Apple's symbols may not be shipped off Apple platforms |
| System font SF Pro | Segoe UI Variable, the Windows system font |
| Dock badge and progress | Taskbar progress and an overlay badge |
| Menu bar extra | System tray icon with a menu |
| UserNotifications | Windows toasts with click handling |
| Finder, Trash | Explorer, Recycle Bin |
| Homebrew tools | Tools already on PATH (winget, Scoop, Chocolatey) |
| DMG, notarization, Homebrew cask | NSIS installer, Authenticode signing when a certificate exists, winget manifest |
| ⌘ shortcuts | Ctrl shortcuts |

- **Dropped, with no Windows equivalent.** Quick Look, the Downloads-stack bounce, Finder
  "Where from" metadata, links dropped on the Dock icon, macOS privacy (TCC) prompts, Safari
  cookies.
- **Kept from the brief's design rules.** Translucency only from a real system material and
  never CSS `backdrop-filter`; no gradients, glows or web fonts; the anti-slop and web-tells
  lists; the motion rules; the signature thumbnail reveal.

## 2026-11-17 · Smoke-test video replaced

- yt-dlp's long-standing test video `BaW_jenozKc` now returns "This video is unavailable" on
  every YouTube client (checked with yt-dlp 2026.10.26, `-v`).
- **Replacement.** `https://www.youtube.com/watch?v=SkVqJ1SGeL0`, "Caminandes 3: Llamigos" by
  Blender, 150 s, Creative Commons Attribution. For manual subtitles and chapters:
  `eRsGyueVLvQ` (Sintel, Blender) and `YE7VzlLtp-4` (Big Buck Bunny, Blender).
- The unavailable video's real output is kept as an error fixture.

## 2026-11-17 · Versions pinned

Checked against crates.io and npm on this date.

| Thing | Version | Note |
|---|---|---|
| Rust | 1.100.0, edition 2024 | pinned in `rust-toolchain.toml` |
| Tauri | 2.12.6 (tauri-build 2.7.5, CLI 2.12.5) | 3.0 is alpha only; staying on 2.x as specified |
| tauri-specta | =2.0.0-rc.29 | with specta =2.0.0-rc.28, specta-typescript 0.0.15 |
| TypeScript | 6.0.3 | npm `latest` is 7.0.2, but typescript-eslint 8.73 supports `<6.1.0` |
| React / Vite / Motion | 19.3.1 / 8.3.8 / 14.14.0 | |
| yt-dlp / Deno / ffmpeg | 2026.10.26 / 2.9.8 / 9.0.2 | current releases |

## 2026-11-17 · Typed IPC: tauri-specta 2 over ts-rs

- **Decision.** tauri-specta, pinned with `=`.
- **Why.** It generates the command wrappers and the event as well as the types, so there is
  no hand-written invoke glue to drift. ts-rs (12.0.1, stable) only generates types.
- **Risk.** tauri-specta 2 is still a release candidate. The exact pins and the lockfile
  contain that. If it breaks on a Tauri upgrade, ts-rs plus a small invoke module is the
  fallback.

## 2026-11-17 · yt-dlp facts checked on the real binary

All run with yt-dlp 2026.10.26 on this PC.

- Every flag in §4 of the brief exists in `--help`. Preset aliases are `mp3, aac, mp4, mkv,
  sleep`, and `-t mp4` expands exactly as the brief says.
- `--print` implies `--quiet`: without `--progress` a download printed 0 progress lines, with
  it they came back.
- Download progress lines and the `after_move` path go to **stdout**; post-processing lines go
  to **stderr**. The engine reads both.
- `%(info.format_id)j`, `%(info.vcodec)j` and `%(info.acodec)j` work inside the download
  progress template, so the stage label ("Downloading video", "Downloading audio") comes from
  yt-dlp and not from file names.
- `--load-info-json x.json --simulate --print …` ran in under a second behind a dead proxy
  (`--proxy http://127.0.0.1:9`), so it makes no network calls.
- A second `-S` is **prepended** to the sort order. `-t mp4 -S res:720` gave 720p H.264;
  `-S res:720 -t mp4` gave 1080p. The cap must come after the alias.
- Re-running a finished download exits 0 and prints the `after_move` path again.
- Nightly builds live in `yt-dlp/yt-dlp-nightly-builds` with the same asset names.

## 2026-11-17 · yt-dlp flavour: the unpackaged zip

- **Decision.** Install `yt-dlp_win.zip` (unpackaged, no self-update), not the single-file
  `yt-dlp.exe`.
- **Why.** The app starts yt-dlp for every probe, preset resolution and download, so start-up
  time is user-visible. Five runs of `--version` each on this PC: single-file 802–820 ms,
  unpackaged 276 ms warm (898 ms on the first run).

## 2026-11-17 · Tool downloads: GitHub Releases, verified by SHA-256

- yt-dlp comes from `yt-dlp/yt-dlp` (or `yt-dlp/yt-dlp-nightly-builds`) and is checked against
  the release's `SHA2-256SUMS`. Deno comes from `denoland/deno` and ffmpeg from
  `GyanD/codexffmpeg` (the gyan.dev builds that ffmpeg.org links to); both are checked against
  the `digest` GitHub publishes for each release asset.
- `SHA2-256SUMS.sig` is not verified. A robust OpenPGP check needs a large dependency (rpgp or
  sequoia) for little gain over TLS to the same host. The brief makes it optional.
- ffmpeg uses the `essentials_build.zip` (115 MB). The `.7z` is 35 MB but needs a 7z decoder
  dependency; revisit if first-run time matters more than the dependency.
- ffmpeg already on PATH is used first, so many users download nothing for it.

## 2026-11-17 · TLS: the system stack

- reqwest and the updater use `native-tls` (SChannel on Windows). No C crypto code to build, a
  smaller binary, and the system trust store, which respects a user's own root CAs.

## 2026-11-17 · One jobs table doubles as history

- History is the finished rows of the jobs table, with an FTS5 index over title, uploader,
  site and URL. "Clear Completed" hides a row from Downloads and keeps it in History.
- **Why.** One model and one set of actions instead of two that must be kept in sync.

## 2026-11-17 · Several links at once go straight to the queue

- One pasted link opens the preview (when the setting says so). Several links become queue
  items with the default preset; each is probed in the queue.
- **Why.** The brief says several links create several items; a stack of preview cards would
  ask the user to confirm formats one by one, which is not what a multi-paste is for.
