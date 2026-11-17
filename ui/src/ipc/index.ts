// The only module the UI imports for talking to Rust. Inside the app it uses the generated
// bindings directly; in a browser, `installMock` (ui/src/ipc/mock.ts) answers them instead.

export { commands, events } from "./bindings";
export type {
  AudioFormat,
  ErrorKind,
  Fix,
  HistoryFilter,
  JobState,
  Link,
  LiveStatus,
  MediaKind,
  OnAdd,
  PlaylistChoice,
  ReleaseChannel,
  Source,
  SponsorBlock,
  Stage,
  ToolId,
  ToolOrigin,
  UiCommand,
  WindowAction,
} from "./bindings";

import type {
  Change_Serialize,
  Components,
  DownloadRequest_Serialize,
  Format_Serialize,
  FormatPick_Serialize,
  Group,
  HistoryPage_Serialize,
  Job_Serialize,
  JobError_Serialize,
  MediaInfo_Serialize,
  Notice,
  Options_Serialize,
  PlaylistEntry_Serialize,
  Preset_Serialize,
  Preview_Serialize,
  PreviewStatus_Serialize,
  Progress_Serialize,
  Resolved_Serialize,
  Section,
  Settings_Serialize,
  Snapshot_Serialize,
  SystemInfo,
  Tool,
  ToolStatus,
} from "./bindings";

// Data as Rust sends it: every field present.
export type Change = Change_Serialize;
export type DownloadRequest = DownloadRequest_Serialize;
export type Format = Format_Serialize;
export type FormatPick = FormatPick_Serialize;
export type HistoryPage = HistoryPage_Serialize;
export type Job = Job_Serialize;
export type JobError = JobError_Serialize;
export type MediaInfo = MediaInfo_Serialize;
export type Options = Options_Serialize;
export type PlaylistEntry = PlaylistEntry_Serialize;
export type Preset = Preset_Serialize;
export type Preview = Preview_Serialize;
export type PreviewStatus = PreviewStatus_Serialize;
export type Progress = Progress_Serialize;
export type Resolved = Resolved_Serialize;
export type Settings = Settings_Serialize;
export type Snapshot = Snapshot_Serialize;
export type { Components, Group, Notice, Section, SystemInfo, Tool, ToolStatus };

/** True inside the lovelyDL app, false in a plain browser (mock mode). */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

interface Boot {
  view?: string;
  system?: SystemInfo;
}

/** What the app knew before the page loaded (see src-tauri/src/windows.rs). */
export function boot(): Boot {
  const fromApp = (window as unknown as { __LOVELYDL_BOOT__?: Boot }).__LOVELYDL_BOOT__;
  if (fromApp) return fromApp;
  const params = new URLSearchParams(location.search);
  return { view: params.get("view") ?? "main" };
}
