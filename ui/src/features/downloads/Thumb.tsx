// The signature element: a row's thumbnail starts desaturated and dimmed, and color fills in
// from the left as the bytes arrive. During post-processing a slow band of light passes over
// it. With Reduce Motion there is no band; the color still follows the progress.

import { AudioWaveform, Film } from "lucide-react";
import { motion, useReducedMotion } from "motion/react";
import { useState } from "react";

import { Icon } from "../../components/Icon";
import type { JobState, MediaKind } from "../../ipc";
import { thumbUrl } from "../../store/derive";
import styles from "./downloads.module.css";

interface Props {
  src: string;
  kind: MediaKind;
  state: JobState;
  fraction: number | null;
  large?: boolean;
}

function revealed(state: JobState, fraction: number | null): number {
  switch (state) {
    case "completed":
    case "postProcessing":
      return 1;
    case "failed":
    case "canceled":
      return 0;
    default:
      return fraction ?? 0;
  }
}

export function Thumb({ src, kind, state, fraction, large }: Props) {
  const url = thumbUrl(src);
  const [loaded, setLoaded] = useState(false);
  const [broken, setBroken] = useState(false);
  const reduced = useReducedMotion();
  const amount = revealed(state, fraction);
  const className = `${styles.thumb} ${large ? styles.thumbLarge : ""}`;

  if (!url || broken) {
    return (
      <span className={`${className} ${styles.thumbEmpty}`} aria-hidden>
        <Icon icon={kind === "audio" ? AudioWaveform : Film} size={large ? 22 : 16} />
      </span>
    );
  }
  return (
    <span className={className} aria-hidden data-state={state}>
      <img
        className={styles.thumbBase}
        src={url}
        alt=""
        decoding="async"
        style={{ opacity: loaded ? 1 : 0 }}
        onLoad={() => setLoaded(true)}
        onError={() => setBroken(true)}
      />
      <img
        className={styles.thumbColor}
        src={url}
        alt=""
        decoding="async"
        style={{ opacity: loaded ? 1 : 0, clipPath: `inset(0 ${((1 - amount) * 100).toFixed(2)}% 0 0)` }}
      />
      {state === "postProcessing" && !reduced && (
        <motion.span
          className={styles.sweep}
          initial={{ x: "-100%" }}
          animate={{ x: "250%" }}
          transition={{ duration: 1.8, ease: "easeInOut", repeat: Infinity, repeatDelay: 0.4 }}
        />
      )}
    </span>
  );
}
