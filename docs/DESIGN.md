# lovelyDL design

A Mac-style utility that runs on Windows: quiet, dense and precise. Thumbnails and titles
carry the color; the chrome stays neutral. This file is the source for `ui/src/styles/tokens.css`.

## Principles

1. **The chrome is silent.** Neutral surfaces, system accent only where something is selected
   or is the one primary action. No gradients, glows, colored shadows or decorative type.
2. **One bold thing.** A row's thumbnail fills in with color as the bytes arrive. Nothing else
   competes with it.
3. **Real material or none.** The sidebar shows the Windows system backdrop (Acrylic or Mica)
   through a transparent region. No CSS `backdrop-filter`. Popovers, menus and sheets are opaque.
4. **Motion explains a change.** If an animation does not answer "what changed?", it is cut.
5. **It behaves like a desktop app.** No selectable chrome, no I-beam over labels, arrow
   pointer on buttons, focus rings for the keyboard only, no layout shift while data loads.

## Layout

```
+--------------------------------------------------------------------------+
| ● ● ●  [▯]         [ Paste a link, or drop one anywhere          ] [⋯][⚙] |  toolbar, 52
+----------------+---------------------------------------------------------+
| Downloads    3 | +-----------------------------------------------------+ |
| History        | | [thumb]  Title of the pasted video              [x] | |  preview panel,
| Failed       1 | |          Uploader   12:41   youtube.com             | |  only while a
|                | |  [Compatible MP4 ⌄] [1080p ⌄] [Options]  ~312 MB [Download] |  link is pending
|  (material)    | +-----------------------------------------------------+ |
|                | Active                                                  |
|                | [thumb] Title…                 62%   3.1 MB/s   0:41 ⏸ ✕ |
|                | [thumb] Title…                 Merging video and audio  |
|                | Queued                                                  |
|                | [thumb] Title…                 Waiting                  |
+----------------+---------------------------------------------------------+
   sidebar 208              content pane, opaque, min window 720 × 460
```

- The window is frameless. Traffic-light controls (close, minimize, zoom) sit at the top left
  over the sidebar: 12 px circles, centers 20 px apart, first center 20 px from the left edge,
  on the toolbar's center line. Glyphs appear on hover of the group. Gray when the window is
  not focused.
- The sidebar runs edge to edge, top to bottom, on the system material. A hairline separates
  it from the content pane.
- Empty toolbar space drags the window; double-click zooms it.
- Everything is left-aligned. Numbers that change sit in right-aligned tabular columns.

## Tokens

### Color (light / dark)

| Token | Light | Dark | Use |
|---|---|---|---|
| `--bg-content` | `#FFFFFF` | `#1E1E1E` | Content pane |
| `--bg-window` | `#ECECEC` | `#282828` | Sheets, Settings window, onboarding |
| `--bg-raised` | `#F6F6F6` | `#2C2C2C` | Popovers, menus |
| `--bg-sidebar-solid` | `#F0F0F0` | `#252525` | Sidebar when no material is available |
| `--fill-1` | `rgb(0 0 0 / .05)` | `rgb(255 255 255 / .07)` | Field background, row hover |
| `--fill-2` | `rgb(0 0 0 / .09)` | `rgb(255 255 255 / .12)` | Pressed, tracks, sidebar selection |
| `--control` | `#FFFFFF` | `rgb(255 255 255 / .10)` | Push buttons, pop-up buttons |
| `--label` | `rgb(0 0 0 / .88)` | `rgb(255 255 255 / .88)` | Primary text |
| `--label-2` | `rgb(0 0 0 / .62)` | `rgb(255 255 255 / .64)` | Secondary text |
| `--label-3` | `rgb(0 0 0 / .56)` | `rgb(255 255 255 / .56)` | Placeholders, tertiary text |
| `--glyph-3` | `rgb(0 0 0 / .30)` | `rgb(255 255 255 / .30)` | Decorative and disabled glyphs only |
| `--separator` | `rgb(0 0 0 / .10)` | `rgb(255 255 255 / .10)` | Hairlines between groups |
| `--accent` | system | system | The user's Windows accent; fallback `#007AFF` / `#0A84FF` |
| `--accent-solid` | derived | derived | Accent darkened until white text on it reaches 4.5:1 |
| `--selection-unfocused` | `#DCDCDC` | `#464646` | Selected row when the window is not focused |
| `--green` | `#248A3D` | `#30D158` | Completed |
| `--red` | `#D70015` | `#FF6961` | Failed |
| `--orange` | `#C93400` | `#FF9F0A` | Warnings, paused |

Every text color reaches 4.5:1 on its background. `--glyph-3` is never used for text.

### Type

One family: the system font (`"Segoe UI Variable Text", "Segoe UI", system-ui`). Monospace
(`"Cascadia Mono", Consolas, ui-monospace`) only in the format table and logs. Tabular figures
(`font-variant-numeric: tabular-nums`) on every number that changes.

| Style | Size / line | Weight | Use |
|---|---|---|---|
| Large title | 26 / 32 | 600 | Onboarding welcome |
| Title 2 | 17 / 22 | 600 | Sheet and settings titles |
| Title 3 | 15 / 20 | 600 | Preview title |
| Headline | 13 / 16 | 600 | Group emphasis, primary buttons |
| Body | 13 / 16 | 400 | Default |
| Row title | 13 / 16 | 500 | Download rows |
| Subhead | 11 / 14 | 400 | Secondary lines, section headers (600) |
| Mono | 12 / 16 | 400 | Format table, logs |

### Space and shape

- 4-pt grid: 4, 8, 12, 16, 20, 24.
- Control height 24 (toolbar field 28). Sidebar row 28. Download row 56. Menu item 24.
- Radii: 4 thumbnails and checkboxes; 6 controls, sidebar rows, menu items; 8 toolbar field
  and menus; 10 popovers and the preview panel; 12 sheets. Inner radius = outer − padding.
- Shadow only on floating surfaces (popover, menu, sheet, drag preview):
  `0 0 0 .5px rgb(0 0 0 / .14), 0 8px 30px rgb(0 0 0 / .20)`.
- Separators only between groups, never between rows.
- Focus: 3 px accent ring at 50% opacity, `:focus-visible` only.

### Icons

Lucide, 16 px in the toolbar and sidebar, 14 px in rows, stroke 1.75, rounded caps. Sidebar
icons are tinted with the accent.

## Controls

Each control is purpose-built; there is no component kit.

| Control | Look |
|---|---|
| Push button | 24 high, radius 6, `--control` fill with a hairline; primary = `--accent-solid` with white text. Pressed darkens on pointer-down. No scale, no hover lift. |
| Pop-up button | Push button with a double chevron; opens a menu anchored to it. |
| Segmented control | `--fill-2` track; the selected segment is a raised pill that slides. |
| Toggle | 32 × 18 switch, accent when on. |
| Checkbox | 14 × 14, radius 4, accent fill with a white check. |
| Text field | 24 high, radius 6, hairline; accent ring on keyboard focus. |
| URL field | 28 high, radius 8, `--fill-1`; a 2 px activity line runs along its bottom while probing. |
| Progress bar | 3 px track in `--fill-2`, accent fill, interpolated between updates. |
| Activity indicator | 14 px spinner, shown only while work is happening. |
| Row | 56 high; thumbnail 64 × 36; selection fills the row with `--accent-solid`. |
| Menu | Opaque `--bg-raised`, radius 8, 5 px padding; highlighted item uses `--accent-solid`. |
| Popover | Opaque `--bg-raised`, radius 10; scales and fades from its anchor. |
| Sheet | `--bg-window`, radius 12; slides down from under the toolbar over a light scrim. |
| Banner | Slides down from under the toolbar; one line of text and at most two buttons. |

## The signature element

A row's thumbnail starts desaturated and slightly dimmed. A color copy is clipped from the
left to the real progress (`clip-path: inset(0 X 0 0)`), so the picture fills in with color as
bytes arrive, and it is in full color when the download completes. During post-processing a
slow, soft band of light sweeps across it. The progress bar and the percentage stay, for
clarity and for screen readers. With Reduce Motion there is no sweep and the clip simply
matches the progress. Items without a thumbnail get a neutral tile with a film or waveform
icon. If it reads as a gimmick in screenshots, it gets toned down, not dressed up.

## Motion

Springs for movement, short tweens for opacity and color. Only `transform`, `opacity` and
`clip-path` animate; layout changes go through Motion's layout animations.

| Token | Value | Use |
|---|---|---|
| `snappy` | spring, bounce 0, 0.18 s | Selection indicators, toggles |
| `smooth` | spring, bounce 0, 0.32 s | Layout shifts, rows moving, sidebar collapse |
| `gentle` | spring, bounce 0, 0.42 s | Sheets, preview panel opening |
| `fade` | 150 ms ease-out | Opacity and color |

No spring overshoots: nothing here is thrown by a gesture except drag-reordering, which uses
`smooth` with the drag's velocity.

Choreography:

1. **Link added.** A thin activity line runs in the URL field while yt-dlp probes. The
   preview panel grows out of the field; the thumbnail fades in once decoded.
2. **Choosing a format.** The pop-up's check moves; the size estimate crossfades.
3. **Starting (Enter).** The preview panel condenses into the first queue row (shared
   element). The sidebar's Downloads count ticks up.
4. **Progress.** Continuous bar plus the thumbnail reveal. Stage text crossfades.
5. **Completion.** A checkmark draws in. After a beat the row moves to Completed, but never
   while the pointer is over the list.
6. **Failure.** The status turns red and an inline explanation with one fix button expands.
7. **Remove or Clear.** Rows collapse with a small, capped stagger.
8. **Reorder.** The row lifts with a shadow and springs into place.
9. **Sheets and popovers.** Sheets slide down from under the toolbar; popovers scale and fade
   from their anchor and leave the same way.
10. **Sidebar.** The selection highlight slides between items; collapsing slides the sidebar.
11. **Dragging a link over the window.** The content dims slightly and an inset accent
    outline appears.
12. **Clipboard banner.** Slides down from under the toolbar, and back up.
13. **Onboarding.** Steps slide horizontally. Component installs show real progress.
14. **Launch.** No splash, no staggered entrance; the first paint is the final layout.

Reduce Motion (Windows "Animation effects" off): movement becomes a crossfade or an instant
change; progress stays meaningful. Rows never move under the pointer or during a drag.

## Accessibility

- Reduce Motion: `prefers-reduced-motion`. Transparency off: `prefers-reduced-transparency`
  makes the sidebar opaque. Contrast themes: `forced-colors` keeps system colors and borders.
- Every control and row has an accessible name; rows expose title, state and percent.
- Finished and failed downloads are announced through a polite live region.
- Full keyboard use: Tab between regions, arrows in lists, Enter, Space, Delete.

## Copy

Sentence case, plain verbs, specific words. Buttons say what will happen ("Download",
"Download 24 videos", "Show in Explorer", "Use cookies from Firefox"). Errors say what
happened and offer one fix, without apology or exclamation marks. Empty states invite the
one next action. All strings live in `ui/src/strings.ts`.

## App icon

Three concepts were sketched as SVG in `docs/icon/`; the choice and the reason are recorded
there once the icon is built (M2).
