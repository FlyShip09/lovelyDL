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

/** The recorded app: how long it runs and what happens when (see capture.mjs). */
export const APP_FRAMES = captures.frames.length;

export const LINE1 = 0; // "Do you want a simple downloader?"
export const LINE2 = 60; // "That supports 1,000+ websites?"
export const LINE3 = 132; // "Here is lovelyDL."
export const REVEAL = 194; // the dark stage opens onto the desktop
export const HANDOFF = REVEAL + 24; // the flying icon lands as the desktop shortcut
export const APP = 256; // first frame of the recorded app
export const APP_END = APP + APP_FRAMES;
export const FEATURES = APP_END + 4;
export const BEAT = 50;
export const END = FEATURES + 3 * BEAT;
export const TOTAL = END + 100;
