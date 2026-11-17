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

## 2026-11-17 · yt-dlp needs `--encoding utf-8` on Windows

- Writing to a pipe, yt-dlp on Windows uses the console code page (cp1252 here) and drops
  characters it cannot encode: a title "Élan – ねこ" came out as "Élan – " with the rest lost.
  `PYTHONIOENCODING` and `PYTHONUTF8` had no effect on the frozen executable; `--encoding
  utf-8` fixed it (checked byte by byte). Every invocation passes it.

## 2026-11-17 · The per-site limit counts by the link's host

- The first version counted by the extractor's site name, which only exists after the probe
  and differs from the link's host, so a site could briefly run more jobs than its limit (a
  test caught 3 against a limit of 1). The link's host is known up front and never changes.

## 2026-11-17 · Change batching never drops a state change

- Progress is coalesced to the newest value per job inside each 100 ms window. A job update
  replaces an earlier one only when both carry the same state, so the UI sees every
  transition it animates (Completed, Failed) even when they happen within one window.

## 2026-11-17 · fake-ytdlp is configured by a file next to it

- Tests copy fake-ytdlp to a temp folder as `yt-dlp.exe`, `ffmpeg.exe` and `deno.exe` and
  write `fake-ytdlp.env` beside them. Environment variables would leak between tests that run
  in parallel in one process. Fixtures are picked by the link (`https://fake.test/<name>`).
- The tests ask cargo for the fake's path (`--message-format=json`): cargo 1.100 no longer
  puts test binaries in `target/debug/deps`, so a path computed from the test binary breaks.

## 2026-11-17 · A single link that fails to probe stays in the preview

- With "download straight away" on, one link still goes through the probe first; if the probe
  fails there is no queue row, the preview shows the error, its fix and Copy Details. Several
  links at once go to the queue, where each failure is a row.

## 2026-11-17 · Screenshots and accessibility checks run in Chromium, not WebKit

- The brief asks for Playwright WebKit because the macOS app used WKWebView. On Windows the
  app's web view is WebView2, which is Chromium, so Playwright's Chromium is the faithful
  stand-in. Real-window captures come from the running app (`scripts/capture-window.ps1`),
  and the real web view is driven through its debugging port (`scripts/drive-app.mjs`).

## 2026-11-17 · Icons: Lucide

- SF Symbols may only be used on Apple platforms, so the Windows app uses Lucide (ISC), drawn
  at 1.75–1.9 px strokes to sit with 13 px text. Sidebar icons take the accent color.

## 2026-11-17 · Window chrome drawn by the page

- The window is frameless and transparent. The page draws the traffic lights, the toolbar and
  the sidebar; empty toolbar space drags the window (`data-tauri-drag-region`) and
  double-clicking it zooms. The window stays hidden until the page has painted twice, so
  there is no blank or white frame at launch.
- Menus are in-window (the toolbar's More menu and context menus). Windows has no global menu
  bar, and its native menus would break the macOS look.
- WebView2's browser shortcuts (F5, Ctrl+R, Ctrl+P, Ctrl+F, zoom, F12) are switched off
  through `ICoreWebView2Settings3`, which Tauri 2.12.6 does not expose. Editing shortcuts
  stay. Checked with real key presses: F5 and Ctrl+R no longer reload.

## 2026-11-17 · The Windows "Transparency effects" switch is honored

- WebView2 reports that switch as `prefers-reduced-transparency`. When it is off, the sidebar
  is painted solid instead of showing the material. It is off on the development PC, which is
  why the real-window screenshots show a solid sidebar.

## 2026-11-17 · Row semantics: grid, not listbox

- Rows carry their own buttons (pause, cancel, retry), which an `option` may not contain. The
  Downloads and History lists are ARIA grids with rows and grid cells; the playlist sheet,
  whose rows hold no buttons, stays a listbox with arrow keys and Space.

## 2026-11-17 · Compatible MP4 re-encodes when it has to

- `-t mp4` only remuxes, which fails when the site offers no H.264 (an Internet Archive
  MPEG-2 ad failed with "Conversion failed!"). The preset now adds `--recode-video mp4`;
  yt-dlp then ignores the alias's remux, re-encodes only when the result is not already MP4,
  and leaves H.264 downloads untouched ("already is in target format mp4"). Checked on both.

## 2026-11-17 · The UI's connection survives React's double mount

- In development React mounts twice; the first mount's cleanup ran before its listener
  existed, so every change was applied twice and streamed playlist entries showed 32 of 17.
  `useConnection` now removes a listener that arrives after unmount, and appended entries
  skip ids already present.

## 2026-11-17 · Drag-out is an app command, not the drag plugin

- `tauri-plugin-drag` lets the page start a drag of any path it names. The app calls the
  `drag` crate (same version, 2.1.1) from its own `drag_out` command instead, which takes a
  job id and drags only that job's finished file. The page gets no drag permission.

## 2026-11-17 · Contrast themes

- Under `forced-colors` the system picks every color. Rules there give each control an
  outline, make focus a real outline, and draw selection, progress, toggles and primary
  buttons with the `Highlight` / `HighlightText` pair, which were invisible before. The
  traffic lights keep their colors (`forced-color-adjust: none`), as they identify the
  buttons.

## 2026-11-17 · A queued row is its own drag item

- Wrapping each row in Motion's `Reorder.Item` added an extra focusable element around the
  row's grid semantics (axe flagged it). The row element itself is now the `Reorder.Item`,
  with `layout="position"` so text never stretches while rows move.

## 2026-11-17 · Motion fixes from the contact sheets

- Removed the entrance fades at launch: the first frame is the real UI.
- The probing preview card appears in its final shape, so nothing jumps when details arrive.
- The preview card leaves the layout at once (`popLayout`) while the list glides up.

## 2026-11-17 · Screen-reader names

- A row's accessible name is "title, status", where status is the same short text the row
  shows ("No connection", not yt-dlp's message). Icon buttons are named by their tooltip
  (`title`). Chromium also reports a `title` as UI Automation help text, so Narrator can
  repeat it at high verbosity; a custom tooltip would avoid that but is not worth its cost.

## 2026-11-17 · CI and releases on Windows runners only

- The app is Windows-only, so CI and releases run on `windows-latest`. Releases are drafts:
  publishing stays a manual step for the owner.
- Signing is conditional. Without `WINDOWS_CERTIFICATE` the installer is unsigned and
  SmartScreen warns on first run (README explains the steps). Without
  `TAURI_SIGNING_PRIVATE_KEY` no updater files are made, and a build without
  `LOVELYDL_UPDATER_PUBKEY` reports that updates are not set up.
- winget replaces the Homebrew cask from the macOS plan; a manifest template is in
  `packaging/winget/`.

## 2026-11-17 · No settings import

- The brief says imported settings may never set custom yt-dlp arguments. There is no
  settings import, so nothing can; SECURITY.md does not claim the feature.


## 2026-11-17 · Leaving elements are inert

- Sheets, onboarding steps and the preview card stay on screen while they animate out. They
  are now `inert` from the moment they start leaving (Motion's `useIsPresent`), so a
  double-click cannot press Download twice and Tab cannot reach a step that is going away.
  Found by the live smoke script, which saw two Continue buttons at once.

## 2026-11-17 · Embedded subtitles leave no loose file

- yt-dlp 2026.10.26 keeps a subtitle file after embedding when `--write-subs` was passed
  explicitly (`already_have_subtitle` in `__init__.py`). We pass it so manual and automatic
  captions can both be requested, so embedding also passes `--compat-options no-keep-subs`.
  `--compat-options` adds to earlier values, so a user's own custom compat options still
  apply.

## 2026-11-17 · Rows settle when the pointer rests

- Rows hold their sections while the pointer moves over the list (so nothing jumps under a
  click) and settle after two seconds without movement. Holding until the pointer left kept
  a finished download under Active indefinitely when the pointer simply rested there.

## 2026-11-17 · Size estimates follow what will be written

- Re-encoded MP3 is sized from LAME V0 (~245 kbit/s), not the source stream. A time range
  scales the size by its share of the duration. Compatible MP4 from a non-MP4 source shows
  MP4 and an approximate size. All are marked with "~".

## 2026-11-17 · Correction: Compatible MP4 ranks H.264 above the cap

- The earlier entry put a bare `-S res:H` after `-t mp4`. A later `-S` is prepended, so the
  cap outranked `vcodec:h264` and 1440p/2160p picked VP9 or AV1 (checked: Big Buck Bunny 4K
  gave `315+258 vp9 2160` inside an .mp4). The cap is now `-S vcodec:h264,res:H,acodec:aac`:
  the same video gives H.264 1080p, and the preview says the source has 2160p.

## 2026-11-17 · A taken file name gets a number

- yt-dlp skips a download whose final file exists ("has already been downloaded") and the
  job would complete with the old file: a 480p copy when 1080p was asked for, or a 30-second
  clip when the whole film was. The engine notices that line and downloads again as
  "Title [id] (2).ext", like a browser. Nothing is ever overwritten.

## 2026-11-17 · Fixes from the fresh-eyes review

- Probe output is cached without `cookies` fields, and not at all while browser cookies are
  on (auth headers derived from a session are secrets too).
- Thumbnails use a separate client that resolves only public addresses (`web.rs`), follows
  redirects only to public hosts, and stops reading at 8 MB.
- Every log line passes through redaction (a wrapping writer), not only the fields that were
  cleaned by hand; `--password=x` and `-px` forms are redacted too.
- A dropped file is refused by the page (`dragover`/`drop` prevented, "not allowed" cursor),
  because WebView2 otherwise opens it in place of the app.
- App updates are downloaded and signature-checked before the engine stops; an empty
  `LOVELYDL_UPDATER_PUBKEY` counts as no key.
- Settings are written by one task that always takes the newest copy.
- Sign-in and cookie errors are retryable, "Choose another folder" changes only that
  download, onboarding can be finished without the tools, a rollback holds that version from
  automatic updates, the template example follows the typing, the proxy field refuses what
  yt-dlp could not use, and queued rows move with Alt+↑/↓.
- Release secrets are scoped to the steps that use them, the interface is built in a step
  without them, and third-party actions are pinned to commits.

## 2026-11-17 · Left as they are

- Playlists and several links at once use the default folder and options; only a single
  video's preview has the Options sheet. Folder and options per playlist would need the
  Options sheet inside the playlist sheet.
- "Video or whole playlist?" is asked only for YouTube links carrying both `v` and `list`.
- The download archive is written for playlist items started with "skip already
  downloaded" on, not for single videos.

## 2026-11-17 · Window buttons follow the platform

- The owner asked for the three Mac-style lights only on macOS. On Windows the window now has
  caption buttons at the top right (hairline SVG glyphs rather than the Segoe icon font, so
  they look the same on every Windows version and in CI) and the app's icon and name at the
  top left. The lights component remains and renders only when the user agent is a Mac.
- The page gets no event for maximizing, so the restore glyph shows when the window fills
  the work area.

## 2026-11-17 · More motion, by request

- The first design kept controls still ("no scale"). The owner asked for a smoother, livelier
  feel, so pressable controls now give on press, rows and sidebar items fade their hover
  fill, active progress bars carry a travelling light, and the empty state and first run have
  short entrances. All movement is off under Reduce Motion; durations stay under 300 ms
  except the first-run spring.
- The empty Downloads view is now a centered invitation (picture, keycaps, a Paste link
  button) instead of two lines at the top.

## 2026-11-17 · Waiting downloads are looked at ahead of their turn

- Links added several at a time used to show their raw address until a slot came free. One
  at a time, the engine now fetches the next waiting link's title and thumbnail and caches
  the answer; the download then starts from the cache, so no link is asked for twice (a
  waiting job under examination is skipped by the scheduler until the answer is in). A
  DRM-protected item is refused right there.

## 2026-11-17 · A dropped connection is not the user's problem

- A failure that is plainly passing (a dropped connection, a 429, yt-dlp dying with nothing to
  say) now puts the download back in the queue by itself: three goes, waiting 5 s, then 10 s,
  then 20 s, saying "Connection lost — trying again" meanwhile. A video that is gone, private
  or region-blocked is never retried, because nothing about it will change. Playlist items are
  left to the playlist. The whole thing can be switched off in Settings.
- The wait is a `Config` value so tests do not sit through it. The queue holds such a job back
  until its wait is over, and the look-ahead leaves it alone rather than asking the site again
  at exactly the wrong moment.

## 2026-11-17 · Four pieces at a time

- `--concurrent-fragments` was 1, yt-dlp's own default. Most sites now serve video in small
  fragments, and four at a time is much faster on them while staying polite. It is a setting,
  so anyone on a thin connection can put it back.

## 2026-11-17 · The queue says how it is doing

- Under the sidebar sections there is now one line for the whole queue: how many are running,
  how fast together, and a single bar. It is there only while something runs. The selector is
  shallow-compared, because it builds a fresh object and only its numbers matter (a plain
  selector re-rendered forever).

