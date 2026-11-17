import "./styles/global.css";

import { MotionConfig } from "motion/react";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { boot, commands, inTauri } from "./ipc";
import { applySystem, trackWindowFocus } from "./system";

async function start() {
  // In a plain browser, a scripted stand-in answers every command (ui/src/ipc/mock.ts).
  if (!inTauri) await import("./ipc/mock");
  const info = boot();
  if (info.system) applySystem(info.system);
  trackWindowFocus();

  // WebView2's own context menu stays only where text can be edited or selected.
  document.addEventListener("contextmenu", (e) => {
    const target = e.target instanceof Element ? e.target : null;
    if (!target?.closest("input, textarea, [contenteditable], .selectable")) e.preventDefault();
  });

  const view = info.view === "settings" ? "settings" : "main";
  const { App } =
    view === "settings" ? await import("./features/settings/SettingsApp") : await import("./App");
  const root = document.getElementById("root");
  if (!root) return;
  createRoot(root).render(
    <StrictMode>
      <MotionConfig reducedMotion="user">
        <App />
      </MotionConfig>
    </StrictMode>,
  );
  // The window stays hidden until the first frame is on screen: no blank or white flash.
  requestAnimationFrame(() => requestAnimationFrame(() => void commands.windowAction("show")));
}

void start();
