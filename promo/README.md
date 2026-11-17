# The lovelyDL ad

A silent product film, about half a minute long, built with [Remotion](https://remotion.dev):
three lines of type, a desktop, the real app doing a real first run and a real download, three
short beats, and a card saying where to get it.

Nothing in it is a drawing of the app. `capture.mjs` drives the actual interface in a browser
and records it frame by frame; the video plays those frames back. When the UI changes, capture
again and the ad is up to date.

What is drawn here: the wallpaper, the taskbar, the pointer and the type. The icon comes from
`docs/icon/icon.svg`. No one else's logos, wallpapers, footage or fonts.

## Capture the interface

The recording needs the UI dev server from the repository root, on a port of its own:

```console
npm ci                                 # repository root
npx vite --port 5199 --strictPort      # leave running
```

Then, from the repository root (`node` and `ffmpeg` on PATH):

```console
cd promo && npm ci && cd ..
node promo/capture.mjs
```

It writes `promo/public/captures/` (lossless WebP frames and PNG stills, committed, around
7 MB) and `promo/src/captures.json` (which picture shows when, and where the pointer went). The
raw PNGs land in `promo/captures-raw/`, which is git-ignored. Set `LOVELYDL_UI` if the server
runs somewhere else, for instance `LOVELYDL_UI=http://localhost:5201 node promo/capture.mjs`.

The script drives the page on a paused clock, one video frame at a time, and steps the app's
own springs and transitions by hand, so the recording is exact and repeats identically. It
checks itself at the end: no page errors, three finished downloads, every event inside the
recording, and few enough distinct pictures to commit.

## Preview

```console
cd promo
npm run dev        # Remotion Studio; the compositions are Ad and Social
```

## Render

From `promo/`:

```console
npx remotion render Ad ../docs/promo/lovelydl-ad.mp4 --codec=h264 --crf=17 --jpeg-quality=100
npx remotion still Social ../docs/promo/social-preview.png
```

The README's GIF is cut from the finished film — the desktop, the setup, the download — and
held for a moment on the last frame:

```console
cd docs/promo
ffmpeg -y -i lovelydl-ad.mp4 -filter_complex \
  "[0:v]select='between(n,240,324)+between(n,378,447)+between(n,515,714)',\
setpts=N/FRAME_RATE/TB,fps=12,scale=800:-2:flags=lanczos,\
tpad=stop_mode=clone:stop_duration=1.4,split[a][b];\
[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle" \
  -loop 0 lovelydl-ad.gif
```

Those frame numbers follow the timeline in `src/theme.ts`; if the recording gets longer or
shorter they move with it.

## How it is put together

`src/theme.ts` holds the whole timeline in frames, the palette and the one family of easing
curves everything uses. Every other file reads its timings from there.

| File | What it is |
| --- | --- |
| `src/Composition.tsx` | The film, start to finish, and the two compositions |
| `src/Intro.tsx` | The three opening lines |
| `src/Desktop.tsx` | The wallpaper, the taskbar, the shortcut, the window growing out of it |
| `src/Outro.tsx` | The three beats, the end card, the social preview |
| `src/parts.tsx` | The icon, the dark stage, the window frame, the pointer, the keycaps |

The pointer is worth a word. It is one hand for the whole film: it arcs in off the corner,
double-clicks the shortcut, and from there replays the pointer the recording itself made, in
the app's own coordinates. That is why its clicks land on the very pixels the app reacted to.
