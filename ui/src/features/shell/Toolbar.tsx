import { Ellipsis, PanelLeft, Settings } from "lucide-react";
import { useRef, useState } from "react";

import { IconButton } from "../../components/Button";
import { TrafficLights } from "../../components/chrome";
import { type MenuItem, openMenuBelow } from "../../components/Menu";
import { commands } from "../../ipc";
import { addLinks, clearLinkError, openSheet, saveSettings, useApp } from "../../store/app";
import { s } from "../../strings";
import styles from "./shell.module.css";

export function toggleSidebar() {
  void saveSettings((st) => ({ ...st, sidebarVisible: !st.sidebarVisible }));
}

export function moreMenu(): MenuItem[] {
  const { jobs } = useApp.getState();
  const list = Object.values(jobs);
  const any = (states: string[]) => list.some((j) => states.includes(j.state));
  return [
    {
      label: s.newDownload,
      shortcut: "Ctrl+N",
      onSelect: () => document.querySelector<HTMLInputElement>("[data-url-field]")?.focus(),
    },
    {
      label: s.pasteAndDownload,
      shortcut: "Ctrl+Shift+V",
      onSelect: () => void commands.clipboardTakeLink().catch(() => undefined),
    },
    { label: s.openDownloadFolder, onSelect: () => void commands.openFolder() },
    { separator: true },
    {
      label: s.pauseAll,
      disabled: !any(["queued", "probing", "downloading", "postProcessing"]),
      onSelect: () => void commands.pauseAll(),
    },
    { label: s.resumeAll, disabled: !any(["paused"]), onSelect: () => void commands.resumeAll() },
    { label: s.retryFailed, disabled: !any(["failed"]), onSelect: () => void commands.retryFailed() },
    {
      label: s.clearCompleted,
      disabled: !any(["completed"]),
      onSelect: () => void commands.clearCompleted(),
    },
    { separator: true },
    { label: s.supportedSites, onSelect: () => void commands.openLink("supportedSites") },
    { label: s.keyboardShortcuts, shortcut: "Ctrl+/", onSelect: () => openSheet({ kind: "shortcuts" }) },
    { label: s.reportIssue, onSelect: () => void commands.openLink("reportIssue") },
    { separator: true },
    { label: s.checkForUpdates, onSelect: () => openSheet({ kind: "updates" }) },
    { label: s.about, onSelect: () => openSheet({ kind: "about" }) },
  ];
}

function UrlField() {
  const [value, setValue] = useState("");
  const probing = useApp((st) => st.preview?.status.status === "probing");
  const submit = async () => {
    if (!value.trim()) return;
    if (await addLinks(value, "typed")) setValue("");
  };
  return (
    <div className={styles.urlWrap}>
      <input
        data-url-field
        className={styles.url}
        value={value}
        placeholder={s.urlPlaceholder}
        aria-label={s.urlLabel}
        spellCheck={false}
        autoComplete="off"
        onChange={(e) => {
          setValue(e.target.value);
          clearLinkError();
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") void submit();
          if (e.key === "Escape") {
            setValue("");
            e.currentTarget.blur();
          }
        }}
      />
      {probing && (
        <span className={styles.activity} aria-hidden>
          <span />
        </span>
      )}
    </div>
  );
}

export function Toolbar({ sidebarVisible }: { sidebarVisible: boolean }) {
  const more = useRef<HTMLButtonElement>(null);
  return (
    <div className={styles.toolbar} data-tauri-drag-region data-lights={!sidebarVisible}>
      {!sidebarVisible && (
        <>
          <TrafficLights />
          <IconButton icon={PanelLeft} label={s.toggleSidebar} onClick={toggleSidebar} />
        </>
      )}
      <UrlField />
      <div className={styles.trailing}>
        <IconButton
          ref={more}
          icon={Ellipsis}
          label={s.more}
          aria-haspopup="menu"
          onClick={() => more.current && openMenuBelow(more.current, moreMenu(), "end")}
        />
        <IconButton icon={Settings} label={s.settings} onClick={() => void commands.openSettings()} />
      </div>
    </div>
  );
}
