// Mock mode: the UI in a plain browser, answered by a scripted stand-in for the engine.
// Open http://localhost:5173/?scenario=busy (or empty, preview, playlist, errors, history,
// onboarding, quit, ambiguous). Add &frozen=1 to stop the simulated progress for screenshots.

import { mockCommands } from "./bindings";
import type {
  Change,
  Components,
  Job,
  MediaInfo,
  PlaylistEntry,
  Preview,
  Progress,
  Resolved,
  Settings,
  Snapshot,
} from ".";
import { events } from ".";

const params = new URLSearchParams(location.search);
const scenario = params.get("scenario") ?? "busy";
const frozen = params.has("frozen");

// Thumbnails: small painted scenes, so the content carries the color as real thumbnails do.
const palettes: [string, string, string, string][] = [
  ["#f6c58f", "#e9785b", "#5a3d5c", "#2d2240"],
  ["#bfe3f0", "#6fb3d2", "#2f6f8f", "#123047"],
  ["#d8f0c2", "#8cc56b", "#3f7a3a", "#1d3b22"],
  ["#f7e7a1", "#f2b134", "#c0562b", "#4b1d1f"],
  ["#e3d3f5", "#a98bd9", "#5b4a9e", "#241c46"],
  ["#ffd6dc", "#ff8fa3", "#c44569", "#3d1428"],
];

function scene(seed: number): string {
  const [sky, glow, hill, ground] = palettes[seed % palettes.length] ?? ["#ccc", "#999", "#666", "#333"];
  const sun = 30 + ((seed * 37) % 100);
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 90"><rect width="160" height="90" fill="${sky}"/><circle cx="${sun}" cy="34" r="14" fill="${glow}"/><path d="M0 62 Q40 40 80 58 T160 50 V90 H0Z" fill="${hill}"/><path d="M0 74 Q50 60 100 76 T160 70 V90 H0Z" fill="${ground}"/></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

const now = Date.now();
const settings: Settings = {
  version: 1,
  onboardingDone: scenario !== "onboarding",
  downloadDir: "C:\\Users\\you\\Downloads",
  onAdd: "preview",
  defaultRequest: {
    preset: { kind: "mp4", maxHeight: null },
    formats: null,
    options: {
      subtitles: false,
      subLangs: ["en"],
      autoSubs: false,
      embedSubs: true,
      embedThumbnail: true,
      embedMetadata: true,
      embedChapters: true,
      sponsorblock: "off",
      sponsorblockCategories: ["sponsor"],
      section: null,
      preciseCuts: false,
      liveFromStart: false,
      waitForLive: false,
    },
    destination: null,
  },
  maxConcurrent: 3,
  maxPerSite: 2,
  filenameTemplate: "%(title).200B [%(id)s].%(ext)s",
  playlistIndex: true,
  skipDownloaded: true,
  rateLimit: "",
  proxy: "",
  paceRequests: false,
  cookiesBrowser: "",
  ytdlpChannel: "stable",
  useSystemTools: false,
  autoUpdateTools: true,
  customYtdlp: "",
  customFfmpegDir: "",
  customJsRuntime: "",
  customArgsEnabled: false,
  customArgs: [],
  concurrentFragments: 1,
  notifications: true,
  noticeCopiedLinks: false,
  trayIcon: false,
  keepAwake: true,
  autoUpdateApp: true,
  sidebarVisible: true,
  lastToolCheck: now,
};

const tool = (id: "ytdlp" | "ffmpeg" | "deno", version: string): Components["ytdlp"] => ({
  id,
  status: scenario === "onboarding" ? { status: "missing" } : { status: "ready" },
  version: scenario === "onboarding" ? "" : version,
  latest: "",
  path: "",
  origin: "managed",
});

const components: Components = {
  ytdlp: tool("ytdlp", "2026.10.26"),
  ffmpeg: tool("ffmpeg", "9.0.2"),
  deno: tool("deno", "2.9.8"),
};

const progress = (p: Partial<Progress>): Progress => ({
  stage: "waiting",
  stageIndex: 0,
  stageCount: 0,
  fraction: null,
  downloaded: 0,
  total: null,
  speed: null,
  eta: null,
  ...p,
});

let seq = 0;
function job(title: string, uploader: string, site: string, p: Partial<Job> = {}): Job {
  seq += 1;
  return {
    id: `j${seq}`,
    url: `https://www.${site}/watch?v=mock${seq}`,
    source: "paste",
    title,
    uploader,
    duration: 60 + seq * 47,
    site,
    thumbnail: scene(seq),
    kind: "video",
    request: settings.defaultRequest,
    state: "queued",
    progress: progress({}),
    planned: null,
    output: "",
    fileMissing: false,
    bytes: null,
    error: null,
    attempts: 0,
    group: null,
    cleared: false,
    createdAt: now - seq * 60000,
    startedAt: null,
    finishedAt: null,
    ...p,
  };
}

const MB = 1_000_000;
const titles = [
  ["Tidal pools of the Pacific coast, filmed over one low tide", "Shoreline Studio", "youtube.com"],
  ["How a mechanical watch keeps time", "Bench & Loupe", "youtube.com"],
  ["Lecture 7: Graph algorithms", "Open Course Archive", "vimeo.com"],
  ["Night train through the Alps in winter", "Slow Rail", "youtube.com"],
  ["Field recording: rain on a tin roof", "Quiet Sounds", "soundcloud.com"],
  ["Building a cedar canoe, part 3", "Workshop Notes", "youtube.com"],
  ["The history of the paperback", "Margins", "youtube.com"],
  ["Morning market in Hanoi", "Street Level", "vimeo.com"],
] as const;

function busyJobs(): Job[] {
  const [a, b, c, d, e, f, g, h] = titles;
  return [
    job(a[0], a[1], a[2], {
      state: "downloading",
      progress: progress({
        stage: "video",
        stageIndex: 1,
        stageCount: 2,
        fraction: 0.62,
        downloaded: 193 * MB,
        total: 312 * MB,
        speed: 3.1 * MB,
        eta: 41,
      }),
      planned: null,
    }),
    job(b[0], b[1], b[2], {
      state: "postProcessing",
      progress: progress({ stage: "merging", fraction: 1, downloaded: 88 * MB }),
    }),
    job(c[0], c[1], c[2], {
      state: "downloading",
      progress: progress({
        stage: "media",
        stageIndex: 1,
        stageCount: 1,
        fraction: 0.18,
        downloaded: 41 * MB,
        total: 228 * MB,
        speed: 1.4 * MB,
        eta: 133,
      }),
    }),
    job(d[0], d[1], d[2]),
    job(e[0], e[1], e[2], { kind: "audio", thumbnail: "" }),
    job(f[0], f[1], f[2], {
      state: "paused",
      progress: progress({ downloaded: 54 * MB, total: 180 * MB, fraction: 0.3 }),
    }),
    job(g[0], g[1], g[2], {
      state: "completed",
      output: "C:\\Users\\you\\Downloads\\The history of the paperback [mock7].mp4",
      bytes: 146 * MB,
      progress: progress({ stage: "done", fraction: 1 }),
      finishedAt: now - 120000,
    }),
    job(h[0], h[1], h[2], {
      state: "failed",
      error: {
        kind: "http403",
        fix: "updateYtdlp",
        retryable: true,
        detail: "[vimeo] 8812: Unable to download video data: HTTP Error 403: Forbidden",
      },
      finishedAt: now - 60000,
    }),
  ];
}

function errorJobs(): Job[] {
  const failed = (kind: Job["error"], title: string, site: string) =>
    job(title, "Someone", site, { state: "failed", error: kind, finishedAt: now - 1000 });
  return [
    failed(
      {
        kind: "signIn",
        fix: "useCookies",
        retryable: false,
        detail: "[youtube] x: Sign in to confirm you're not a bot",
      },
      "Members livestream archive",
      "youtube.com",
    ),
    failed(
      {
        kind: "formatUnavailable",
        fix: "chooseFormat",
        retryable: false,
        detail: "Requested format is not available",
      },
      "Concert in 8K",
      "youtube.com",
    ),
    failed(
      { kind: "drm", fix: null, retryable: false, detail: "This video is DRM protected" },
      "A film from a streaming service",
      "example.com",
    ),
    failed(
      { kind: "unknown", fix: "retry", retryable: true, detail: "ERROR: something nobody has seen before" },
      "Unusual page",
      "example.org",
    ),
  ];
}

function historyJobs(): Job[] {
  return titles.map(([title, uploader, site], i) =>
    job(title, uploader, site, {
      state: i === 4 ? "failed" : "completed",
      kind: i === 4 ? "audio" : "video",
      output: i === 4 ? "" : `C:\\Users\\you\\Videos\\${title}.mp4`,
      bytes: (40 + i * 33) * MB,
      fileMissing: i === 2,
      cleared: true,
      error:
        i === 4
          ? { kind: "network", fix: "retry", retryable: true, detail: "The site could not be reached" }
          : null,
      finishedAt: now - i * 3_600_000 * 7,
    }),
  );
}

const formats = (): MediaInfo["formats"] => {
  const f = (
    id: string,
    ext: string,
    height: number | null,
    vcodec: string,
    acodec: string,
    tbr: number,
    mb: number,
  ) => ({
    id,
    ext,
    width: height ? Math.round((height * 16) / 9) : null,
    height,
    fps: height ? 24 : null,
    vcodec,
    acodec,
    tbr,
    bytes: mb * MB,
    bytesApprox: false,
    note: height ? `${height}p` : "medium",
    dynamicRange: height ? "SDR" : "",
    language: acodec ? "en" : "",
    protocol: "https",
  });
  return [
    f("140", "m4a", null, "", "mp4a.40.2", 129, 2.4),
    f("251", "webm", null, "", "opus", 131, 2.5),
    f("135", "mp4", 480, "avc1.4D401E", "", 360, 6.8),
    f("136", "mp4", 720, "avc1.4D401F", "", 617, 11.6),
    f("137", "mp4", 1080, "avc1.640028", "", 2003, 37.6),
    f("248", "webm", 1080, "vp9", "", 1478, 27.7),
    f("399", "mp4", 1080, "av01.0.08M.08", "", 1278, 24),
    f("313", "webm", 2160, "vp9", "", 9800, 184),
  ];
};

function info(): MediaInfo {
  return {
    id: "mock",
    title: "Tidal pools of the Pacific coast, filmed over one low tide",
    uploader: "Shoreline Studio",
    duration: 761,
    site: "youtube.com",
    thumbnail: scene(1),
    webpageUrl: "https://www.youtube.com/watch?v=mock",
    live: "notLive",
    releaseAt: null,
    ageRestricted: false,
    membersOnly: false,
    drm: false,
    formats: formats(),
    subtitles: ["en", "fr", "ja"],
    autoCaptions: true,
    chapters: 6,
  };
}

function resolve(request: Settings["defaultRequest"]): Resolved {
  const p = request.preset;
  if (p.kind === "audio") {
    return {
      exact: true,
      formatIds: ["140"],
      height: null,
      fps: null,
      vcodec: "",
      acodec: "mp4a.40.2",
      ext: p.format,
      bytes: 2.4 * MB,
      bytesApprox: p.format !== "m4a",
      sourceHeight: 2160,
    };
  }
  const h = Math.min(p.maxHeight ?? 2160, p.kind === "mp4" ? 1080 : 2160);
  const mp4 = p.kind === "mp4";
  return {
    exact: true,
    formatIds: [mp4 ? "137" : "313", "140"],
    height: h,
    fps: 24,
    vcodec: mp4 ? "avc1.640028" : "vp9",
    acodec: "mp4a.40.2",
    ext: mp4 ? "mp4" : "mkv",
    bytes: (h / 1080) ** 2 * 40 * MB,
    bytesApprox: false,
    sourceHeight: 2160,
  };
}

function playlist(): PlaylistEntry[] {
  return titles.concat(titles).map(([title, uploader], i) => ({
    id: `pl${i}`,
    url: `https://www.youtube.com/watch?v=pl${i}`,
    title: i < titles.length ? title : `${title} (extended)`,
    uploader,
    duration: 120 + i * 61,
    thumbnail: scene(i + 2),
    downloaded: i === 3,
  }));
}

const state: Snapshot = {
  jobs: scenario === "busy" || scenario === "quit" ? busyJobs() : scenario === "errors" ? errorJobs() : [],
  preview: null,
  settings,
  components,
  system: { accent: params.get("accent") ?? "#0067c0", material: false, appVersion: "0.1.0" },
};
const history = scenario === "history" ? historyJobs() : busyJobs().filter((j) => j.state === "completed");

if (scenario === "preview") {
  state.preview = {
    id: "p1",
    url: "https://www.youtube.com/watch?v=mock",
    source: "paste",
    request: settings.defaultRequest,
    status: { status: "ready", info: info(), resolved: resolve(settings.defaultRequest) },
  };
} else if (scenario === "ambiguous") {
  state.preview = {
    id: "p1",
    url: "https://www.youtube.com/watch?v=mock&list=PL1",
    source: "paste",
    request: settings.defaultRequest,
    status: { status: "ambiguous" },
  };
} else if (scenario === "playlist") {
  state.preview = {
    id: "p1",
    url: "https://www.youtube.com/playlist?list=PL1",
    source: "paste",
    request: settings.defaultRequest,
    status: {
      status: "playlist",
      title: "Slow travel, collected",
      uploader: "Slow Rail",
      entries: playlist(),
      complete: true,
    },
  };
} else if (scenario === "probing") {
  state.preview = {
    id: "p1",
    url: "https://www.youtube.com/watch?v=mock",
    source: "paste",
    request: settings.defaultRequest,
    status: { status: "probing" },
  };
}

function emit(changes: Change[]) {
  void events.update.emit(changes);
}

function find(id: string): Job | undefined {
  return state.jobs.find((j) => j.id === id);
}

function update(id: string, patch: Partial<Job>) {
  const j = find(id);
  if (!j) return;
  Object.assign(j, patch);
  emit([{ type: "jobUpdated", data: { ...j } }]);
  ensureTicking();
}

// Simulated progress, only while something is active. Nothing ticks when nothing happens.
let timer: number | undefined;
function ensureTicking() {
  if (frozen || timer !== undefined) return;
  timer = window.setInterval(() => {
    const active = state.jobs.filter((j) => j.state === "downloading" || j.state === "postProcessing");
    if (active.length === 0) {
      window.clearInterval(timer);
      timer = undefined;
      return;
    }
    const changes: Change[] = [];
    for (const j of active) {
      if (j.state === "postProcessing") {
        if (Math.random() < 0.08) {
          Object.assign(j, {
            state: "completed",
            bytes: j.progress.total ?? 120 * MB,
            finishedAt: Date.now(),
            progress: progress({ stage: "done", fraction: 1 }),
            output: `C:\\Users\\you\\Downloads\\${j.title}.mp4`,
          });
          changes.push(
            { type: "jobUpdated", data: { ...j } },
            { type: "notice", data: { kind: "finished", jobId: j.id, title: j.title, path: j.output } },
          );
        }
        continue;
      }
      const total = j.progress.total ?? 200 * MB;
      const speed = (1 + Math.random()) * 1.6 * MB;
      const downloaded = Math.min(total, (j.progress.downloaded ?? 0) + speed / 4);
      j.progress = {
        ...j.progress,
        stage: j.progress.stage === "waiting" ? "media" : j.progress.stage,
        downloaded,
        total,
        speed,
        eta: (total - downloaded) / speed,
        fraction: downloaded / total,
      };
      if (downloaded >= total) {
        Object.assign(j, {
          state: "postProcessing",
          progress: progress({ stage: "merging", fraction: 1, downloaded, total }),
        });
        changes.push({ type: "jobUpdated", data: { ...j } });
      } else {
        changes.push({ type: "progress", data: { id: j.id, progress: j.progress } });
      }
    }
    // Start the next queued job when a slot is free.
    const running = state.jobs.filter((j) =>
      ["downloading", "postProcessing", "probing"].includes(j.state),
    ).length;
    const next = state.jobs.find((j) => j.state === "queued");
    if (next && running < settings.maxConcurrent) {
      Object.assign(next, {
        state: "downloading",
        startedAt: Date.now(),
        progress: progress({ stage: "media", stageIndex: 1, stageCount: 1, fraction: 0, total: 150 * MB }),
      });
      changes.push({ type: "jobUpdated", data: { ...next } });
    }
    if (changes.length) emit(changes);
  }, 250);
}

function setPreview(preview: Preview | null) {
  state.preview = preview;
  emit([{ type: "preview", data: preview }]);
}

const ok = null;

mockCommands({
  getSnapshot: () => {
    ensureTicking();
    return structuredClone(state);
  },
  addLinks: ({ text }) => {
    const links = text.match(/https?:\/\/\S+/g) ?? [];
    if (links.length === 0)
      throw new Error("No link found. Paste a web address that starts with http or https.");
    if (links.length > 1) {
      for (const link of links) {
        const j = job("", "", new URL(link).hostname.replace(/^www\./, ""), { url: link, title: "" });
        state.jobs.push(j);
        emit([{ type: "jobAdded", data: { ...j } }]);
      }
      ensureTicking();
      return links.length;
    }
    const request = settings.defaultRequest;
    setPreview({
      id: `p${Date.now()}`,
      url: links[0] ?? "",
      source: "paste",
      request,
      status: { status: "probing" },
    });
    window.setTimeout(() => {
      if (!state.preview) return;
      setPreview({
        ...state.preview,
        status: { status: "ready", info: info(), resolved: resolve(state.preview.request) },
      });
    }, 700);
    return 1;
  },
  previewUpdate: ({ request }) => {
    const p = state.preview;
    if (p?.status.status !== "ready") return;
    const full = { ...settings.defaultRequest, ...request } as Settings["defaultRequest"];
    setPreview({ ...p, request: full, status: { ...p.status, resolved: resolve(full) } });
  },
  previewChoose: ({ choice }) => {
    if (!state.preview) return;
    const p = state.preview;
    setPreview({
      ...p,
      status:
        choice === "video"
          ? { status: "ready", info: info(), resolved: resolve(p.request) }
          : {
              status: "playlist",
              title: "Slow travel, collected",
              uploader: "Slow Rail",
              entries: playlist(),
              complete: true,
            },
    });
  },
  previewDismiss: () => setPreview(null),
  previewStart: ({ selection }) => {
    const p = state.preview;
    if (!p) return ok;
    const added: Job[] = [];
    if (p.status.status === "ready") {
      const i = p.status.info;
      added.push(
        job(i.title, i.uploader, i.site, {
          thumbnail: i.thumbnail,
          planned: p.status.resolved,
          request: p.request,
        }),
      );
    } else if (p.status.status === "playlist") {
      for (const e of p.status.entries.filter((e) => selection.includes(e.id)))
        added.push(job(e.title, e.uploader, "youtube.com", { thumbnail: e.thumbnail }));
    }
    state.jobs.push(...added);
    setPreview(null);
    emit(added.map((j) => ({ type: "jobAdded", data: { ...j } })));
    ensureTicking();
    return ok;
  },
  jobPause: ({ id }) =>
    update(id, {
      state: "paused",
      progress: { ...(find(id)?.progress ?? progress({})), speed: null, eta: null },
    }),
  jobResume: ({ id }) => update(id, { state: "queued" }),
  jobCancel: ({ id }) => update(id, { state: "canceled" }),
  jobRetry: ({ id }) => update(id, { state: "queued", error: null, attempts: (find(id)?.attempts ?? 0) + 1 }),
  jobRemove: ({ ids }) => {
    state.jobs = state.jobs.filter((j) => !ids.includes(j.id));
    emit(ids.map((id) => ({ type: "jobRemoved", data: id })));
  },
  jobApplyFix: ({ id }) => update(id, { state: "queued", error: null }),
  jobUpdateRequest: () => ok,
  jobFormats: () => info(),
  jobsReorder: ({ ids }) => {
    state.jobs.sort((a, b) => ids.indexOf(a.id) - ids.indexOf(b.id));
    emit([{ type: "order", data: state.jobs.map((j) => j.id) }]);
  },
  pauseAll: () =>
    state.jobs
      .filter((j) => ["downloading", "queued", "probing", "postProcessing"].includes(j.state))
      .forEach((j) => update(j.id, { state: "paused" })),
  resumeAll: () =>
    state.jobs.filter((j) => j.state === "paused").forEach((j) => update(j.id, { state: "queued" })),
  retryFailed: () =>
    state.jobs
      .filter((j) => j.state === "failed" && j.error?.retryable)
      .forEach((j) => update(j.id, { state: "queued", error: null })),
  clearCompleted: () => {
    const ids = state.jobs.filter((j) => j.state === "completed").map((j) => j.id);
    state.jobs = state.jobs.filter((j) => !ids.includes(j.id));
    emit(ids.map((id) => ({ type: "jobRemoved", data: id })));
  },
  historyQuery: ({ query, filter, offset, limit }) => {
    const q = query.toLowerCase();
    const items = history.filter(
      (j) =>
        (filter === "all" ||
          (filter === "failed" ? j.state === "failed" : j.state === "completed" && j.kind === filter)) &&
        [j.title, j.uploader, j.site, j.url].some((t) => t.toLowerCase().includes(q)),
    );
    return { items: items.slice(offset, offset + limit), total: items.length };
  },
  historyRemove: () => undefined,
  historyTrash: () => ok,
  downloadAgain: () => ok,
  reveal: () => ok,
  openFile: () => ok,
  openFolder: () => ok,
  copyLink: () => ok,
  copyText: () => ok,
  jobDetails: ({ id }) => `lovelyDL/0.1.0 (mock)\nyt-dlp 2026.10.26\n\nJob ${id}`,
  settingsSet: ({ settings: next }) => {
    Object.assign(settings, next);
    emit([{ type: "settings", data: { ...settings } }]);
    return ok;
  },
  chooseFolder: () => "D:\\Media",
  checkFolder: () => ok,
  templatePreview: ({ template }) =>
    template
      .replace("%(title).200B", "Tidal pools of the Pacific coast")
      .replace("%(id)s", "mock")
      .replace("%(ext)s", "mp4"),
  componentsRefresh: () => undefined,
  componentInstall: () => ok,
  componentsInstallMissing: async () => {
    for (const id of ["ytdlp", "ffmpeg", "deno"] as const) {
      for (let f = 0; f <= 1; f += 0.1) {
        components[id] = { ...components[id], status: { status: "installing", fraction: f } };
        emit([{ type: "components", data: structuredClone(components) }]);
        await new Promise((r) => setTimeout(r, 60));
      }
      components[id] = {
        ...components[id],
        status: { status: "ready" },
        version: id === "ytdlp" ? "2026.10.26" : id === "ffmpeg" ? "9.0.2" : "2.9.8",
      };
      emit([{ type: "components", data: structuredClone(components) }]);
    }
    return ok;
  },
  componentRollback: () => ok,
  clipboardTakeLink: () => 1,
  clipboardDismiss: () => undefined,
  windowAction: () => ok,
  windowLabel: () => (params.get("view") === "settings" ? "settings" : "main"),
  quit: () => ok,
  openSettings: () => {
    window.open(`${location.pathname}?view=settings`, "_blank", "width=700,height=560");
    return ok;
  },
  openLink: () => ok,
  appUpdateCheck: () => null,
  appUpdateInstall: () => ok,
});

if (scenario === "quit") {
  window.setTimeout(() => emit([{ type: "command", data: "confirmQuit" }]), 50);
}
if (scenario === "clipboard") {
  window.setTimeout(() => emit([{ type: "command", data: "checkClipboard" }]), 50);
}
