import { useVirtualizer } from "@tanstack/react-virtual";
import { Check } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { Button } from "../../components/Button";
import { Sheet } from "../../components/chrome";
import { Checkbox, SearchField, Spinner } from "../../components/controls";
import { Icon } from "../../components/Icon";
import type { DownloadRequest, Format, MediaInfo } from "../../ipc";
import { commands } from "../../ipc";
import { dismissPreview, openSheet, startPreview, updatePreview, useApp } from "../../store/app";
import { codecName, formatBytes, formatClock } from "../../store/derive";
import { s } from "../../strings";
import { OptionsSheet } from "../downloads/OptionsSheet";
import { PresetControls } from "../downloads/PreviewCard";
import { Thumb } from "../downloads/Thumb";
import styles from "./sheets.module.css";

const close = () => openSheet(null);

function QuitSheet({ count }: { count: number }) {
  return (
    <div className={styles.alert}>
      <h2 className={styles.title}>{s.quitTitle(count)}</h2>
      <p className={styles.body}>{s.quitBody}</p>
      <div className={styles.buttons}>
        <Button onClick={close}>{s.cancelButton}</Button>
        <Button variant="primary" data-autofocus onClick={() => void commands.quit()}>
          {s.quitButton}
        </Button>
      </div>
    </div>
  );
}

const shortcuts: [string, string][] = [
  ["Ctrl+V", s.shortcutAddCopied],
  ["Ctrl+N", s.newDownload],
  ["Ctrl+Shift+V", s.pasteAndDownload],
  ["Enter", s.shortcutDownloadPreview],
  ["Space", s.shortcutPauseResume],
  ["Delete", s.shortcutRemove],
  ["Ctrl+A", s.shortcutSelectAll],
  ["Ctrl+1, 2, 3", s.shortcutSections],
  ["Ctrl+F", s.searchHistory],
  ["Ctrl+Alt+S", s.toggleSidebar],
  ["Ctrl+,", s.settings],
  ["Ctrl+Q", s.quitButton],
];

function ShortcutsSheet() {
  return (
    <div className={styles.alert}>
      <h2 className={styles.title}>{s.keyboardShortcuts}</h2>
      <dl className={styles.shortcuts}>
        {shortcuts.map(([keys, what]) => (
          <div key={keys} className={styles.shortcut}>
            <dt>{what}</dt>
            <dd>{keys}</dd>
          </div>
        ))}
      </dl>
      <div className={styles.buttons}>
        <Button variant="primary" data-autofocus onClick={close}>
          {s.done}
        </Button>
      </div>
    </div>
  );
}

const credits: [string, string][] = [
  ["yt-dlp", "Unlicense"],
  ["FFmpeg", "GPL"],
  ["Deno", "MIT"],
  ["Tauri", "MIT or Apache-2.0"],
  ["React, Motion, Zustand, TanStack Virtual, Lucide", "MIT and ISC"],
];

function AboutSheet() {
  const version = useApp((st) => st.system.appVersion);
  return (
    <div className={styles.alert}>
      <h2 className={styles.title}>{s.app}</h2>
      <p className={styles.body}>{s.aboutVersion(version || "0.1.0")}</p>
      <p className={styles.body}>{s.legal}</p>
      <ul className={styles.credits}>
        {credits.map(([name, license]) => (
          <li key={name}>
            <span>{name}</span>
            <span className={styles.muted}>{license}</span>
          </li>
        ))}
      </ul>
      <div className={styles.buttons}>
        <Button onClick={() => void commands.openLink("project")}>{s.github}</Button>
        <Button variant="primary" data-autofocus onClick={close}>
          {s.done}
        </Button>
      </div>
    </div>
  );
}

function UpdatesSheet() {
  const [state, setState] = useState<
    | { phase: "checking" }
    | { phase: "current" }
    | { phase: "found"; version: string }
    | { phase: "error"; message: string }
    | { phase: "installing" }
  >({ phase: "checking" });
  useEffect(() => {
    commands.appUpdateCheck().then(
      (version) => setState(version ? { phase: "found", version } : { phase: "current" }),
      (error: unknown) => setState({ phase: "error", message: String(error) }),
    );
  }, []);
  return (
    <div className={styles.alert}>
      <h2 className={styles.title}>{s.checkForUpdates.replace("…", "")}</h2>
      {state.phase === "checking" && (
        <p className={styles.row}>
          <Spinner /> {s.checking}
        </p>
      )}
      {state.phase === "current" && <p className={styles.body}>{s.upToDate}</p>}
      {state.phase === "found" && <p className={styles.body}>{s.updateFound(state.version)}</p>}
      {state.phase === "error" && <p className={styles.body}>{state.message}</p>}
      {state.phase === "installing" && (
        <p className={styles.row}>
          <Spinner /> {s.installing}
        </p>
      )}
      <div className={styles.buttons}>
        <Button onClick={close}>{state.phase === "found" ? s.later : s.done}</Button>
        {state.phase === "found" && (
          <Button
            variant="primary"
            data-autofocus
            onClick={() => {
              setState({ phase: "installing" });
              commands
                .appUpdateInstall()
                .catch((error: unknown) => setState({ phase: "error", message: String(error) }));
            }}
          >
            {s.installAndRestart}
          </Button>
        )}
      </div>
    </div>
  );
}

function PlaylistSheet() {
  const preview = useApp((st) => st.preview);
  const skipDefault = useApp((st) => st.settings?.skipDownloaded ?? true);
  const [query, setQuery] = useState("");
  const [deselected, setDeselected] = useState<Set<string>>(new Set());
  const [skip, setSkip] = useState(skipDefault);
  const [anchor, setAnchor] = useState<number | null>(null);
  const [active, setActive] = useState(0);
  const scroll = useRef<HTMLDivElement>(null);
  const status = preview?.status.status === "playlist" ? preview.status : null;
  const entries = useMemo(() => status?.entries ?? [], [status]);
  const shown = useMemo(() => {
    const q = query.toLowerCase();
    return q
      ? entries.filter((e) => e.title.toLowerCase().includes(q) || e.uploader.toLowerCase().includes(q))
      : entries;
  }, [entries, query]);
  const isOn = (id: string, downloaded: boolean) => !deselected.has(id) && !(skip && downloaded);
  const chosen = entries.filter((e) => isOn(e.id, e.downloaded));
  const virtual = useVirtualizer({
    count: shown.length,
    getScrollElement: () => scroll.current,
    estimateSize: () => 44,
    overscan: 10,
  });
  if (!preview || !status) return null;

  const setRange = (to: number, on: boolean) => {
    const from = anchor ?? to;
    const [a, b] = from < to ? [from, to] : [to, from];
    const ids = shown.slice(a, b + 1).map((e) => e.id);
    setDeselected((prev) => {
      const next = new Set(prev);
      ids.forEach((id) => (on ? next.delete(id) : next.add(id)));
      return next;
    });
  };
  // Entries skipped as already downloaded cannot be picked, so they do not count here.
  const allOn = shown.filter((e) => !(skip && e.downloaded)).every((e) => isOn(e.id, e.downloaded));
  const toggle = (id: string, on: boolean) =>
    setDeselected((prev) => {
      const next = new Set(prev);
      if (on) next.add(id);
      else next.delete(id);
      return next;
    });

  return (
    <div className={styles.playlist}>
      <div className={styles.header}>
        <h2 className={styles.title}>{s.playlistTitle(status.title)}</h2>
        <span className={styles.muted}>
          {[status.uploader, s.playlistCount(entries.length, !status.complete)].filter(Boolean).join(" · ")}
        </span>
      </div>
      <div className={styles.tools}>
        <SearchField
          label={s.searchPlaylist}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className={styles.grow}
        />
        <Button
          variant="plain"
          onClick={() => setDeselected(allOn ? new Set(shown.map((e) => e.id)) : new Set())}
        >
          {allOn ? s.selectNone : s.selectAll}
        </Button>
      </div>
      <div ref={scroll} className={styles.entries}>
        <div
          role="listbox"
          aria-multiselectable
          aria-label={s.playlistTitle(status.title)}
          aria-activedescendant={shown[active] ? `entry-${shown[active].id}` : undefined}
          tabIndex={0}
          className={styles.entryList}
          style={{ height: virtual.getTotalSize(), position: "relative" }}
          onKeyDown={(e) => {
            const step = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
            if (step) {
              const next = Math.max(0, Math.min(shown.length - 1, active + step));
              setActive(next);
              virtual.scrollToIndex(next);
              if (e.shiftKey) setRange(next, true);
              else setAnchor(next);
            } else if (e.key === " ") {
              const entry = shown[active];
              if (entry) toggle(entry.id, isOn(entry.id, entry.downloaded));
            } else return;
            e.preventDefault();
          }}
        >
          {virtual.getVirtualItems().map((row) => {
            const entry = shown[row.index];
            if (!entry) return null;
            const on = isOn(entry.id, entry.downloaded);
            return (
              <div
                key={entry.id}
                id={`entry-${entry.id}`}
                role="option"
                aria-selected={on}
                data-active={row.index === active}
                className={styles.entry}
                style={{ transform: `translateY(${row.start}px)` }}
                onClick={(e) => {
                  setActive(row.index);
                  if (e.shiftKey) setRange(row.index, !on);
                  else {
                    toggle(entry.id, on);
                    setAnchor(row.index);
                  }
                }}
              >
                <span className={styles.box} data-on={on} aria-hidden>
                  {on && <Icon icon={Check} size={11} />}
                </span>
                <span className={styles.entryThumb}>
                  <Thumb src={entry.thumbnail} kind="video" state="completed" fraction={1} />
                </span>
                <span className={styles.entryTitle}>{entry.title}</span>
                {entry.downloaded && <span className={styles.tag}>{s.alreadyDownloaded}</span>}
                <span className={`${styles.muted} tabular`}>{formatClock(entry.duration)}</span>
              </div>
            );
          })}
        </div>
      </div>
      <div className={styles.footer}>
        <PresetControls request={preview.request} onChange={updatePreview} />
        <Checkbox checked={skip} onChange={setSkip} label={s.skipDownloaded} />
        <span className={styles.footerEnd}>
          <Button onClick={dismissPreview}>{s.cancelButton}</Button>
          <Button
            variant="primary"
            disabled={chosen.length === 0}
            onClick={() =>
              void startPreview(
                chosen.map((e) => e.id),
                skip,
              )
            }
          >
            {s.downloadCount(chosen.length)}
          </Button>
        </span>
      </div>
    </div>
  );
}

function describe(f: Format): string[] {
  const res = f.height ? `${f.width ?? "?"}×${f.height}` : s.formatAudioCell;
  return [
    f.id,
    res,
    f.fps ? `${Math.round(f.fps)}` : "",
    codecName(f.vcodec || f.acodec),
    f.tbr ? `${Math.round(f.tbr)}k` : "",
    `${f.bytesApprox && f.bytes ? s.approx : ""}${formatBytes(f.bytes)}`,
    f.ext,
  ];
}

function FormatsSheet({ jobId }: { jobId: string | null }) {
  const preview = useApp((st) => st.preview);
  const job = useApp((st) => (jobId ? st.jobs[jobId] : undefined));
  const [info, setInfo] = useState<MediaInfo | null>(
    preview?.status.status === "ready" && !jobId ? preview.status.info : null,
  );
  const [error, setError] = useState<string | null>(null);
  const request: DownloadRequest | undefined = jobId ? job?.request : preview?.request;
  const [video, setVideo] = useState<string | null>(request?.formats?.video ?? null);
  const [audio, setAudio] = useState<string | null>(request?.formats?.audio ?? null);
  useEffect(() => {
    if (!jobId) return;
    commands.jobFormats(jobId).then(setInfo, (e: unknown) => setError(String(e)));
  }, [jobId]);
  const videos = (info?.formats ?? [])
    .filter((f) => f.vcodec)
    .sort((a, b) => (b.height ?? 0) - (a.height ?? 0) || (b.tbr ?? 0) - (a.tbr ?? 0));
  const audios = (info?.formats ?? [])
    .filter((f) => !f.vcodec && f.acodec)
    .sort((a, b) => (b.tbr ?? 0) - (a.tbr ?? 0));
  const apply = () => {
    if (!request) return;
    const next = { ...request, formats: video || audio ? { video, audio } : null };
    if (jobId) void commands.jobUpdateRequest(jobId, next);
    else updatePreview(next);
    close();
  };
  const table = (title: string, list: Format[], value: string | null, set: (v: string | null) => void) => (
    <fieldset className={styles.formatGroup}>
      <legend className={styles.groupTitle}>{title}</legend>
      <div className={styles.formatTable} role="radiogroup" aria-label={title}>
        <label className={styles.formatRow}>
          <input type="radio" checked={value === null} onChange={() => set(null)} />
          <span className={styles.formatAuto}>{s.chosenByPreset}</span>
        </label>
        {list.map((f) => (
          <label key={f.id} className={styles.formatRow}>
            <input type="radio" checked={value === f.id} onChange={() => set(f.id)} />
            {describe(f).map((cell, i) => (
              <span key={i} className={styles.cell}>
                {cell}
              </span>
            ))}
          </label>
        ))}
      </div>
    </fieldset>
  );
  return (
    <div className={styles.playlist}>
      <div className={styles.header}>
        <h2 className={styles.title}>{s.formatsTitle}</h2>
        <span className={styles.muted}>{s.formatsBody}</span>
      </div>
      <div className={styles.entries}>
        {!info && !error && (
          <p className={styles.row}>
            <Spinner /> {s.gettingDetails}
          </p>
        )}
        {error && <p className={styles.body}>{error}</p>}
        {info && table(s.formatsVideo, videos, video, setVideo)}
        {info && table(s.formatsAudio, audios, audio, setAudio)}
      </div>
      <div className={styles.footer}>
        <span className={styles.grow} />
        <Button onClick={close}>{s.cancelButton}</Button>
        <Button variant="primary" disabled={!info} onClick={apply}>
          {jobId ? s.useAndRetry : s.useThese}
        </Button>
      </div>
    </div>
  );
}

export function Sheets() {
  const sheet = useApp((st) => st.sheet);
  const kind = sheet?.kind;
  const onClose = kind === "playlist" ? dismissPreview : close;
  // The dialog's name matches the title each sheet shows.
  const label = kind
    ? {
        quit: s.quitTitle(sheet.kind === "quit" ? sheet.count : 0),
        shortcuts: s.keyboardShortcuts,
        about: s.app,
        updates: s.checkForUpdates.replace("…", ""),
        playlist: s.playlistTitle(""),
        formats: s.formatsTitle,
        options: s.optionsTitle,
        onboarding: "",
      }[kind]
    : "";
  return (
    <Sheet
      open={kind !== undefined && kind !== "onboarding"}
      onClose={onClose}
      label={label}
      wide={kind === "playlist" || kind === "formats"}
    >
      {sheet?.kind === "quit" && <QuitSheet count={sheet.count} />}
      {kind === "shortcuts" && <ShortcutsSheet />}
      {kind === "about" && <AboutSheet />}
      {kind === "updates" && <UpdatesSheet />}
      {kind === "playlist" && <PlaylistSheet />}
      {sheet?.kind === "formats" && <FormatsSheet jobId={sheet.jobId} />}
      {kind === "options" && <OptionsSheet />}
    </Sheet>
  );
}
