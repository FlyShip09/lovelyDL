// The UI's copy of the engine's state, rebuilt from `get_snapshot` and kept current by the
// `update` event. Rust owns the truth; this store only mirrors it and holds view state.

import { useEffect } from "react";
import { create } from "zustand";

import type {
  Change,
  Components,
  DownloadRequest,
  Job,
  Notice,
  PlaylistChoice,
  Preview,
  Settings,
  Source,
  SystemInfo,
  UiCommand,
} from "../ipc";
import { commands, events } from "../ipc";
import { s } from "../strings";
import { applySystem } from "../system";

export type View = "downloads" | "history" | "failed";

export type Sheet =
  | { kind: "playlist" }
  | { kind: "quit"; count: number }
  | { kind: "shortcuts" }
  | { kind: "about" }
  | { kind: "updates" }
  | { kind: "formats"; jobId: string | null }
  | { kind: "options" }
  | { kind: "onboarding" };

interface State {
  ready: boolean;
  jobs: Record<string, Job>;
  order: string[];
  preview: Preview | null;
  settings: Settings | null;
  components: Components | null;
  system: SystemInfo;

  view: View;
  selection: string[];
  sheet: Sheet | null;
  clipboardOffer: boolean;
  linkError: string | null;
  /** Bumped whenever finished jobs change, so a visible history list queries again. */
  historyVersion: number;
  /** A row to scroll to and select (from a notification click). */
  focusJob: string | null;
  /** Polite message for screen readers. */
  announcement: string;
  /** Jobs that finished a moment ago: their row stays put and shows a check for a beat. */
  justCompleted: string[];
}

const initial: State = {
  ready: false,
  jobs: {},
  order: [],
  preview: null,
  settings: null,
  components: null,
  system: { accent: "", material: false, appVersion: "" },
  view: "downloads",
  selection: [],
  sheet: null,
  clipboardOffer: false,
  linkError: null,
  historyVersion: 0,
  focusJob: null,
  announcement: "",
  justCompleted: [],
};

/** How long a finished row stays where it was, with its check mark, before it moves. */
export const SETTLE_MS = 1800;

export const useApp = create<State>()(() => initial);

const set = useApp.setState;
const get = useApp.getState;

function announce(notice: Notice): string {
  switch (notice.kind) {
    case "finished":
      return `Downloaded ${notice.title}`;
    case "failed":
      return `Download failed: ${notice.title}`;
    case "groupFinished":
      return `Playlist ${notice.title} downloaded`;
  }
}

function runCommand(command: UiCommand) {
  switch (command) {
    case "newDownload":
      set({ view: "downloads" });
      document.querySelector<HTMLInputElement>("[data-url-field]")?.focus();
      break;
    case "showDownloads":
      set({ view: "downloads" });
      break;
    case "showFailed":
      set({ view: "failed" });
      break;
    case "confirmQuit": {
      const count = Object.values(get().jobs).filter((j) =>
        ["probing", "downloading", "postProcessing"].includes(j.state),
      ).length;
      set({ sheet: { kind: "quit", count } });
      break;
    }
    case "checkClipboard":
      set({ clipboardOffer: true });
      break;
  }
}

/** Applies one batch of changes from Rust. */
export function apply(changes: Change[]) {
  const state = get();
  let jobs = state.jobs;
  let order = state.order;
  const patch: Partial<State> = {};
  const touch = () => {
    if (jobs === state.jobs) jobs = { ...jobs };
  };
  for (const change of changes) {
    switch (change.type) {
      case "jobAdded":
        touch();
        jobs[change.data.id] = change.data;
        if (!order.includes(change.data.id)) order = [...order, change.data.id];
        break;
      case "jobUpdated": {
        const id = change.data.id;
        if (change.data.state === "completed" && jobs[id]?.state !== "completed") {
          patch.justCompleted = [...(patch.justCompleted ?? state.justCompleted), id];
          window.setTimeout(
            () => set((st) => ({ justCompleted: st.justCompleted.filter((x) => x !== id) })),
            SETTLE_MS,
          );
        }
        touch();
        jobs[id] = change.data;
        break;
      }
      case "jobRemoved":
        jobs = Object.fromEntries(Object.entries(jobs).filter(([id]) => id !== change.data));
        order = order.filter((id) => id !== change.data);
        patch.selection = (patch.selection ?? state.selection).filter((id) => id !== change.data);
        break;
      case "progress": {
        const job = jobs[change.data.id];
        if (job) {
          touch();
          jobs[change.data.id] = { ...job, progress: change.data.progress };
        }
        break;
      }
      case "order":
        order = change.data;
        break;
      case "preview": {
        patch.preview = change.data;
        const sheet = patch.sheet === undefined ? state.sheet : patch.sheet;
        if (change.data?.status.status === "playlist") patch.sheet = { kind: "playlist" };
        // A new link replaced the playlist: its sheet would be left with nothing to show.
        else if (sheet?.kind === "playlist") patch.sheet = null;
        break;
      }
      case "playlistEntries": {
        const preview = patch.preview ?? state.preview;
        if (preview?.id === change.data.previewId && preview.status.status === "playlist") {
          // Entries already present are skipped, so a repeated batch can never duplicate them.
          const known = new Set(preview.status.entries.map((e) => e.id));
          const fresh = change.data.entries.filter((e) => !known.has(e.id));
          patch.preview = {
            ...preview,
            status: { ...preview.status, entries: [...preview.status.entries, ...fresh] },
          };
        }
        break;
      }
      case "settings":
        patch.settings = change.data;
        break;
      case "components":
        patch.components = change.data;
        break;
      case "system":
        patch.system = change.data;
        applySystem(change.data);
        break;
      case "notice":
        patch.announcement = announce(change.data);
        break;
      case "command":
        runCommand(change.data);
        break;
      case "showJob":
        patch.view = change.data in jobs && jobs[change.data]?.state === "failed" ? "failed" : "downloads";
        patch.selection = [change.data];
        patch.focusJob = change.data;
        break;
      case "historyChanged":
        patch.historyVersion = (patch.historyVersion ?? state.historyVersion) + 1;
        break;
    }
  }
  if (jobs !== state.jobs) patch.jobs = jobs;
  if (order !== state.order) patch.order = order;
  set(patch);
}

/** Loads everything from Rust and starts listening. Safe to call again after a reload. */
export async function connect(): Promise<() => void> {
  // Listen first, so nothing that happens during the snapshot call is missed twice or lost.
  const queued: Change[][] = [];
  let loaded = false;
  const unlisten = await events.update.listen((event) => {
    if (loaded) apply(event.payload);
    else queued.push(event.payload);
  });
  const snapshot = await commands.getSnapshot();
  applySystem(snapshot.system);
  set({
    ready: true,
    jobs: Object.fromEntries(snapshot.jobs.map((j) => [j.id, j])),
    order: snapshot.jobs.map((j) => j.id),
    preview: snapshot.preview,
    settings: snapshot.settings,
    components: snapshot.components,
    system: snapshot.system,
    sheet: !snapshot.settings.onboardingDone
      ? { kind: "onboarding" }
      : snapshot.preview?.status.status === "playlist"
        ? { kind: "playlist" }
        : null,
  });
  loaded = true;
  // Changes that raced the snapshot are re-applied; applying a newer state twice is harmless.
  queued.forEach(apply);
  return unlisten;
}

/**
 * Connects while the component is mounted. Unmounting before the connection is up (React's
 * development double mount does exactly that) still removes the listener once it exists;
 * otherwise every change would be applied twice.
 */
export function useConnection() {
  useEffect(() => {
    let stop: (() => void) | undefined;
    let gone = false;
    void connect().then((unlisten) => {
      if (gone) unlisten();
      else stop = unlisten;
    });
    return () => {
      gone = true;
      stop?.();
    };
  }, []);
}

// ----- actions ---------------------------------------------------------------------------

export async function addLinks(text: string, source: Source): Promise<boolean> {
  try {
    await commands.addLinks(text, source);
    set({ linkError: null, view: "downloads" });
    return true;
  } catch (error) {
    set({ linkError: typeof error === "string" ? error : s.noLink });
    return false;
  }
}

export function clearLinkError() {
  if (get().linkError) set({ linkError: null });
}

export function updatePreview(request: DownloadRequest) {
  const preview = get().preview;
  if (!preview) return;
  set({ preview: { ...preview, request } });
  void commands.previewUpdate(preview.id, request);
}

export function choosePreview(choice: PlaylistChoice) {
  const preview = get().preview;
  if (preview) void commands.previewChoose(preview.id, choice);
}

export function dismissPreview() {
  const preview = get().preview;
  if (!preview) return;
  set({ preview: null, sheet: get().sheet?.kind === "playlist" ? null : get().sheet });
  void commands.previewDismiss(preview.id);
}

export async function startPreview(selection: string[] = [], skipDownloaded = false) {
  const preview = get().preview;
  if (!preview) return;
  try {
    await commands.previewStart(preview.id, preview.request, selection, skipDownloaded);
    set({ sheet: null, view: "downloads" });
  } catch (error) {
    set({ linkError: String(error) });
  }
}

export function setView(view: View) {
  set({ view, selection: [] });
}

export function select(ids: string[]) {
  set({ selection: ids });
}

export function openSheet(sheet: Sheet | null) {
  set({ sheet });
}

export function dismissClipboard() {
  set({ clipboardOffer: false });
  void commands.clipboardDismiss();
}

export async function acceptClipboard() {
  set({ clipboardOffer: false });
  try {
    await commands.clipboardTakeLink();
    set({ view: "downloads", linkError: null });
  } catch (error) {
    set({ linkError: String(error) });
  }
}

export async function saveSettings(change: (s: Settings) => Settings) {
  const current = get().settings;
  if (!current) return;
  const next = change(current);
  set({ settings: next });
  await commands.settingsSet(next);
}

export function focusHandled() {
  set({ focusJob: null });
}
