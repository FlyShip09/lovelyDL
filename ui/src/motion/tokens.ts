// Motion tokens. Springs for movement, short tweens for opacity and color. Values tuned by eye
// against native sheets and popovers; the reasons are in docs/DESIGN.md.

import type { Transition } from "motion/react";

/** Selection indicators, toggles. */
export const snappy: Transition = { type: "spring", bounce: 0, duration: 0.18 };
/** Rows moving, layout shifts, the sidebar. */
export const smooth: Transition = { type: "spring", bounce: 0, duration: 0.32 };
/** Sheets and the preview card opening. */
export const gentle: Transition = { type: "spring", bounce: 0, duration: 0.42 };
/** Opacity and color. */
export const fade: Transition = { duration: 0.15, ease: "easeOut" };
