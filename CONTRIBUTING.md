# Contributing

Thanks for helping. Bug reports with the row's **Copy details** text are the most useful
thing you can send; it has the versions, the command and the log with personal data removed.

## Setup

Windows 10 or 11, [Rust](https://rustup.rs) (the pinned version in `rust-toolchain.toml`
installs itself), Node.js 22 or later.

```bash
npm ci
```

```bash
npx tauri dev
```

The interface also runs in a plain browser with mock data, which is the fastest way to work
on screens: `npm run dev`, then open `http://localhost:5173/?scenario=busy`. The scenarios
are listed at the top of `ui/src/ipc/mock.ts`.

`LOVELYDL_HOME=<folder>` makes the app keep its settings, tools, cache and default download
folder under that folder, so test runs never touch your real ones.

## Before you open a pull request

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

```bash
npm run typecheck && npm run lint && npm test && npm run e2e
```

`npm run e2e` rewrites the screenshots in `docs/screenshots/mock`; look at the ones your
change touches, in light, dark and contrast. If you change a Rust type that crosses IPC,
`cargo test` regenerates `ui/src/ipc/bindings.ts`; commit it.

## House rules

- Rust owns state; the UI draws it. New behavior goes in `crates/engine` with a test
  (fake-ytdlp can replay a recorded session; see `scripts/capture-fixture.sh`).
- No `unwrap` or `expect` outside tests. Errors use `thiserror` in the engine.
- yt-dlp runs with an argument array, never a shell, and `--` before the URL.
- Every user-visible string goes in `ui/src/strings.ts`, written as DESIGN.md's Copy section
  describes: sentence case, plain verbs, no apologies or exclamation marks.
- Colors, sizes and radii come from `ui/src/styles/tokens.css`. Only `transform`, `opacity`
  and `clip-path` animate.
- New dependencies need a reason in `docs/DECISIONS.md`, with exact versions pinned.
- Nothing that phones home: no telemetry, analytics or remote configuration.

## Scope

lovelyDL does not and will not circumvent DRM or paywalls. Pull requests that do are
closed.

By contributing you agree that your work is licensed under the GPL-3.0-or-later, like the
rest of the project.
