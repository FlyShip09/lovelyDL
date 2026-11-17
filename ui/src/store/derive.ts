// Pure functions that turn engine data into what rows and cards show. No React here.

import type { Job, JobState, Preset, Resolved } from "../ipc";
import { errorCopy, s, stageLabel } from "../strings";

const units = ["KB", "MB", "GB", "TB"];

/** Sizes in decimal units, like Explorer's "MB" and Finder: 312 MB, 1.4 GB, 980 KB. */
export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes) || bytes < 0) return "";
  if (bytes < 1000) return `${Math.round(bytes)} bytes`;
  let value = bytes / 1000;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  const digits = value >= 100 || unit === 0 ? 0 : 1;
  return `${value.toFixed(digits)} ${units[unit] ?? ""}`;
}

export function formatSpeed(bytesPerSecond: number | null | undefined): string {
  const size = formatBytes(bytesPerSecond);
  return size ? `${size}/s` : "";
}

/** 0:41, 12:41, 1:02:03. */
export function formatClock(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || !Number.isFinite(seconds) || seconds < 0) return "";
  const total = Math.round(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const sec = total % 60;
  const pad = (n: number) => n.toString().padStart(2, "0");
  return h > 0 ? `${h}:${pad(m)}:${pad(sec)}` : `${m}:${pad(sec)}`;
}

export function presetName(preset: Preset): string {
  switch (preset.kind) {
    case "best":
      return s.bestQuality;
    case "mp4":
      return s.compatibleMp4;
    case "audio":
      return preset.format === "opus" ? "Opus" : preset.format.toUpperCase();
  }
}

export function heightName(height: number | null): string {
  return height === null ? s.anyHeight : height === 2160 ? "4K (2160p)" : `${height}p`;
}

const codecNames: [string, string][] = [
  ["avc", "H.264"],
  ["h264", "H.264"],
  ["hev", "HEVC"],
  ["hvc", "HEVC"],
  ["h265", "HEVC"],
  ["vp09", "VP9"],
  ["vp9", "VP9"],
  ["av01", "AV1"],
  ["mp4a", "AAC"],
  ["aac", "AAC"],
  ["opus", "Opus"],
  ["vorbis", "Vorbis"],
  ["mp3", "MP3"],
  ["flac", "FLAC"],
];

export function codecName(codec: string): string {
  if (!codec) return "";
  const lower = codec.toLowerCase();
  return codecNames.find(([prefix]) => lower.startsWith(prefix))?.[1] ?? codec.split(".")[0] ?? codec;
}

/** "1080p · H.264 · AAC · MP4" for what a preset will actually produce. */
export function resolvedSummary(r: Resolved): string {
  const parts: string[] = [];
  if (r.height !== null) parts.push(`${r.height}p${r.fps !== null && r.fps > 30 ? Math.round(r.fps) : ""}`);
  if (r.vcodec) parts.push(codecName(r.vcodec));
  if (r.acodec) parts.push(codecName(r.acodec));
  if (r.ext) parts.push(r.ext.toUpperCase());
  return parts.join(" · ");
}

export function resolvedSize(r: Resolved): string {
  const size = formatBytes(r.bytes);
  return size && (r.bytesApprox || !r.exact) ? `${s.approx}${size}` : size;
}

/** A note when the preset gives less than the source has: "1080p in this format; the source has 2160p". */
export function topsOutNote(preset: Preset, r: Resolved): string | null {
  if (preset.kind === "audio" || r.height === null || r.sourceHeight === null) return null;
  if (r.height >= r.sourceHeight) return null;
  const cap = preset.maxHeight;
  if (cap !== null && r.height >= Math.min(cap, r.sourceHeight)) return null;
  return s.presetTopsOut(r.height, r.sourceHeight);
}

export function isActive(state: JobState): boolean {
  return state === "probing" || state === "downloading" || state === "postProcessing";
}

export type SectionId = "active" | "queued" | "paused" | "failed" | "canceled" | "completed";

export const sectionTitle: Record<SectionId, string> = {
  active: s.sectionActive,
  queued: s.sectionQueued,
  paused: s.sectionPaused,
  failed: s.sectionFailed,
  canceled: s.sectionCanceled,
  completed: s.sectionCompleted,
};

export function sectionOf(state: JobState): SectionId {
  switch (state) {
    case "probing":
    case "downloading":
    case "postProcessing":
      return "active";
    case "queued":
      return "queued";
    case "paused":
      return "paused";
    case "failed":
      return "failed";
    case "canceled":
      return "canceled";
    case "completed":
      return "completed";
  }
}

const sectionOrder: SectionId[] = ["active", "queued", "paused", "failed", "canceled", "completed"];

/** The Downloads list: sections in a fixed order, jobs in queue order inside each. */
export function groupJobs(order: string[], jobs: Record<string, Job>): { id: SectionId; jobs: Job[] }[] {
  const buckets = new Map<SectionId, Job[]>();
  for (const id of order) {
    const job = jobs[id];
    if (!job || job.cleared) continue;
    const section = sectionOf(job.state);
    const list = buckets.get(section) ?? [];
    list.push(job);
    buckets.set(section, list);
  }
  // Finished downloads show newest first; the rest keep the queue order.
  buckets.get("completed")?.sort((a, b) => (b.finishedAt ?? 0) - (a.finishedAt ?? 0));
  return sectionOrder.flatMap((id) => {
    const list = buckets.get(id);
    return list ? [{ id, jobs: list }] : [];
  });
}

/** The second line of a row: what is happening now, in words. */
export function statusLine(job: Job): string {
  const p = job.progress;
  switch (job.state) {
    case "queued":
      return s.waiting;
    case "paused":
      return (p.downloaded ?? 0) > 0 && p.total
        ? s.ofTotal(formatBytes(p.downloaded), formatBytes(p.total))
        : s.paused;
    case "canceled":
      return s.canceled;
    case "probing":
      return stageLabel.probing;
    case "postProcessing":
      return stageLabel[p.stage];
    case "downloading": {
      const label = stageLabel[p.stage];
      const multi = p.stageCount > 1 && (p.stage === "video" || p.stage === "audio");
      return multi ? `${label} (${s.stageOf(p.stageIndex, p.stageCount)})` : label;
    }
    case "completed":
      if (job.fileMissing) return s.fileMissing;
      return [formatBytes(job.bytes), job.site].filter(Boolean).join(" · ");
    case "failed":
      // The short explanation; the raw message stays in the details below the row.
      return job.error ? errorCopy[job.error.kind].title : "";
  }
}

/** Text a screen reader hears for a row: title, state and percent. */
export function rowLabel(job: Job): string {
  const title = job.title || job.url;
  const percent =
    job.progress.fraction !== null ? `, ${Math.round(job.progress.fraction * 100)} percent` : "";
  return `${title}, ${statusLine(job)}${isActive(job.state) ? percent : ""}`;
}

/** The thumbnail address served by the app, so the page never loads remote images itself. */
export function thumbUrl(link: string): string | null {
  // Mock mode hands in ready-made images.
  if (link.startsWith("data:")) return link;
  if (!link.startsWith("http")) return null;
  return `http://thumb.localhost/?u=${encodeURIComponent(link)}`;
}
