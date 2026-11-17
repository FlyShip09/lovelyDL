// First run: welcome, choose the download folder, install the tools, done. Steps slide
// sideways like a setup assistant; installs show real progress.

import { AnimatePresence, motion } from "motion/react";
import { useState } from "react";

import { Button } from "../../components/Button";
import { TrafficLights } from "../../components/chrome";
import { ProgressBar, Toggle } from "../../components/controls";
import type { Tool } from "../../ipc";
import { commands } from "../../ipc";
import { gentle } from "../../motion/tokens";
import { openSheet, saveSettings, useApp } from "../../store/app";
import { s } from "../../strings";
import styles from "./onboarding.module.css";

const toolNames: Record<Tool["id"], [string, string]> = {
  ytdlp: ["yt-dlp", "Reads the site and downloads the media"],
  ffmpeg: ["ffmpeg", "Joins video and audio, converts formats"],
  deno: ["Deno", "Lets yt-dlp read YouTube"],
};

function ToolRow({ tool }: { tool: Tool }) {
  const [name, what] = toolNames[tool.id];
  const status = tool.status;
  return (
    <div className={styles.tool}>
      <div className={styles.toolText}>
        <span className={styles.toolName}>{name}</span>
        <span className={styles.muted}>{what}</span>
      </div>
      <div className={styles.toolState}>
        {status.status === "ready" && (
          <span className={styles.ready}>
            {tool.origin === "system" ? `Found, ${tool.version}` : tool.version}
          </span>
        )}
        {status.status === "missing" && <span className={styles.muted}>Not installed</span>}
        {status.status === "checking" && <span className={styles.muted}>Checking…</span>}
        {status.status === "broken" && <span className={styles.broken}>Doesn't run</span>}
        {status.status === "installing" && (
          <ProgressBar value={status.fraction} label={`Installing ${name}`} />
        )}
      </div>
    </div>
  );
}

export function Onboarding() {
  const [step, setStep] = useState(0);
  const [direction, setDirection] = useState(1);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const settings = useApp((st) => st.settings);
  const components = useApp((st) => st.components);
  const tools = components ? [components.ytdlp, components.ffmpeg, components.deno] : [];
  const allReady = tools.length > 0 && tools.every((t) => t.status.status === "ready");

  const go = (to: number) => {
    setDirection(to > step ? 1 : -1);
    setError(null);
    setStep(to);
  };

  const chooseFolder = async () => {
    const folder = await commands.chooseFolder(settings?.downloadDir ?? "");
    if (!folder) return;
    try {
      // Writing a test file now means any Windows permission question comes up here, with context.
      await commands.checkFolder(folder);
      await saveSettings((st) => ({ ...st, downloadDir: folder }));
      setError(null);
    } catch (e) {
      setError(`lovelyDL can't save to that folder: ${String(e)}`);
    }
  };

  const install = async () => {
    setBusy(true);
    setError(null);
    try {
      await commands.componentsInstallMissing();
    } catch (e) {
      setError(String(e));
    }
    setBusy(false);
  };

  const finish = async () => {
    await saveSettings((st) => ({ ...st, onboardingDone: true }));
    openSheet(null);
  };

  const steps = [
    <div key="welcome" className={styles.step}>
      <img src="/icon.svg" alt="" className={styles.icon} />
      <h1 className={styles.title}>Welcome to lovelyDL</h1>
      <p className={styles.lead}>
        Paste a link, get the file. Video and audio from the sites yt-dlp supports, saved to your PC.
      </p>
      <p className={styles.legal}>{s.legal}</p>
      <div className={styles.buttons}>
        <Button variant="primary" large data-autofocus onClick={() => go(1)}>
          Continue
        </Button>
      </div>
    </div>,
    <div key="folder" className={styles.step}>
      <h1 className={styles.title}>Where should downloads go?</h1>
      <p className={styles.lead}>You can change this later, or for a single download.</p>
      <div className={styles.folder}>
        <span className={`${styles.path} selectable`}>{settings?.downloadDir}</span>
        <Button onClick={() => void chooseFolder()}>{s.changeDestination}</Button>
      </div>
      {error && <p className={styles.error}>{error}</p>}
      <div className={styles.buttons}>
        <Button large onClick={() => go(0)}>
          Back
        </Button>
        <Button
          variant="primary"
          large
          onClick={() =>
            void commands.checkFolder(settings?.downloadDir ?? "").then(
              () => go(2),
              (e: unknown) => setError(`lovelyDL can't save to that folder: ${String(e)}`),
            )
          }
        >
          Continue
        </Button>
      </div>
    </div>,
    <div key="tools" className={styles.step}>
      <h1 className={styles.title}>Get the tools</h1>
      <p className={styles.lead}>
        lovelyDL uses three free tools. It downloads them from their official releases, checks each file, and
        keeps them up to date.
      </p>
      <div className={styles.tools}>
        {tools.map((tool) => (
          <ToolRow key={tool.id} tool={tool} />
        ))}
      </div>
      <label className={styles.option}>
        <Toggle
          checked={settings?.useSystemTools ?? false}
          label="Use tools already installed on this PC"
          onChange={(on) => void saveSettings((st) => ({ ...st, useSystemTools: on }))}
        />
        <span>Use tools already installed on this PC</span>
      </label>
      {error && <p className={styles.error}>{error}</p>}
      <div className={styles.buttons}>
        <Button large onClick={() => go(1)}>
          Back
        </Button>
        {allReady ? (
          <Button variant="primary" large onClick={() => go(3)}>
            Continue
          </Button>
        ) : (
          <Button variant="primary" large disabled={busy} onClick={() => void install()}>
            {busy ? "Installing…" : "Install"}
          </Button>
        )}
      </div>
    </div>,
    <div key="done" className={styles.step}>
      <h1 className={styles.title}>You're set</h1>
      <p className={styles.lead}>
        Copy a link in your browser, come back, and press Ctrl+V. Or drag the link onto the window.
      </p>
      <div className={styles.buttons}>
        <Button variant="primary" large data-autofocus onClick={() => void finish()}>
          Start using lovelyDL
        </Button>
      </div>
    </div>,
  ];

  return (
    <div className={styles.window}>
      <div className={styles.top} data-tauri-drag-region>
        <TrafficLights />
      </div>
      <div className={styles.stage}>
        <AnimatePresence mode="popLayout" initial={false} custom={direction}>
          <motion.div
            key={step}
            custom={direction}
            variants={{
              enter: (d: number) => ({ x: d * 80, opacity: 0 }),
              center: { x: 0, opacity: 1 },
              exit: (d: number) => ({ x: d * -80, opacity: 0 }),
            }}
            initial="enter"
            animate="center"
            exit="exit"
            transition={gentle}
            className={styles.slide}
          >
            {steps[step]}
          </motion.div>
        </AnimatePresence>
      </div>
      <div className={styles.dots} aria-hidden>
        {steps.map((_, i) => (
          <span key={i} className={styles.dot} data-on={i === step} />
        ))}
      </div>
    </div>
  );
}
