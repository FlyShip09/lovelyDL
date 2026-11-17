# Security

## Reporting a problem

Please report security problems privately through GitHub's
[private vulnerability reporting](https://github.com/FlyShip09/lovelyDL/security/advisories/new),
not in a public issue. Include what an attacker could do, how to reproduce it, and the
lovelyDL version. You should hear back within a week.

Problems in yt-dlp, FFmpeg or Deno themselves belong with those projects; lovelyDL updates to
their fixed releases.

## Supported versions

Only the latest release gets fixes. The app updates itself when a signed release is
available.

## What the app guarantees

- yt-dlp is started with an argument array, never through a shell, and with `--` before the
  link, so a link cannot become an option. Only `http` and `https` links are accepted.
- `lovelydl://` links can carry a link and a preset name only. They open a preview and never
  start a download, set options or folders, or pass custom yt-dlp arguments.
- Custom yt-dlp arguments need an explicit opt-in in Settings › Advanced.
- Tools are downloaded from their official GitHub releases over HTTPS and checked against
  the published SHA-256 sums before use. App updates are verified against the release
  signing key built into the app.
- Browser cookies are read by yt-dlp at download time. lovelyDL never copies, stores or logs
  them: `cookies` fields are stripped from yt-dlp's output before anything is cached, and
  while browser cookies are in use nothing from a probe is cached at all. Every log line and
  Copy details redact credentials, tokens and the home folder.
- Thumbnail addresses come from the pages themselves, so they may only reach the public
  internet (never this PC or the local network, also not through DNS or a redirect), and are
  read with an 8 MB cap.
- The web view has a strict Content Security Policy and may call only the app's own
  commands; it never loads remote images or scripts.
- No telemetry, analytics, accounts or remote configuration.
