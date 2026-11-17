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

- The window is frameless, and its buttons follow the platform. On Windows: caption buttons
  (minimize, maximize or restore, close) at the top right, 46 px wide and as tall as the
  toolbar, hairline glyphs, the system's red on close; the app's icon and name sit at the top
  left of the sidebar. On macOS: traffic lights at the top left, 12 px circles with centers
  20 px apart, glyphs on hover. Both dim when the window is not focused.
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
| Push button | 24 high, radius 6, `--control` fill with a hairline; primary = `--accent-solid` with white text. Hover shifts brightness slightly; a press darkens and gives a little (`scale(0.97)`, 140 ms, `--ease-out`). No hover lift. |
| Pop-up button | Push button with a double chevron; opens a menu anchored to it. |
| Segmented control | `--fill-2` track; the selected segment is a raised pill that slides. |
| Toggle | 32 × 18 switch, accent when on. |
| Checkbox | 14 × 14, radius 4, accent fill with a white check. |
| Text field | 24 high, radius 6, hairline; accent ring on keyboard focus. |
| URL field | 28 high, radius 8, `--fill-1`; a 2 px activity line runs along its bottom while probing. |
| Progress bar | 3 px track in `--fill-2`, accent fill, interpolated between updates; a soft light travels along the fill while work is under way. |
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
`clip-path` animate; layout changes go through Motion's layout animations (`layout="position"`,
never size, so text never stretches).

Final values, tuned against contact sheets of recorded flows (`scripts/record-motion.mjs`,
`docs/screenshots/motion/`):

| Token | Value | Use |
|---|---|---|
| `snappy` | spring, bounce 0, 0.18 s | Selection indicators, toggles, the sidebar count |
| `smooth` | spring, bounce 0, 0.32 s | Rows moving, the list closing up, the sidebar sliding |
| `gentle` | spring, bounce 0, 0.42 s | Sheets, the preview card |
| `fade` | 150 ms ease-out | Opacity and color, status text crossfades |

Nothing in everyday use overshoots; the one exception is first run, where the welcome icon
lands with a soft spring. The only gesture-driven motion, dragging a queued row, uses `smooth`.

CSS easing tokens: `--ease-out` `cubic-bezier(0.23, 1, 0.32, 1)` for anything answering the
pointer (presses 140 ms, hovers 120 ms), `--ease-in-out` for movement, `--ease-spring` for the
first-run icon. Rows and sidebar items fade their hover fill; pressable controls scale to
0.97 (icon buttons 0.92) while held.

Choreography as built:

1. **Link added.** A 2 px line runs in the link field while yt-dlp reads it. If that takes
   longer than 300 ms, the preview card appears already in its final shape (a blank thumbnail
   tile and "Getting details…"), so nothing moves when the details arrive. The thumbnail
   fades in once decoded.
2. **Choosing a format.** The pop-up's check moves; the summary and size crossfade.
3. **Return.** The card leaves the layout at once (it fades and drops 24 px) while the list
   glides up into its place; the new row arrives at the top. The sidebar count pulses.
4. **Progress.** The bar glides between updates and never moves back within a stage; speed
   and time left change at most twice a second; the thumbnail fills with color.
5. **Completion.** A check draws in where the row's buttons were. The row stays with the
   active ones for 1.8 s, then joins Completed — but not while the pointer is moving over the
   list (two seconds of stillness lets it settle).
6. **Failure.** The status turns red and the explanation with its fix expands below.
7. **Remove, Clear.** Rows collapse in height and fade, with a stagger capped at six rows.
8. **Reorder.** A queued row lifts with a shadow and the others make way; drop to keep.
9. **Sheets** slide down from under the toolbar over a scrim and back up. **Menus** scale
   from 97 % and fade from their anchor.
10. **Sidebar.** The selection highlight slides; collapsing slides the sidebar out and the
    content follows.
11. **Dragging a link in** dims the content and draws an inset accent outline.
12. **Clipboard and update banners** slide down from under the toolbar.
13. **Onboarding** steps slide sideways; installs show real progress.
14. **Launch.** No splash. Rows and chrome are simply there; only the empty state arrives, in
    three soft steps (picture, title, hint), because it is an invitation rather than content.
15. **First run.** The welcome icon lands with a soft spring and the three tools arrive one
    after another.

Reduce Motion (Windows "Animation effects" off, `prefers-reduced-motion`): Motion's
`reducedMotion="user"` turns movement into instant changes and keeps opacity, the thumbnail
sweep is not shown, and progress keeps working.

## Accessibility

- Reduce Motion: `prefers-reduced-motion`. Transparency off: `prefers-reduced-transparency`
  makes the sidebar opaque. Contrast themes: under `forced-colors` every control gets an
  outline the system colors, focus is a real outline, and selection, progress, toggles and
  primary buttons use the system Highlight pair. The traffic lights keep their colors.
- Every control and row has an accessible name; rows expose title, state and percent.
- Finished and failed downloads are announced through a polite live region.
- Full keyboard use: Tab between regions, arrows in lists, Enter, Space, Delete.

## Copy

Sentence case, plain verbs, specific words. Buttons say what will happen ("Download",
"Download 24 videos", "Show in Explorer", "Use cookies from Firefox"). Errors say what
happened and offer one fix, without apology or exclamation marks. Empty states invite the
one next action. All strings live in `ui/src/strings.ts`.

## App icon

Three concepts are in `docs/icon/` with the reasons for the choice: a play triangle turned
to point down, resting on a tray line, on a rose tile drawn to the macOS icon grid.
