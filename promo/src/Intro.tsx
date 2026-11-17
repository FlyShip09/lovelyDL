// The opening: three lines of kinetic type on the dark stage.

import React from "react";
import { AbsoluteFill, Interactive, interpolate, Sequence, useCurrentFrame } from "remotion";

import { ramp } from "./parts";
import { BLUE_LIGHT, CLAMP, FONT, LEAVE, LINE1, LINE2, LINE3, REVEAL, ROSE, ROSE_LIGHT, SETTLE } from "./theme";

const type: React.CSSProperties = {
  fontFamily: FONT,
  fontSize: 150,
  fontWeight: 600,
  lineHeight: 1.12,
  letterSpacing: "-0.025em",
  color: "white",
  textAlign: "center",
};

/** A word that rises into place `at` frames into its line. */
const Word: React.FC<{ at: number; children: React.ReactNode; style?: React.CSSProperties }> = ({
  at,
  children,
  style,
}) => {
  const frame = useCurrentFrame();
  return (
    <span
      style={{
        display: "inline-block",
        whiteSpace: "pre",
        opacity: ramp(frame, at, 9),
        translate: `0px ${interpolate(frame, [at, at + 18], [70, 0], { ...CLAMP, easing: SETTLE })}px`,
        ...style,
      }}
    >
      {children}
    </span>
  );
};

/** A centered block of type that lifts away over its last frames. */
const Line: React.FC<{ leaveAt: number; children: React.ReactNode }> = ({ leaveAt, children }) => {
  const frame = useCurrentFrame();
  const leave = ramp(frame, leaveAt, 9, LEAVE);
  return (
    <AbsoluteFill
      style={{
        ...type,
        justifyContent: "center",
        alignItems: "center",
        opacity: 1 - leave,
        translate: `0px ${-50 * leave}px`,
      }}
    >
      {children}
    </AbsoluteFill>
  );
};

const rose: React.CSSProperties = {
  backgroundImage: `linear-gradient(180deg, ${ROSE_LIGHT}, ${ROSE})`,
  WebkitBackgroundClip: "text",
  backgroundClip: "text",
  color: "transparent",
};

const Simple: React.FC = () => (
  <Line leaveAt={LINE2 - LINE1 - 8}>
    <Interactive.Div name="Do you want a">
      <Word at={0}>Do </Word>
      <Word at={3}>you </Word>
      <Word at={6}>want </Word>
      <Word at={9}>a</Word>
    </Interactive.Div>
    <Interactive.Div name="simple downloader?">
      {/* The gradient needs room below the baseline, or the "p" loses its tail. */}
      <Word at={13} style={{ ...rose, paddingBottom: 24, marginBottom: -24 }}>
        simple{" "}
      </Word>
      <Word at={17}>downloader?</Word>
    </Interactive.Div>
  </Line>
);

const Count: React.FC = () => {
  const frame = useCurrentFrame();
  const value = Math.round(interpolate(frame, [10, 30], [0, 1000], { ...CLAMP, easing: SETTLE }));
  return (
    // The final number holds the space, so the line never shifts while the digits run.
    <span style={{ position: "relative", display: "inline-block", color: BLUE_LIGHT, fontVariantNumeric: "tabular-nums" }}>
      <span style={{ visibility: "hidden" }}>1,000+</span>
      <span style={{ position: "absolute", right: 0, top: 0, whiteSpace: "pre" }}>
        {value.toLocaleString("en-US")}
        <span style={{ display: "inline-block", opacity: ramp(frame, 28, 5), scale: 0.6 + 0.4 * ramp(frame, 28, 10, SETTLE) }}>
          +
        </span>
      </span>
    </span>
  );
};

const Websites: React.FC = () => (
  <Line leaveAt={LINE3 - LINE2 - 8}>
    <Interactive.Div name="That supports">
      <Word at={0}>That </Word>
      <Word at={3}>supports</Word>
    </Interactive.Div>
    <Interactive.Div name="1,000+ websites?">
      <Word at={7}>
        <Count />{" "}
      </Word>
      <Word at={11}>websites?</Word>
    </Interactive.Div>
  </Line>
);

/** "Here is lovelyDL.", with room above for the icon. */
const HereIs: React.FC = () => (
  <Line leaveAt={REVEAL - LINE3 - 2}>
    <div style={{ height: 250 }} />
    <Interactive.Div name="Here is lovelyDL.">
      <Word at={8} style={{ fontWeight: 400, color: "rgba(255, 255, 255, 0.74)" }}>
        Here is{" "}
      </Word>
      <Word at={13} style={{ fontWeight: 700 }}>
        lovelyDL.
      </Word>
    </Interactive.Div>
  </Line>
);

/** Where the icon rests above "Here is lovelyDL." (the ad draws it, then flies it to the desktop). */
export const HERO = { x: 960, y: 372, size: 190 };

export const Intro: React.FC = () => (
  <>
    <Sequence name="Do you want a simple downloader?" from={LINE1} durationInFrames={LINE2 - LINE1 + 2} layout="none">
      <Simple />
    </Sequence>
    <Sequence name="That supports 1,000+ websites?" from={LINE2} durationInFrames={LINE3 - LINE2 + 2} layout="none">
      <Websites />
    </Sequence>
    <Sequence name="Here is lovelyDL." from={LINE3} durationInFrames={REVEAL - LINE3 + 8} layout="none">
      <HereIs />
    </Sequence>
  </>
);
