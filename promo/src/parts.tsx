// Small pieces shared by the scenes: the icon, the dark stage, the app window, the pointer.

import React from "react";
import { AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame } from "remotion";

import captures from "./captures.json";
import {
  adAt,
  APP,
  BLUE,
  CLAMP,
  FONT,
  GLIDE,
  HEIGHT,
  INK,
  K,
  OPEN,
  POINT,
  ROSE,
  SETTLE,
  SHORTCUT,
  WIDTH,
  WIN,
} from "./theme";

/** The app icon, sized by its rose tile (the artwork has a margin around it). */
export const AppIcon: React.FC<{ size: number; style?: React.CSSProperties }> = ({ size, style }) => {
  const art = (size * 1024) / 824;
  return (
    <div style={{ position: "relative", width: size, height: size, flex: "none", ...style }}>
      <Img
        src={staticFile("captures/icon.svg")}
        style={{ position: "absolute", width: art, height: art, left: (size - art) / 2, top: (size - art) / 2, maxWidth: "none" }}
      />
    </div>
  );
};

/** The dark ground of the type scenes: near-black with a rose and a blue glow that drift. */
export const Stage: React.FC<{ children?: React.ReactNode; style?: React.CSSProperties }> = ({ children, style }) => {
  const frame = useCurrentFrame();
  const drift = Math.sin(frame / 70) * 4;
  return (
    <AbsoluteFill
      style={{
        backgroundColor: INK,
        backgroundImage: [
          `radial-gradient(900px 700px at ${18 + drift}% ${12 - drift}%, ${ROSE}3d, transparent 70%)`,
          `radial-gradient(1000px 800px at ${84 - drift}% ${92 + drift}%, ${BLUE}55, transparent 70%)`,
        ].join(","),
        ...style,
      }}
    >
      {children}
    </AbsoluteFill>
  );
};

const picture = (index: number) => staticFile(`captures/app/u${String(index).padStart(4, "0")}.webp`);
/** The recorded app at a frame of its own clock, held on its first and last pictures. */
export const appFrame = (t: number) =>
  picture(captures.frames[Math.max(0, Math.min(captures.frames.length - 1, Math.round(t)))]);
export const still = (name: keyof typeof captures.stills) => staticFile(captures.stills[name]);

/** A capture inside a window's edge and shadow. `width` is in video pixels. */
export const AppWindow: React.FC<{ src: string; width: number; style?: React.CSSProperties }> = ({
  src,
  width,
  style,
}) => {
  const k = width / captures.width;
  return (
    <div
      style={{
        width,
        height: captures.height * k,
        borderRadius: 8 * k,
        boxShadow: `0 ${28 * k}px ${70 * k}px rgba(4, 8, 24, 0.42), 0 ${4 * k}px ${14 * k}px rgba(4, 8, 24, 0.22), 0 0 0 1px rgba(0, 0, 0, 0.28)`,
        ...style,
      }}
    >
      <Img src={src} style={{ display: "block", width: "100%", height: "100%", borderRadius: 8 * k }} />
    </div>
  );
};

/** 0 → 1 over `[from, from + length]`, clamped: the building block for staggered entrances. */
export const ramp = (frame: number, from: number, length: number, easing?: (t: number) => number) =>
  interpolate(frame, [from, from + length], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing,
  });

// ----- the pointer ------------------------------------------------------------------------
//
// One hand drives the whole ad. It arcs to the desktop shortcut, double-clicks, and from then
// on replays the pointer the recording made: capture.mjs stored where it went and when, in the
// app's own coordinates and frames, so the clicks land on the very pixels the app reacted to.

type Stop = { at: number; travel: number; x: number; y: number };

const where = (e: { x: number; y: number }) => ({ x: WIN.x + e.x * K, y: WIN.y + e.y * K });
const START = { x: WIDTH + 60, y: HEIGHT - 40 }; // just off the bottom-right corner

const TRACK: Stop[] = [
  { at: OPEN, travel: OPEN - POINT, x: SHORTCUT.x, y: SHORTCUT.y + 6 },
  ...captures.events
    .filter((e) => e.type !== "key")
    .map((e) => ({
      at: APP + adAt(e.arrive as number),
      travel: adAt(e.arrive as number) - adAt((e.arrive as number) - (e.travel as number)),
      ...where(e as { x: number; y: number }),
    })),
];

/** Frames the button is held down: the double-click, then every click in the recording. */
const PRESSES: [number, number][] = [
  [OPEN + 2, OPEN + 4],
  [OPEN + 6, OPEN + 8],
  ...captures.events
    .filter((e) => e.type === "click")
    .map((e) => [APP + adAt(e.down as number), APP + adAt(e.up as number)] as [number, number]),
];

export const pointerAt = (frame: number) => {
  let from: { x: number; y: number } = START;
  for (const stop of TRACK) {
    if (frame >= stop.at) {
      from = stop;
      continue;
    }
    const begin = stop.at - stop.travel;
    if (frame <= begin) break;
    const u = GLIDE((frame - begin) / stop.travel);
    // A hand swings; it does not slide along a ruler. The control point sits off to one side
    // of the straight line, so every move leaves on a curve and arrives on one.
    const dx = stop.x - from.x;
    const dy = stop.y - from.y;
    const len = Math.hypot(dx, dy) || 1;
    const bow = Math.min(90, len * 0.17);
    const cx = (from.x + stop.x) / 2 - (dy / len) * bow;
    const cy = (from.y + stop.y) / 2 + (dx / len) * bow;
    const v = 1 - u;
    return { x: v * v * from.x + 2 * v * u * cx + u * u * stop.x, y: v * v * from.y + 2 * v * u * cy + u * u * stop.y };
  }
  return { x: from.x, y: from.y };
};

/** The arrow itself, its tip exactly on the point it is pointing at. */
export const Pointer: React.FC = () => {
  const frame = useCurrentFrame();
  const { x, y } = pointerAt(frame);
  const pressed = PRESSES.some(([down, up]) => frame >= down && frame < up);
  return (
    <svg
      viewBox="-1.2 -1.2 15.7 24.5"
      width={24}
      height={39}
      style={{
        position: "absolute",
        left: x,
        top: y,
        overflow: "visible",
        transform: `scale(${pressed ? 0.86 : 1})`,
        transformOrigin: "0 0",
        filter: "drop-shadow(1px 2px 2.5px rgba(0, 0, 0, 0.45))",
      }}
    >
      <path
        d="M0 0 L0 19.3 L4.6 14.9 L7.6 22.1 L10.6 20.9 L7.7 13.9 L13.3 13.9 Z"
        fill="#FFFFFF"
        stroke="#1A1A1F"
        strokeWidth={1.1}
        strokeLinejoin="round"
      />
    </svg>
  );
};

/** A keystroke, spelled out under the window: the ad is silent and has no hands to show. */
export const Keycap: React.FC<{ label: string; hold: number }> = ({ label, hold }) => {
  const frame = useCurrentFrame();
  const show = ramp(frame, 0, 7, SETTLE) * (1 - ramp(frame, hold, 8));
  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        right: 0,
        // Above the window, beside the toolbar the keys are typed into — the taskbar has the
        // room below.
        top: WIN.y - 76,
        display: "flex",
        justifyContent: "center",
        opacity: show,
        translate: `0px ${interpolate(show, [0, 1], [14, 0], CLAMP)}px`,
      }}
    >
      <div
        style={{
          fontFamily: FONT,
          fontSize: 27,
          fontWeight: 600,
          letterSpacing: "0.01em",
          color: "rgba(255, 255, 255, 0.92)",
          padding: "11px 24px 13px",
          borderRadius: 11,
          background: "rgba(16, 18, 28, 0.74)",
          border: "1px solid rgba(255, 255, 255, 0.22)",
          boxShadow: "0 2px 0 rgba(255, 255, 255, 0.1) inset, 0 10px 26px rgba(0, 0, 0, 0.35)",
        }}
      >
        {label}
      </div>
    </div>
  );
};
