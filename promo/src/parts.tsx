// Small pieces shared by the scenes: the icon, the dark stage, the app window.

import React from "react";
import { AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame } from "remotion";

import captures from "./captures.json";
import { BLUE, INK, ROSE } from "./theme";

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
