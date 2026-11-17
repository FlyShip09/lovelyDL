import { beforeEach, describe, expect, it, vi } from "vitest";

import { contrast, readableOn, solidAccent } from "../system";
import { apply, SETTLE_MS, useApp } from "./app";
import { job } from "../test/fixtures";

const initial = useApp.getState();

describe("applying changes from Rust", () => {
  beforeEach(() => {
    useApp.setState(initial, true);
    vi.useFakeTimers();
  });

  it("adds, updates, orders and removes jobs", () => {
    apply([
      { type: "jobAdded", data: job({ id: "a" }) },
      { type: "jobAdded", data: job({ id: "b" }) },
    ]);
    expect(useApp.getState().order).toEqual(["a", "b"]);
    apply([{ type: "order", data: ["b", "a"] }]);
    expect(useApp.getState().order).toEqual(["b", "a"]);
    const progress = { ...job().progress, fraction: 0.5, stage: "media" as const };
    apply([{ type: "progress", data: { id: "a", progress } }]);
    expect(useApp.getState().jobs.a?.progress.fraction).toBe(0.5);
    useApp.setState({ selection: ["a", "b"] });
    apply([{ type: "jobRemoved", data: "a" }]);
    expect(Object.keys(useApp.getState().jobs)).toEqual(["b"]);
    expect(useApp.getState().selection).toEqual(["b"]);
  });

  it("holds a just-finished job for a beat", () => {
    apply([{ type: "jobAdded", data: job({ id: "a", state: "postProcessing" }) }]);
    apply([{ type: "jobUpdated", data: job({ id: "a", state: "completed" }) }]);
    expect(useApp.getState().justCompleted).toEqual(["a"]);
    // Another update in the same state does not restart the beat.
    apply([{ type: "jobUpdated", data: job({ id: "a", state: "completed", title: "x" }) }]);
    expect(useApp.getState().justCompleted).toEqual(["a"]);
    vi.advanceTimersByTime(SETTLE_MS);
    expect(useApp.getState().justCompleted).toEqual([]);
  });

  it("opens the playlist sheet when a playlist preview arrives and streams its entries", () => {
    const entry = {
      id: "e1",
      url: "https://x.test/1",
      title: "One",
      uploader: "",
      duration: null,
      thumbnail: "",
      downloaded: false,
    };
    const request = job().request;
    apply([
      {
        type: "preview",
        data: {
          id: "p",
          url: "https://x.test/pl",
          source: "paste",
          request,
          status: { status: "playlist", title: "PL", uploader: "", entries: [entry], complete: false },
        },
      },
    ]);
    expect(useApp.getState().sheet).toEqual({ kind: "playlist" });
    apply([{ type: "playlistEntries", data: { previewId: "p", entries: [{ ...entry, id: "e2" }] } }]);
    const status = useApp.getState().preview?.status;
    expect(status?.status === "playlist" && status.entries.map((e) => e.id)).toEqual(["e1", "e2"]);
  });

  it("announces finished downloads to screen readers", () => {
    apply([{ type: "notice", data: { kind: "finished", jobId: "a", title: "Llamigos", path: "C:\\x.mp4" } }]);
    expect(useApp.getState().announcement).toBe("Downloaded Llamigos");
  });
});

describe("accent colors", () => {
  const white: [number, number, number] = [255, 255, 255];

  it("darkens a light accent until white text on it is readable", () => {
    const yellow: [number, number, number] = [255, 200, 0];
    expect(contrast(yellow, white)).toBeLessThan(4.5);
    expect(contrast(solidAccent(yellow), white)).toBeGreaterThanOrEqual(4.5);
  });

  it("finds a readable text shade for both themes", () => {
    const blue: [number, number, number] = [0, 120, 212];
    const light: [number, number, number] = [0xe4, 0xe4, 0xe4];
    const dark: [number, number, number] = [0x3a, 0x3a, 0x3a];
    expect(contrast(readableOn(blue, light), light)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(readableOn(blue, dark), dark)).toBeGreaterThanOrEqual(4.5);
  });
});
