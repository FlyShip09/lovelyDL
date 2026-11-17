# App icon

Three concepts, drawn on the 1024 px macOS grid (824 px tile, 185 px corner radius):

| | Concept | Verdict |
|---|---|---|
| A | A play triangle turned to point down, resting on a tray line | **Chosen.** Says "media" and "save" in one shape, stays legible at 16 px, and is not a stock download glyph. |
| B | A heart outline with an arrow inside | Rejected: reads as a "like" button, and the outline thins out at small sizes. |
| C | Two stacked video frames with an arrow | Rejected: too many parts for a taskbar icon. |

`icon.svg` is the chosen concept. The app's icons are generated from it:

```bash
npx tauri icon docs/icon/icon.svg -o src-tauri/icons
```
