import { describe, expect, it } from "vitest";

import type { Resolved } from "../ipc";
import { job } from "../test/fixtures";
import {
  codecName,
  formatBytes,
  formatClock,
  groupJobs,
  resolvedSize,
  resolvedSummary,
  rowLabel,
  statusLine,
  thumbUrl,
  topsOutNote,
} from "./derive";

const resolved = (r: Partial<Resolved> = {}): Resolved => ({
  exact: true,
  formatIds: ["137", "140"],
  height: 1080,
  fps: 24,
  vcodec: "avc1.640028",
  acodec: "mp4a.40.2",
  ext: "mp4",
  bytes: 40_008_538,
  bytesApprox: false,
  sourceHeight: 2160,
  ...r,
});

describe("formatting", () => {
  it("formats sizes in decimal units", () => {
    expect(formatBytes(999)).toBe("999 bytes");
    expect(formatBytes(9_200_000)).toBe("9.2 MB");
    expect(formatBytes(312_000_000)).toBe("312 MB");
    expect(formatBytes(1_400_000_000)).toBe("1.4 GB");
    expect(formatBytes(null)).toBe("");
    expect(formatBytes(Number.NaN)).toBe("");
  });

  it("formats clocks", () => {
    expect(formatClock(41)).toBe("0:41");
    expect(formatClock(761)).toBe("12:41");
    expect(formatClock(3723)).toBe("1:02:03");
    expect(formatClock(null)).toBe("");
  });

  it("names codecs", () => {
    expect(codecName("avc1.640028")).toBe("H.264");
    expect(codecName("av01.0.08M.08")).toBe("AV1");
    expect(codecName("opus")).toBe("Opus");
    expect(codecName("")).toBe("");
  });
});

describe("what a preset produces", () => {
  it("summarizes and sizes the resolution", () => {
    expect(resolvedSummary(resolved())).toBe("1080p · H.264 · AAC · MP4");
    expect(resolvedSummary(resolved({ fps: 60 }))).toBe("1080p60 · H.264 · AAC · MP4");
    expect(resolvedSize(resolved())).toBe("40.0 MB");
    expect(resolvedSize(resolved({ exact: false }))).toBe("~40.0 MB");
    expect(resolvedSize(resolved({ bytesApprox: true }))).toBe("~40.0 MB");
  });

  it("says when a preset tops out below the source", () => {
    expect(topsOutNote({ kind: "mp4", maxHeight: null }, resolved())).toBe(
      "1080p in this format; the source has 2160p",
    );
    // Asking for 1080p and getting 1080p is not worth a note.
    expect(topsOutNote({ kind: "mp4", maxHeight: 1080 }, resolved())).toBeNull();
    expect(topsOutNote({ kind: "best", maxHeight: null }, resolved({ height: 2160 }))).toBeNull();
    expect(topsOutNote({ kind: "audio", format: "mp3" }, resolved({ height: null }))).toBeNull();
  });
});

describe("rows", () => {
  it("groups jobs into sections in a fixed order, newest completed first", () => {
    const jobs = {
      a: job({ id: "a", state: "completed", finishedAt: 1 }),
      b: job({ id: "b", state: "downloading" }),
      c: job({ id: "c", state: "queued" }),
      d: job({ id: "d", state: "completed", finishedAt: 5 }),
      e: job({ id: "e", state: "completed", cleared: true }),
    };
    const groups = groupJobs(["a", "b", "c", "d", "e"], jobs);
    expect(groups.map((g) => [g.id, g.jobs.map((j) => j.id)])).toEqual([
      ["active", ["b"]],
      ["queued", ["c"]],
      ["completed", ["d", "a"]],
    ]);
  });

  it("describes what a job is doing", () => {
    const downloading = job({
      state: "downloading",
      progress: {
        stage: "video",
        stageIndex: 1,
        stageCount: 2,
        fraction: 0.62,
        downloaded: 1,
        total: 2,
        speed: 1,
        eta: 1,
      },
    });
    expect(statusLine(downloading)).toBe("Downloading video (1 of 2)");
    expect(rowLabel(downloading)).toBe("A title, Downloading video (1 of 2), 62 percent");
    expect(statusLine(job({ state: "completed", bytes: 146_000_000 }))).toBe("146 MB · youtube.com");
    expect(
      statusLine(job({ state: "postProcessing", progress: { ...downloading.progress, stage: "merging" } })),
    ).toBe("Merging video and audio");
  });

  it("serves thumbnails through the app, never straight from the internet", () => {
    expect(thumbUrl("https://i.ytimg.com/vi/x/hq.jpg")).toBe(
      "http://thumb.localhost/?u=https%3A%2F%2Fi.ytimg.com%2Fvi%2Fx%2Fhq.jpg",
    );
    expect(thumbUrl("javascript:alert(1)")).toBeNull();
    expect(thumbUrl("")).toBeNull();
  });
});
