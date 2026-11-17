// The desktop the app is opened from: a wallpaper drawn here, a taskbar, one shortcut.

import React from "react";
import { AbsoluteFill, interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";

import { AppIcon, appFrame, AppWindow, ramp } from "./parts";
import { APP, BLUE, CLAMP, FONT, GLIDE, HANDOFF, OPEN, REVEAL, ROSE, SETTLE, SHORTCUT, WIN } from "./theme";

const GROW = OPEN + 8; // the second click has been let go of

/** An original wallpaper: soft bands of colour, each with a lit crest, drifting very slowly. */
const Wallpaper: React.FC = () => {
  const frame = useCurrentFrame();
  const d = (speed: number, span: number) => Math.sin(frame / speed) * span;
  // Every band is one curve across the screen: filled downward, and drawn again as a thin
  // highlight along the crest, which is what keeps it from reading as a blurred gradient.
  const bands = [
    {
      fill: "band1",
      to: -200,
      curve: `M-260 ${600 + d(190, 14)} C 320 ${386 + d(150, 20)} 700 ${742 + d(170, 16)} 1120 ${548 + d(210, 18)} S 1800 ${300 + d(160, 20)} 2180 ${452}`,
    },
    {
      fill: "band2",
      to: 1280,
      curve: `M-260 ${712 + d(230, 16)} C 360 ${912 + d(180, 20)} 720 ${586 + d(200, 18)} 1140 ${778 + d(170, 18)} S 1800 ${982 + d(220, 16)} 2180 ${842}`,
    },
    {
      fill: "band3",
      to: 1280,
      curve: `M-260 ${992 + d(260, 12)} C 440 ${868 + d(190, 16)} 840 ${1076 + d(210, 14)} 1300 ${944 + d(240, 14)} S 1880 ${852} 2180 ${1012}`,
    },
  ];
  return (
    <svg viewBox="0 0 1920 1080" preserveAspectRatio="none" style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }}>
      <defs>
        <linearGradient id="ground" x1="0" y1="0" x2="0.3" y2="1">
          <stop offset="0" stopColor="#1C2347" />
          <stop offset="0.5" stopColor="#121829" />
          <stop offset="1" stopColor="#080B14" />
        </linearGradient>
        <linearGradient id="band1" x1="0" y1="1" x2="0.9" y2="0">
          <stop offset="0" stopColor={ROSE} stopOpacity="0.7" />
          <stop offset="1" stopColor="#6B3BB8" stopOpacity="0.3" />
        </linearGradient>
        <linearGradient id="band2" x1="1" y1="0" x2="0.1" y2="1">
          <stop offset="0" stopColor={BLUE} stopOpacity="0.68" />
          <stop offset="1" stopColor="#15A5B4" stopOpacity="0.3" />
        </linearGradient>
        <linearGradient id="band3" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#24458F" stopOpacity="0.62" />
          <stop offset="1" stopColor="#8E3A66" stopOpacity="0.4" />
        </linearGradient>
        <filter id="soften" x="-20%" y="-20%" width="140%" height="140%">
          <feGaussianBlur stdDeviation="30" />
        </filter>
        <filter id="crest" x="-10%" y="-10%" width="120%" height="120%">
          <feGaussianBlur stdDeviation="3" />
        </filter>
        <radialGradient id="vignette" cx="0.5" cy="0.46" r="0.8">
          <stop offset="0.4" stopColor="#000" stopOpacity="0" />
          <stop offset="1" stopColor="#000" stopOpacity="0.42" />
        </radialGradient>
      </defs>
      <rect width="1920" height="1080" fill="url(#ground)" />
      <g filter="url(#soften)">
        {bands.map((b) => (
          <path key={b.fill} d={`${b.curve} L2180 ${b.to} L-260 ${b.to} Z`} fill={`url(#${b.fill})`} />
        ))}
      </g>
      <g filter="url(#crest)" fill="none" stroke="#FFFFFF" strokeOpacity="0.17" strokeWidth="2.5">
        {bands.map((b) => (
          <path key={b.fill} d={b.curve} />
        ))}
      </g>
      {/* A vignette keeps the eye on the middle of the screen, where the window opens. */}
      <rect width="1920" height="1080" fill="url(#vignette)" />
    </svg>
  );
};

/** A place on the taskbar: whatever sits there, and the line that says it is running. */
const Slot: React.FC<{ children: React.ReactNode; running?: number }> = ({ children, running = 0 }) => (
  <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 6 }}>
    {children}
    <div style={{ width: 18 * running, height: 3, borderRadius: 2, background: "rgba(255, 255, 255, 0.78)" }} />
  </div>
);

/** A placeholder tile: no one's logo, just a shape in a muted colour. */
const Tile: React.FC<{ mark: React.ReactNode; tint: string }> = ({ mark, tint }) => (
  <div
    style={{
      width: 46,
      height: 46,
      borderRadius: 11,
      background: `linear-gradient(160deg, ${tint}, rgba(255, 255, 255, 0.05))`,
      border: "1px solid rgba(255, 255, 255, 0.1)",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
    }}
  >
    {mark}
  </div>
);

const Taskbar: React.FC = () => {
  const frame = useCurrentFrame();
  const rise = ramp(frame, REVEAL + 4, 20, SETTLE);
  const running = ramp(frame, APP - 14, 14, GLIDE);
  const mark = { fill: "none", stroke: "rgba(255, 255, 255, 0.5)", strokeWidth: 1.8 } as const;
  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        right: 0,
        bottom: 0,
        height: 72,
        background: "rgba(18, 20, 30, 0.72)",
        backdropFilter: "blur(26px)",
        borderTop: "1px solid rgba(255, 255, 255, 0.08)",
        opacity: rise,
        translate: `0px ${interpolate(rise, [0, 1], [72, 0], CLAMP)}px`,
      }}
    >
      <div style={{ position: "absolute", inset: 0, display: "flex", alignItems: "center", justifyContent: "center", gap: 14 }}>
        <Slot><Tile tint="rgba(255, 255, 255, 0.14)" mark={<svg width="22" height="22" {...mark}><rect x="3" y="4" width="16" height="14" rx="2.5" /><path d="M3 9h16" /></svg>} /></Slot>
        <Slot><Tile tint="rgba(120, 160, 230, 0.2)" mark={<svg width="22" height="22" {...mark}><circle cx="11" cy="11" r="7.5" /><path d="M3.5 11h15M11 3.5c3.4 3.8 3.4 11.2 0 15" /></svg>} /></Slot>
        <Slot><Tile tint="rgba(150, 140, 220, 0.2)" mark={<svg width="22" height="22" {...mark}><path d="M4 16.5 9 8l3.4 5 2.3-3 3.3 6.5Z" /></svg>} /></Slot>
        {/* Windows marks a running app with a line under its tile. */}
        <Slot running={running}><AppIcon size={46} /></Slot>
        <Slot><Tile tint="rgba(255, 255, 255, 0.1)" mark={<svg width="22" height="22" {...mark}><path d="M5 17V7M11 17V4M17 17v-7" /></svg>} /></Slot>
        <Slot><Tile tint="rgba(230, 170, 150, 0.18)" mark={<svg width="22" height="22" {...mark}><rect x="3.5" y="5" width="15" height="12" rx="2" /><circle cx="8" cy="10" r="1.8" /><path d="M4 16l5-4 4.5 3.5L17 12" /></svg>} /></Slot>
      </div>
      <div
        style={{
          position: "absolute",
          right: 26,
          top: 0,
          bottom: 0,
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          alignItems: "flex-end",
          gap: 2,
          fontFamily: FONT,
          fontSize: 14,
          color: "rgba(255, 255, 255, 0.72)",
        }}
      >
        <span>10:24</span>
        <span>17/11/2026</span>
      </div>
    </div>
  );
};

const Shortcut: React.FC = () => {
  const frame = useCurrentFrame();
  // The first press selects it; the pair of presses opens it.
  const picked = ramp(frame, OPEN + 2, 3) * (1 - ramp(frame, APP - 6, 10));
  const top = SHORTCUT.y - SHORTCUT.size / 2;
  return (
    <>
      <div
        style={{
          position: "absolute",
          left: SHORTCUT.x - 66,
          top: top - 14,
          width: 132,
          height: SHORTCUT.size + 60,
          borderRadius: 8,
          background: `rgba(140, 190, 255, ${0.16 * picked})`,
          boxShadow: `inset 0 0 0 1px rgba(190, 220, 255, ${0.3 * picked})`,
        }}
      />
      {/* The icon the intro flew here takes over at the handoff; until then it is drawn above. */}
      <AppIcon
        size={SHORTCUT.size}
        style={{
          position: "absolute",
          left: SHORTCUT.x - SHORTCUT.size / 2,
          top,
          opacity: frame >= HANDOFF ? 1 : 0,
          filter: "drop-shadow(0 6px 12px rgba(0, 0, 0, 0.45))",
        }}
      />
      <span
        style={{
          position: "absolute",
          left: SHORTCUT.x - 66,
          top: top + SHORTCUT.size + 10,
          width: 132,
          textAlign: "center",
          fontFamily: FONT,
          fontSize: 21,
          color: "white",
          opacity: ramp(frame, HANDOFF - 4, 10),
          textShadow: "0 1px 4px rgba(0, 0, 0, 0.75)",
        }}
      >
        lovelyDL
      </span>
    </>
  );
};

/** The window grows out of the shortcut: same centre, same size, then up to its own. */
const Opening: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const k = spring({ frame: frame - GROW, fps, config: { damping: 15, mass: 0.95, stiffness: 105 } });
  const scale = interpolate(k, [0, 1], [SHORTCUT.size / WIN.w, 1]);
  const dx = interpolate(k, [0, 1], [SHORTCUT.x - (WIN.x + WIN.w / 2), 0]);
  const dy = interpolate(k, [0, 1], [SHORTCUT.y - (WIN.y + WIN.h / 2), 0]);
  return (
    <div
      style={{
        position: "absolute",
        left: WIN.x,
        top: WIN.y,
        opacity: ramp(frame, GROW, 7),
        transform: `translate(${dx}px, ${dy}px) scale(${scale})`,
        transformOrigin: "center",
      }}
    >
      <AppWindow
        src={appFrame(0)}
        width={WIN.w}
        // Rounded like the icon it came out of, squaring off as it becomes a window.
        style={{ borderRadius: interpolate(k, [0, 1], [200, 10], CLAMP) }}
      />
    </div>
  );
};

export const Desktop: React.FC = () => {
  const frame = useCurrentFrame();
  // The stage does not cut to the desktop, it opens onto it: the picture settles forward
  // out of the dark. A dissolve is read by the eye, not by a curve, so the fade is linear.
  const open = ramp(frame, REVEAL, 18);
  return (
    <AbsoluteFill
      style={{ opacity: open, scale: `${interpolate(ramp(frame, REVEAL, 22, GLIDE), [0, 1], [1.06, 1], CLAMP)}` }}
    >
      <Wallpaper />
      <Shortcut />
      <Taskbar />
      {frame >= GROW && frame < APP && <Opening />}
    </AbsoluteFill>
  );
};
