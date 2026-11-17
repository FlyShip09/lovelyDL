# Architecture

How lovelyDL fits together, as built. The reasons behind each choice are in
[DECISIONS.md](DECISIONS.md); the look and motion are in [DESIGN.md](DESIGN.md).

```
ui/                 React 19 + TypeScript (Vite)      draws Rust's state; owns no data
  │  typed commands, one typed event (tauri-specta generates ui/src/ipc/bindings.ts)
src-tauri/          Tauri 2.12 shell on WebView2      commands, windows, tray, taskbar, toasts
  │  src/platform/  Windows-specific calls (registry accent, keep-awake, browser keys, toasts)
crates/engine/      pure Rust, no Tauri               queue, yt-dlp driver, tools, SQLite, settings
crates/fake-ytdlp/  test double                       replays recorded yt-dlp sessions
```

## One link, end to end

1. The user pastes a link. The UI calls `add_links(text, source)`.
2. The engine validates it (http or https only) and probes it with
   `yt-dlp -j --flat-playlist --lazy-playlist`. The info JSON is cached for 30 minutes.
3. A `Preview` change reaches the UI, which shows the card. Choosing a preset calls
   `preview_update`; the engine resolves the exact result offline with
   `--load-info-json … --simulate --print` and sends the new size and resolution.
4. Download calls `preview_start`. The job is queued; the scheduler starts it when the
   global and per-site limits allow.
5. `ytdlp::run` spawns yt-dlp with an argv array (never a shell, `--` before the URL) inside
   a Windows Job Object, with its own temp folder on the destination volume.
6. `ytdlp::parse` reads sentinel lines (`@@lovelydl@@`) from both pipes: download progress,
   post-processing steps and the final path. The engine coalesces progress and flushes
   changes at most every 100 ms as one `Update` event.
7. On completion the job's row moves to Completed, the history table gets its search entry,
   and the shell updates the taskbar, tray and notification.

Pause and cancel close the Job Object, which ends yt-dlp and any ffmpeg it started. yt-dlp
resumes the `.part` file on the next start, including after the app quits.

## Engine (`crates/engine`)

One actor task owns all state. Every call (`Engine::add_links`, `job_pause`, …) is a message
to it, so there are no shared locks, and tests drive it deterministically with fake-ytdlp.

| Module | Job |
|---|---|
| `model` | Every type that crosses IPC: `Job`, `Preview`, `Settings`, `Components`, `Change` |
| `state` | The one table of legal job state transitions |
| `engine` | The actor: scheduler, jobs, previews, settings, components, thumbnails |
| `store` | SQLite (bundled rusqlite) on its own thread: jobs, FTS5 history search, migrations |
| `settings` | Versioned JSON, sanitized on load, written atomically by one writer task; a bad file is kept as `.json.bad` |
| `toolchain` | Find, install, verify (SHA-256), update, roll back and health-check yt-dlp, ffmpeg, Deno |
| `ytdlp::args` | Pure argv builder, covered by golden tests |
| `ytdlp::parse` | Sentinel-line parser |
| `ytdlp::probe` | Probe, info-JSON cache, exact preset resolution |
| `ytdlp::errors` | Classifier built from captured yt-dlp output (29 fixtures) |
| `ytdlp::run` | Spawn in a Job Object, read both pipes, stop |
| `redact` | Strips credentials, tokens and the home folder from every log line and Copy details |
| `web` | The client for page-chosen addresses (thumbnails): public internet only, size-capped |

Job states: `Queued → Probing → Downloading ⇄ PostProcessing → Completed`, plus `Paused`,
`Failed` and `Canceled`. One table holds every job; History is the finished ones, and Clear
Completed hides a job from Downloads without removing it from History. Playlist items are
ordinary jobs that share a group.

## Shell (`src-tauri`)

- `commands.rs`: thin wrappers around engine calls, plus window, file and clipboard actions.
  `drag_out` takes a job id and drags only that job's finished file.
- `lib.rs`: start-up (folders, logging, single instance, deep links), the `thumb` protocol
  that serves thumbnails so the page never loads remote images, and window events.
- `shell.rs`: reacts to changes: taskbar progress and badge, toasts, keep-awake, tray.
- `windows.rs`: the frameless, transparent main window and the Settings window; Acrylic when
  the system supports it.
- `tray.rs`, `badge.rs`, `strings.rs` (the shell's own text).

Capabilities (`capabilities/default.json`) give the page only event listening, window
dragging and maximize toggling. Everything else goes through the typed commands.

## UI (`ui/src`)

- `ipc/`: generated bindings, plus `mock.ts`, which plays scripted scenarios when the page
  runs in a browser (development and screenshot tests).
- `store/app.ts`: a Zustand store that mirrors the snapshot and applies each `Change`.
  `derive.ts` has the pure functions rows and labels are built from.
- `features/`: downloads (preview card, rows, options), history, onboarding, settings, and
  shell (toolbar, sidebar, sheets, banners, global keys and drop handling).
- `components/`: the hand-built controls (buttons, pop-ups, segmented control, toggles,
  menus, sheets). There is no component kit.
- `strings.ts`: every user-visible string. `styles/tokens.css`: every color, size and radius.

## IPC

Commands reject with a plain message on failure. One event, `Update(Vec<Change>)`, carries
everything from Rust: `JobAdded`, `JobUpdated`, `JobRemoved`, `Progress`, `Order`, `Preview`,
`PlaylistEntries`, `Settings`, `Components`, `System`, `Notice`, `Command`, `ShowJob`,
`HistoryChanged`. A reloaded page rebuilds itself from `get_snapshot`.

Deep links (`lovelydl://download?url=…&preset=…`) carry a link and a preset name only. They
open a preview; they never start a download or set options.

## Where things are stored

| What | Where |
|---|---|
| Settings, database, download archive | `%APPDATA%\com.flyship09.lovelydl\` |
| Managed tools (previous version kept for rollback) | `%LOCALAPPDATA%\com.flyship09.lovelydl\bin\` |
| Probe cache, thumbnails, partial downloads | `%LOCALAPPDATA%\com.flyship09.lovelydl\cache\` |
| Logs | `%LOCALAPPDATA%\com.flyship09.lovelydl\logs\` |

`LOVELYDL_HOME=<folder>` puts all of these, and the default download folder, under one
folder. Tests and fresh-install runs use it.

## Tests

| Layer | Where | What |
|---|---|---|
| Engine units | `crates/engine/src/**` | argv golden tests, parsers, classifier, state table, settings, redaction |
| Engine integration | `crates/engine/tests/engine.rs` | the actor against fake-ytdlp replaying recorded sessions |
| Tools | `crates/engine/tests/toolchain.rs` | install, verify, update and rollback against a local HTTP server |
| Real tools | `crates/engine/tests/real_tools.rs` | one real download with the installed yt-dlp and ffmpeg (skips if absent) |
| Shell | `src-tauri/tests/shell.rs` | bindings regenerate (CI fails if the committed file differs), badge drawing, platform calls |
| UI units | `ui/src/**/*.test.ts` | store and derive functions (Vitest) |
| Screens | `ui/e2e/screens.spec.ts` | every screen in light, dark and contrast, with axe |
| Interactions | `ui/e2e/interactions.spec.ts` | drag to reorder, keys, sheets, pop-ups, playlist ranges |

The live checks that need the real app (downloads, quit and resume, deep links, the tray)
are listed per milestone in [PROGRESS.md](PROGRESS.md), with the scripts in `scripts/`.
