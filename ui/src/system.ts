// System look: the user's accent color, whether a real material is behind the window, and
// whether the window is focused (selections turn gray when it is not, like on a Mac).

import type { SystemInfo } from "./ipc";

function parseHex(hex: string): [number, number, number] | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m?.[1]) return null;
  const n = parseInt(m[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function luminance([r, g, b]: [number, number, number]): number {
  const lin = (c: number) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

export function contrast(a: [number, number, number], b: [number, number, number]): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

const white: [number, number, number] = [255, 255, 255];

/** The accent darkened just enough for white text on it to reach 4.5:1. */
export function solidAccent(rgb: [number, number, number]): [number, number, number] {
  let color = rgb;
  for (let step = 0; step < 40 && contrast(color, white) < 4.5; step++) {
    color = [color[0] * 0.95, color[1] * 0.95, color[2] * 0.95].map(Math.round) as [number, number, number];
  }
  return color;
}

const css = ([r, g, b]: [number, number, number], alpha = 1) =>
  alpha === 1 ? `rgb(${r} ${g} ${b})` : `rgb(${r} ${g} ${b} / ${alpha})`;

/** Moves a color toward black or white until it reaches 4.6:1 against a background. */
export function readableOn(
  rgb: [number, number, number],
  background: [number, number, number],
): [number, number, number] {
  const lighten = luminance(background) < 0.2;
  let color = rgb;
  for (let step = 0; step < 60 && contrast(color, background) < 4.6; step++) {
    color = color.map((c) => Math.round(lighten ? c + (255 - c) * 0.06 : c * 0.95)) as [
      number,
      number,
      number,
    ];
  }
  return color;
}

export function applySystem(system: SystemInfo) {
  const root = document.documentElement;
  root.dataset.material = String(system.material);
  const rgb = parseHex(system.accent);
  if (!rgb) return;
  root.style.setProperty("--accent", css(rgb));
  root.style.setProperty("--accent-solid", css(solidAccent(rgb)));
  root.style.setProperty("--accent-tint", css(rgb, 0.14));
  root.style.setProperty("--focus-ring", css(rgb, 0.5));
  // Checked against the darkest light background and the lightest dark one.
  root.style.setProperty("--accent-text-light", css(readableOn(rgb, [0xe4, 0xe4, 0xe4])));
  root.style.setProperty("--accent-text-dark", css(readableOn(rgb, [0x3a, 0x3a, 0x3a])));
}

/** Window buttons follow the platform: three lights on a Mac, caption buttons on Windows. */
export const isMac = /Mac/i.test(navigator.userAgent);

/** Keeps `data-window-focused` on <html> current, and names the platform for the styles. */
export function trackWindowFocus() {
  const root = document.documentElement;
  root.dataset.platform = isMac ? "mac" : "windows";
  const update = () => (root.dataset.windowFocused = String(document.hasFocus()));
  window.addEventListener("focus", update);
  window.addEventListener("blur", update);
  update();
}
