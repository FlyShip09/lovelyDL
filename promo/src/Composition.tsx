// The ad itself: three lines of type, a desktop, the real app, three beats, a card.

import React from "react";
import { AbsoluteFill, Composition, interpolate, Sequence, useCurrentFrame } from "remotion";

import captures from "./captures.json";
import { Desktop } from "./Desktop";
import { HERO, Intro } from "./Intro";
import { Outro, Social } from "./Outro";
import { AppIcon, appFrame, AppWindow, Keycap, Pointer, ramp, Stage } from "./parts";
import {
  adAt,
  APP,
  APP_END,
  appAt,
  CLAMP,
  FPS,
  GLIDE,
  HANDOFF,
  HEIGHT,
  INK,
  LINE3,
  POINT,
  REVEAL,
  SETTLE,
  SHORTCUT,
  TOTAL,
  WIDTH,
  WIN,
} from "./theme";

/** The icon the third line introduces, which then flies out and lands as the shortcut. */
const Hero: React.FC = () => {
  const frame = useCurrentFrame();
  const show = ramp(frame, LINE3 + 6, 20, SETTLE);
  const land = ramp(frame, REVEAL, HANDOFF - REVEAL, GLIDE);
  const size = interpolate(land, [0, 1], [HERO.size, SHORTCUT.size], CLAMP);
  const x = interpolate(land, [0, 1], [HERO.x, SHORTCUT.x], CLAMP);
  // It is thrown rather than dragged: the path bows upward and settles onto the desktop.
  const y = interpolate(land, [0, 1], [HERO.y, SHORTCUT.y], CLAMP) - Math.sin(Math.PI * land) * 64;
  return (
    <AppIcon
      size={size}
      style={{
        position: "absolute",
        left: x - size / 2,
        top: y - size / 2,
        opacity: ramp(frame, LINE3 + 6, 10),
        scale: `${interpolate(show, [0, 1], [0.62, 1], CLAMP)}`,
        filter: "drop-shadow(0 16px 36px rgba(229, 64, 94, 0.34))",
      }}
    />
  );
};

/** The keystrokes the recording made, spelled out one at a time under the window. */
const keys = captures.events.filter((e) => e.type === "key");
const Keystrokes: React.FC = () => (
  <>
    {keys.map((e, i) => {
      const at = Math.round(adAt(e.frame as number));
      const next = keys[i + 1];
      // A keycap leaves in time for the next one; they share one place under the window.
      const hold = next ? Math.min(22, Math.round(adAt(next.frame as number)) - at - 7) : 22;
      return (
        <Sequence key={at} name={e.label} from={APP + at} durationInFrames={hold + 10} layout="none">
          <Keycap label={e.label as string} hold={hold} />
        </Sequence>
      );
    })}
  </>
);

export const Ad: React.FC = () => {
  const frame = useCurrentFrame();
  const leave = ramp(frame, APP_END - 16, 16); // the screen dissolves into the beats
  return (
    <AbsoluteFill style={{ backgroundColor: INK }}>
      <Stage />
      {frame < REVEAL + 10 && <Intro />}
      {frame >= REVEAL - 1 && frame <= APP_END && (
        <AbsoluteFill style={{ opacity: 1 - leave }}>
          <Desktop />
          {frame >= APP && (
            <AppWindow
              src={appFrame(appAt(frame - APP))}
              width={WIN.w}
              style={{ position: "absolute", left: WIN.x, top: WIN.y }}
            />
          )}
          <Keystrokes />
          {frame >= POINT - 2 && <Pointer />}
        </AbsoluteFill>
      )}
      {/* Above the desktop: it is still in the air until it lands as the shortcut. */}
      {frame < HANDOFF && <Hero />}
      <Outro />
    </AbsoluteFill>
  );
};

export const Ads: React.FC = () => (
  <>
    <Composition id="Ad" component={Ad} durationInFrames={TOTAL} fps={FPS} width={WIDTH} height={HEIGHT} />
    {/* The repository's social preview, rendered as a still. */}
    <Composition id="Social" component={Social} durationInFrames={1} fps={FPS} width={1280} height={640} />
  </>
);
