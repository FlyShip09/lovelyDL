// Records the real lovelyDL interface for the ad.
//
// The app's UI runs in a plain browser against its scripted mock (ui/src/ipc/mock.ts). This
// script drives it with Playwright on a paused clock, one video frame at a time, so the
// recording is exact and repeatable: the app's own springs, progress bars and hover states,
// at 30 frames a second. Identical frames are stored once.
//
//   npm ci                                   (repo root; brings Playwright and the UI)
//   npx vite --port 5199 --strictPort        (repo root; leave running)
//   node promo/capture.mjs                   (needs ffmpeg on PATH)
//
// Writes promo/public/captures/ (WebP frames and PNG stills, committed) and
// promo/src/captures.json (which frame shows when, and where the pointer clicks).

import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const BASE = process.env.LOVELYDL_UI ?? "http://localhost:5199";
const FPS = 30;
const VIEW = { width: 1000, height: 660 };
const SCALE = 2;
const LINK = "https://www.example.com/watch?v=tidal-pools";

const here = (p) => fileURLToPath(new URL(p, import.meta.url));
const RAW = here("./captures-raw/"); // git-ignored
const PUB = here("./public/captures/");
for (const dir of [RAW, PUB]) rmSync(dir, { recursive: true, force: true });
for (const dir of [RAW, `${PUB}app/`]) mkdirSync(dir, { recursive: true });
copyFileSync(here("../docs/icon/icon.svg"), `${PUB}icon.svg`); // one source for the icon

const browser = await chromium.launch();
const context = await browser.newContext({
  viewport: VIEW,
  deviceScaleFactor: SCALE,
  colorScheme: "light",
  reducedMotion: "no-preference",
});

// ----- the recording: first run, then one download ----------------------------------------

const page = await context.newPage();
const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(e.message));
// Without element.animate() the app's motion library animates in script, on the clock this
// script steps. (Browser-run animations would keep real time and drift from the frames.)
await page.addInitScript(() => delete Element.prototype.animate);
await page.clock.install({ time: new Date("2026-11-17T10:24:00") });
await page.goto(`${BASE}/?scenario=onboarding`);
await page.waitForSelector("#root > *");
await page.waitForTimeout(500);
await page.clock.pauseAt((await page.evaluate(() => Date.now())) + 100);

let n = 0; // the frame about to be recorded
let clockMs = 0;
const frames = []; // frame -> index of its picture
const pictures = new Map(); // hash -> index
const events = []; // what the pointer and keyboard do, for the ad to act out
const marks = {}; // named moments

// React schedules its work through MessageChannel, which the paused clock leaves alone.
const settle = () =>
  page.evaluate(
    () =>
      new Promise((done) => {
        const channel = new MessageChannel();
        channel.port1.onmessage = () => done();
        channel.port2.postMessage(0);
      }),
  );

// CSS transitions and keyframes (hover fades, the spinner) are stepped by hand as well.
const stepCss = (ms) =>
  page.evaluate((ms) => {
    const seen = (window.__stepped ??= new Map());
    for (const animation of document.getAnimations()) {
      if (animation.playState === "finished" || animation.playState === "idle") continue;
      // A new one began within this frame. (Starting it at zero would freeze a transition whose
      // target moves every frame, such as the thumbnail filling with color.)
      const at = (seen.get(animation) ?? 0) + ms;
      seen.set(animation, at);
      const end = animation.effect?.getComputedTiming().endTime ?? 0;
      if (Number.isFinite(end) && at >= end) animation.finish();
      else {
        animation.pause();
        animation.currentTime = at;
      }
    }
  }, ms);

/** Records `count` frames; `speed` runs the app's clock faster than the video's. */
async function record(count = 1, speed = 1) {
  for (let i = 0; i < count; i++) {
    const ms = Math.round(((n + 1) * 1000) / FPS) - Math.round((n * 1000) / FPS);
    await settle();
    await page.clock.runFor(Math.round(ms * speed));
    clockMs += ms * speed;
    await settle();
    await stepCss(ms * speed);
    await patchPreview();
    const png = await page.screenshot({ omitBackground: true });
    const hash = createHash("sha1").update(png).digest("hex");
    if (!pictures.has(hash)) {
      pictures.set(hash, pictures.size);
      writeFileSync(`${RAW}u${String(pictures.size - 1).padStart(4, "0")}.png`, png);
    }
    frames.push(pictures.get(hash));
    n++;
  }
}

const center = async (locator) => {
  const box = await locator.boundingBox();
  if (!box) throw new Error(`Nothing to point at: ${locator}`);
  return { x: Math.round(box.x + box.width / 2), y: Math.round(box.y + box.height / 2) };
};

/** The pointer travels for `travel` frames, hovers, presses and lets go. */
async function click(locator, travel = 18, at = null) {
  const { x, y } = at ?? (await center(locator));
  await record(travel);
  await page.mouse.move(x, y);
  const arrive = n;
  await record(3);
  await page.mouse.down();
  const down = n;
  await record(3);
  await page.mouse.up();
  events.push({ type: "click", travel, arrive, down, up: n, x, y });
}

async function moveTo(x, y, travel = 16) {
  await record(travel);
  await page.mouse.move(x, y);
  events.push({ type: "move", travel, arrive: n, x, y });
}

const key = (label) => events.push({ type: "key", frame: n, label });
const mark = (name) => (marks[name] = n);
const button = (name) => page.getByRole("button", { name, exact: true });

// The mock answers every link as the same video on a real site. The ad shows a neutral one.
async function patchPreview(p = page) {
  await p.evaluate(async (link) => {
    const { events } = await import("/src/ipc/index.ts");
    const { useApp } = await import("/src/store/app.ts");
    const preview = useApp.getState().preview;
    if (preview?.status.status !== "ready" || preview.status.info.site === "example.com") return;
    const info = { ...preview.status.info, site: "example.com", webpageUrl: link };
    await events.update.emit([{ type: "preview", data: { ...preview, status: { ...preview.status, info } } }]);
  }, LINK);
  if (p === page) await settle();
}

// First run.
mark("welcome");
await record(18);
await click(button("Continue"), 22);
mark("folder");
await record(20);
await click(button("Continue"), 16);
mark("tools");
await record(18);
await click(button("Install"), 16);
mark("installing");
// The mock installs three tools in two seconds; the ad watches at one and a half speed.
while (!(await button("Continue").isVisible())) {
  await record(1, 1.5);
  if (n - marks.installing > 120) throw new Error("The tools never finished installing.");
}
mark("installed");
await record(8);
await click(button("Continue"), 14);
mark("done");
await record(20);
await click(button("Start using lovelyDL"), 14);
mark("main");
await record(10);

// Paste a link, read the preview, download.
const field = page.locator("[data-url-field]");
await click(field, 18, { x: 330, y: (await center(field)).y });
mark("field");
await record(6);
key("Ctrl + V");
await page.keyboard.insertText(LINK);
mark("pasted");
await record(18);
key("Enter");
await field.press("Enter");
mark("entered");
await record(8);
while (!(await button("Download").isVisible())) {
  await record(1);
  if (n - marks.entered > 90) throw new Error("The preview never arrived.");
}
mark("preview");
await record(38);
await click(button("Download"), 18);
mark("started");
await record(2);

// From here the script plays the engine: the mock's own queue only moves in its busy scene.
await page.evaluate(async () => {
  const { events } = await import("/src/ipc/index.ts");
  const { useApp } = await import("/src/store/app.ts");
  const palettes = [
    ["#d8f0c2", "#8cc56b", "#3f7a3a", "#1d3b22"],
    ["#e3d3f5", "#a98bd9", "#5b4a9e", "#241c46"],
  ];
  // Painted like the mock's own thumbnails (ui/src/ipc/mock.ts).
  const scene = ([sky, glow, hill, ground], sun) =>
    `data:image/svg+xml,${encodeURIComponent(
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 90"><rect width="160" height="90" fill="${sky}"/><circle cx="${sun}" cy="34" r="14" fill="${glow}"/><path d="M0 62 Q40 40 80 58 T160 50 V90 H0Z" fill="${hill}"/><path d="M0 74 Q50 60 100 76 T160 70 V90 H0Z" fill="${ground}"/></svg>`,
    )}`;
  const first = { ...Object.values(useApp.getState().jobs)[0], site: "example.com" };
  const more = [
    { id: "ad2", title: "How a mechanical watch keeps time", uploader: "Bench & Loupe", site: "example.org", thumbnail: scene(palettes[0], 104), duration: 512 },
    { id: "ad3", title: "Night train through the Alps in winter", uploader: "Slow Rail", site: "example.net", thumbnail: scene(palettes[1], 58), duration: 1904 },
  ].map((j) => ({ ...first, ...j, url: `https://www.${j.site}/watch?v=${j.id}` }));
  const emit = (changes) => events.update.emit(changes);
  void emit([{ type: "jobUpdated", data: first }]);
  const totals = { [first.id]: 40e6, ad2: 26.4e6, ad3: 18.2e6 };
  const job = (id) => useApp.getState().jobs[id];
  window.__ad = {
    first: first.id,
    add: () => emit(more.map((data) => ({ type: "jobAdded", data }))),
    /** `fraction` of the way through; null finishes the file. */
    progress: (id, fraction, speed, eta) => {
      const total = totals[id];
      if (fraction === null) {
        return emit([
          {
            type: "jobUpdated",
            data: {
              ...job(id),
              state: "completed",
              bytes: total,
              finishedAt: Date.now(),
              output: `C:\\Users\\you\\Downloads\\${job(id).title}.mp4`,
              progress: { ...job(id).progress, stage: "done", fraction: 1, speed: null, eta: null },
            },
          },
        ]);
      }
      const progress = {
        stage: "media",
        stageIndex: 1,
        stageCount: 1,
        fraction,
        downloaded: fraction * total,
        total,
        speed,
        eta: eta ?? ((1 - fraction) * total) / speed,
      };
      if (job(id).state === "downloading") return emit([{ type: "progress", data: { id, progress } }]);
      return emit([{ type: "jobUpdated", data: { ...job(id), state: "downloading", startedAt: Date.now(), progress } }]);
    },
  };
});

// Each download: when it starts and how long it takes, in frames after the click. The last
// row finishes first and the first row last, so finished rows never change places.
const plan = [
  { id: await page.evaluate(() => window.__ad.first), from: 12, frames: 70, total: 40e6 },
  { id: "ad2", from: 34, frames: 42, total: 26.4e6 },
  { id: "ad3", from: 38, frames: 32, total: 18.2e6 },
];
const ADD_AT = 26;
const lastDone = Math.max(...plan.map((p) => p.from + p.frames));
for (let k = 0; k <= lastDone + 24; k++) {
  if (k === 4) {
    // The pointer gets out of the way of the rows.
    await page.mouse.move(700, 400);
    events.push({ type: "move", travel: 16, arrive: n + 16, x: 700, y: 400 });
  }
  if (k === ADD_AT) {
    key("Ctrl + V");
    mark("pastedMore");
    await page.evaluate(() => window.__ad.add());
  }
  for (const p of plan) {
    const t = (k - p.from) / p.frames;
    if (t < 0 || t > 1) continue;
    // Mostly steady, with a slow start, as a real transfer has.
    const eased = (x) => 0.75 * x + 0.25 * x * x * (3 - 2 * x);
    // Shown as a time-lapse: a fifth of the speed the moving bar implies.
    // Speed and time left hold still for the last moments, so the row drops them the instant
    // the file is done (the app keeps a changing number on screen for half a second).
    const calm = Math.min(k, p.from + p.frames - 20);
    const u = (calm - p.from) / p.frames;
    const perSecond = ((eased(u + 1 / p.frames) - eased(u)) * p.total * FPS) / 5;
    const speed = Math.max(0.4e6, perSecond * (1 + 0.12 * Math.sin(calm / 3 + p.from)));
    await page.evaluate(([id, f, s, eta]) => window.__ad.progress(id, f, s, eta), [
      p.id,
      t >= 1 ? null : eased(t),
      speed,
      k > calm ? 1 : undefined,
    ]);
  }
  if (k === lastDone) mark("allDone");
  await record(1);
}
mark("end");

const completed = await page.evaluate(async () => {
  const { useApp } = await import("/src/store/app.ts");
  return Object.values(useApp.getState().jobs).filter((j) => j.state === "completed").length;
});
await page.close();

// ----- stills for the feature beats and the social preview --------------------------------

// A context of their own: the paused clock above belongs to the whole first context.
const calm = await browser.newContext({
  viewport: VIEW,
  deviceScaleFactor: SCALE,
  colorScheme: "light",
  reducedMotion: "reduce",
});
const stills = {};
async function still(name, url, then) {
  const p = await calm.newPage();
  p.on("pageerror", (e) => pageErrors.push(e.message));
  await p.goto(`${BASE}${url}`);
  await p.waitForSelector("#root > *");
  await then?.(p);
  await p.waitForTimeout(700); // springs and fades settle
  await p.screenshot({ path: `${PUB}${name}.png`, omitBackground: true });
  stills[name] = `captures/${name}.png`;
  await p.close();
}
await still("playlist", "/?scenario=playlist&frozen=1");
await still("formats", "/?scenario=preview&frozen=1", async (p) => {
  await patchPreview(p);
  await p.getByRole("button", { name: /^Format/ }).click();
});
await still("queue", "/?scenario=busy&frozen=1", (p) =>
  // An ad has no use for the scene's failed download.
  p.evaluate(async () => {
    const { events } = await import("/src/ipc/index.ts");
    const { useApp } = await import("/src/store/app.ts");
    const failed = Object.values(useApp.getState().jobs).filter((j) => j.state === "failed");
    await events.update.emit(failed.map((j) => ({ type: "jobRemoved", data: j.id })));
  }),
);
await browser.close();

// ----- checks, then pack the frames --------------------------------------------------------

const assert = (ok, message) => {
  if (!ok) throw new Error(message);
};
assert(pageErrors.length === 0, `The page reported errors: ${pageErrors.join("; ")}`);
assert(frames.length === n, "Every frame needs a picture.");
assert(completed === 3, `Expected three finished downloads, found ${completed}.`);
assert(events.every((e) => (e.arrive ?? e.frame) <= n), "An event lies beyond the last frame.");
assert(pictures.size < 420, `Too many distinct frames to commit (${pictures.size}).`);

// Lossless WebP: flat interface pixels pack smaller than PNG, with the window's round corners.
execFileSync(
  "ffmpeg",
  ["-v", "error", "-y", "-i", `${RAW}u%04d.png`, "-c:v", "libwebp", "-lossless", "1", "-compression_level", "6", `${PUB}app/u%04d.webp`],
  { stdio: "inherit" },
);

writeFileSync(
  here("./src/captures.json"),
  `${JSON.stringify({ fps: FPS, width: VIEW.width, height: VIEW.height, pictures: pictures.size, frames, events, marks, stills })}\n`,
);
console.log(
  `${n} frames (${(n / FPS).toFixed(1)} s), ${pictures.size} distinct pictures, app clock ${(clockMs / 1000).toFixed(1)} s`,
);
console.log(marks);
