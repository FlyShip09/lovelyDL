// After the demonstration: three short beats, then the card that says where to get it.

import React from "react";
import { AbsoluteFill, interpolate, Interactive, Sequence, useCurrentFrame } from "remotion";

import { AppIcon, AppWindow, ramp, Stage, still } from "./parts";
import { BEAT, BLUE_LIGHT, CLAMP, END, FEATURES, FONT, SETTLE } from "./theme";

/** Everything the ad claims about the app, and the picture that shows it. */
const BEATS = [
  { title: "Whole playlists, your pick", shot: "playlist" },
  { title: "MP4 · MP3 · M4A · FLAC", shot: "formats" },
  { title: "Picks itself up after a dropped connection", shot: "queue" },
] as const;

const Beat: React.FC<{ title: string; shot: "playlist" | "formats" | "queue" }> = ({ title, shot }) => {
  const frame = useCurrentFrame();
  const show = ramp(frame, 0, 11, SETTLE); // the movement, which springs
  const fade = ramp(frame, 0, 12); // the dissolve, which does not
  const go = ramp(frame, BEAT, 12);
  // The picture keeps drifting upward through the beat, so the cut never feels like a slideshow.
  const drift = interpolate(frame, [0, BEAT + 10], [16, -16], CLAMP);
  return (
    <AbsoluteFill style={{ opacity: fade * (1 - go), alignItems: "center" }}>
      <div
        style={{
          fontFamily: FONT,
          fontSize: 78,
          fontWeight: 600,
          letterSpacing: "-0.02em",
          lineHeight: 1.1,
          color: "white",
          textAlign: "center",
          maxWidth: 1420,
          marginTop: 148,
          translate: `0px ${interpolate(show, [0, 1], [34, 0], CLAMP)}px`,
        }}
      >
        {title}
      </div>
      <AppWindow
        src={still(shot)}
        width={980}
        style={{
          position: "absolute",
          top: 352,
          translate: `0px ${drift + interpolate(show, [0, 1], [26, 0], CLAMP)}px`,
          scale: `${interpolate(show, [0, 1], [0.97, 1], CLAMP)}`,
        }}
      />
    </AbsoluteFill>
  );
};

export const Features: React.FC = () => (
  <>
    {BEATS.map((beat, i) => (
      <Sequence key={beat.shot} name={beat.title} from={FEATURES + i * BEAT} durationInFrames={BEAT + 12} layout="none">
        <Beat {...beat} />
      </Sequence>
    ))}
  </>
);

/** A line of the end card, each one arriving a little after the one above it. */
const Rise: React.FC<{ at: number; children: React.ReactNode; style?: React.CSSProperties }> = ({ at, children, style }) => {
  const frame = useCurrentFrame();
  const show = ramp(frame, at, 16, SETTLE);
  return (
    <div
      style={{
        fontFamily: FONT,
        opacity: ramp(frame, at, 9),
        translate: `0px ${interpolate(show, [0, 1], [26, 0], CLAMP)}px`,
        ...style,
      }}
    >
      {children}
    </div>
  );
};

export const EndCard: React.FC = () => {
  const frame = useCurrentFrame();
  const show = ramp(frame, 0, 16);
  return (
    <AbsoluteFill
      style={{ alignItems: "center", justifyContent: "center", gap: 0, opacity: show, textAlign: "center" }}
    >
      <Rise at={0}>
        <AppIcon size={186} style={{ filter: "drop-shadow(0 18px 40px rgba(229, 64, 94, 0.3))" }} />
      </Rise>
      <Interactive.Div name="lovelyDL">
        <Rise at={5} style={{ fontSize: 112, fontWeight: 700, letterSpacing: "-0.03em", color: "white", marginTop: 34 }}>
          lovelyDL
        </Rise>
      </Interactive.Div>
      <Rise at={12} style={{ fontSize: 46, color: "rgba(255, 255, 255, 0.76)", marginTop: 14 }}>
        Free and open source
      </Rise>
      <Rise at={19} style={{ fontSize: 40, color: BLUE_LIGHT, marginTop: 30, letterSpacing: "0.005em" }}>
        github.com/FlyShip09/lovelyDL
      </Rise>
      <Rise at={28} style={{ fontSize: 26, color: "rgba(255, 255, 255, 0.46)", marginTop: 56 }}>
        Download only what you have the right to save.
      </Rise>
    </AbsoluteFill>
  );
};

export const Outro: React.FC = () => (
  <>
    <Features />
    <Sequence name="End card" from={END} layout="none">
      <EndCard />
    </Sequence>
  </>
);

/** The repository's social preview: the same card, lying down, with the app beside it. */
export const Social: React.FC = () => (
  <Stage>
    <div style={{ position: "absolute", left: 86, top: 0, bottom: 0, width: 600, display: "flex", flexDirection: "column", justifyContent: "center", fontFamily: FONT }}>
      <AppIcon size={118} style={{ filter: "drop-shadow(0 12px 26px rgba(229, 64, 94, 0.32))" }} />
      <div style={{ fontSize: 86, fontWeight: 700, letterSpacing: "-0.03em", color: "white", marginTop: 22 }}>lovelyDL</div>
      <div style={{ fontSize: 31, lineHeight: 1.35, color: "rgba(255, 255, 255, 0.78)", marginTop: 16, maxWidth: 520 }}>
        A calm video and audio downloader for Windows
      </div>
      <div style={{ fontSize: 23, color: BLUE_LIGHT, marginTop: 26 }}>Free and open source</div>
    </div>
    {/* The app leans in from the right, cropped by the edge: a glimpse, not a screenshot. */}
    <AppWindow src={still("formats")} width={820} style={{ position: "absolute", left: 742, top: 108 }} />
  </Stage>
);
