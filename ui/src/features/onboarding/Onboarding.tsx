// First run: welcome, choose the download folder, install the tools, done. Steps slide
// sideways like a setup assistant; installs show real progress.

import { AnimatePresence, motion, useIsPresent, type HTMLMotionProps } from "motion/react";
import { useState } from "react";

import { Button } from "../../components/Button";
import { CaptionButtons, TrafficLights } from "../../components/chrome";
import { ProgressBar, Toggle } from "../../components/controls";
import type { Tool } from "../../ipc";
import { commands } from "../../ipc";
import { gentle } from "../../motion/tokens";
import { openSheet, saveSettings, useApp } from "../../store/app";
import { s } from "../../strings";
import styles from "./onboarding.module.css";

const toolNames: Record<Tool["id"], [string, string]> = {
  ytdlp: ["yt-dlp", s.toolPurposeYtdlp],
  ffmpeg: ["ffmpeg", s.toolPurposeFfmpeg],
  deno: ["Deno", s.toolPurposeDeno],
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
            {tool.origin === "system" ? s.toolFound(tool.version) : tool.version}
          </span>
        )}
        {status.status === "missing" && <span className={styles.muted}>{s.notInstalled}</span>}
        {status.status === "checking" && <span className={styles.muted}>{s.checking}</span>}
        {status.status === "broken" && <span className={styles.broken}>{s.toolBroken}</span>}
        {status.status === "installing" && (
          <ProgressBar value={status.fraction} label={s.installingTool(name)} />
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
      setError(s.cantSaveFolder(String(e)));
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
      <h1 className={styles.title}>{s.welcomeTitle}</h1>
      <p className={styles.lead}>{s.welcomeLead}</p>
      <p className={styles.legal}>{s.legal}</p>
      <div className={styles.buttons}>
        <Button variant="primary" large data-autofocus onClick={() => go(1)}>
          {s.continueButton}
        </Button>
      </div>
    </div>,
    <div key="folder" className={styles.step}>
      <h1 className={styles.title}>{s.folderTitle}</h1>
      <p className={styles.lead}>{s.folderLead}</p>
      <div className={styles.folder}>
        <span className={`${styles.path} selectable`}>{settings?.downloadDir}</span>
        <Button onClick={() => void chooseFolder()}>{s.changeDestination}</Button>
      </div>
      {error && <p className={styles.error}>{error}</p>}
      <div className={styles.buttons}>
        <Button large onClick={() => go(0)}>
          {s.back}
        </Button>
        <Button
          variant="primary"
          large
          onClick={() =>
            void commands.checkFolder(settings?.downloadDir ?? "").then(
              () => go(2),
              (e: unknown) => setError(s.cantSaveFolder(String(e))),
            )
          }
        >
          {s.continueButton}
        </Button>
      </div>
    </div>,
    <div key="tools" className={styles.step}>
      <h1 className={styles.title}>{s.toolsTitle}</h1>
      <p className={styles.lead}>{s.toolsLead}</p>
      <div className={styles.tools}>
        {tools.map((tool) => (
          <ToolRow key={tool.id} tool={tool} />
        ))}
      </div>
      <label className={styles.option}>
        <Toggle
          checked={settings?.useSystemTools ?? false}
          label={s.useSystemTools}
          onChange={(on) => void saveSettings((st) => ({ ...st, useSystemTools: on }))}
        />
        <span>{s.useSystemTools}</span>
      </label>
      {error && <p className={styles.error}>{error}</p>}
      <div className={styles.buttons}>
        <Button large onClick={() => go(1)}>
          {s.back}
        </Button>
        {allReady ? (
          <Button variant="primary" large onClick={() => go(3)}>
            {s.continueButton}
          </Button>
        ) : (
          <>
            {/* Offline or rate-limited, the app must still open; Settings can install later. */}
            {!busy && (
              <Button variant="plain" large onClick={() => go(3)}>
                {s.setUpLater}
              </Button>
            )}
            <Button variant="primary" large disabled={busy} onClick={() => void install()}>
              {busy ? s.installing : s.install}
            </Button>
          </>
        )}
      </div>
    </div>,
    <div key="done" className={styles.step}>
      <h1 className={styles.title}>{s.doneTitle}</h1>
      <p className={styles.lead}>{s.doneLead}</p>
      {!allReady && <p className={styles.lead}>{s.toolsLater}</p>}
      <div className={styles.buttons}>
        <Button variant="primary" large data-autofocus onClick={() => void finish()}>
          {s.startUsing}
        </Button>
      </div>
    </div>,
  ];

  return (
    <div className={styles.window}>
      <div className={styles.top} data-tauri-drag-region>
        <TrafficLights />
        <span data-tauri-drag-region />
        <CaptionButtons />
      </div>
      <div className={styles.stage}>
        <AnimatePresence mode="popLayout" initial={false} custom={direction}>
          <Slide
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
          </Slide>
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

/** A step on its way out stays visible for its slide but takes no more clicks or focus. */
function Slide(props: HTMLMotionProps<"div">) {
  return <motion.div {...props} inert={!useIsPresent()} />;
}
