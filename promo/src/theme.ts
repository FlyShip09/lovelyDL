// Shared look and the ad's timeline, in frames at 30 fps.

import { Easing } from "remotion";

import captures from "./captures.json";

export const FPS = 30;
export const WIDTH = 1920;
export const HEIGHT = 1080;

// Segoe UI Variable ships with Windows 11; elsewhere the stack falls back to the system face.
export const FONT = '"Segoe UI Variable Display", "Segoe UI Variable", "Segoe UI", system-ui, sans-serif';
export const ROSE = "#E5405E";
export const ROSE_LIGHT = "#FF7A8A";
export const BLUE = "#0067C0";
export const BLUE_LIGHT = "#6CB8FF";
export const INK = "#0B0C11";
export const PAPER = "#FFFFFF";

// One family of curves for the whole film.
export const CLAMP = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;
export const SETTLE = Easing.spring({ damping: 200 }); // arrives without bounce
export const GLIDE = Easing.bezier(0.45, 0, 0.15, 1); // moves between two rests
export const LEAVE = Easing.bezier(0.5, 0, 0.9, 0.4); // exits quickly

/** Where the app window sits on screen, and the scale from capture pixels to video pixels. */
export const WIN = {
  w: 1300,
  h: Math.round((1300 * captures.height) / captures.width),
  x: Math.round((WIDTH - 1300) / 2),
  y: Math.round((HEIGHT - (1300 * captures.height) / captures.width) / 2),
};
export const K = WIN.w / captures.width;

/** The desktop shortcut: the icon's centre, and how big it is. */
export const SHORTCUT = { x: 186, y: 196, size: 96 };

// The recording (see capture.mjs) runs its first-run pages a little brisker than real time,
// so the ad spends its seconds on the part a viewer came for. Both directions are needed:
// the pictures step forward through `appAt`, the recorded pointer comes back through `adAt`.
const SETUP = captures.marks.main; // app frames up to the main window
const SETUP_SPEED = 1.25;
const SETUP_AD = SETUP / SETUP_SPEED;
export const appAt = (t: number) => (t < SETUP_AD ? t * SETUP_SPEED : SETUP + (t - SETUP_AD));
export const adAt = (a: number) => (a < SETUP ? a / SETUP_SPEED : SETUP_AD + (a - SETUP));

export const APP_FRAMES = Math.ceil(adAt(captures.frames.length));

export const LINE1 = 0; // "Do you want a simple downloader?"
export const LINE2 = 60; // "That supports 1,000+ websites?"
export const LINE3 = 132; // "Here is lovelyDL."
export const REVEAL = 194; // the dark stage opens onto the desktop
export const HANDOFF = REVEAL + 24; // the flying icon lands as the desktop shortcut
export const POINT = HANDOFF + 6; // the pointer starts its arc towards the shortcut
export const OPEN = POINT + 30; // it arrives, and double-clicks
export const APP = OPEN + 40; // the window has finished growing: the recording runs
export const APP_END = APP + APP_FRAMES;
export const FEATURES = APP_END + 2;
export const BEAT = 54;
export const END = FEATURES + 3 * BEAT;
export const TOTAL = END + 96;
