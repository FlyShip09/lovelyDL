// Live check of the queue in the real app, against scripts/test-server.py:
// five links at a concurrency of two, pause and resume, cancel and clean-up.
//
//   python scripts/test-server.py .dev/media/clip.mp4 18766 1500000 &
//   LOVELYDL_HOME=<test home with maxConcurrent 2> scripts/dev-app.sh &
//   node scripts/live-check.mjs <test home>
import { chromium } from "@playwright/test";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";

const home = process.argv[2];
const browser = await chromium.connectOverCDP("http://127.0.0.1:9333");
const page = browser.contexts().flatMap((c) => c.pages())[0];
await page.waitForSelector("[data-url-field]");

const rows = () =>
  page.evaluate(() =>
    [...document.querySelectorAll("[data-job]")].map((row) => ({
      id: row.getAttribute("data-job"),
      section: row.closest("[role=rowgroup]")?.getAttribute("aria-label"),
      label: row.getAttribute("aria-label"),
    })),
  );
const partial = (id) => join(home, "cache", "partial", id);
const parts = (id) => (existsSync(partial(id)) ? readdirSync(partial(id)).filter((f) => f.endsWith(".part")) : []);
const wait = (ms) => page.waitForTimeout(ms);
const button = (id, name) => page.locator(`[data-job="${id}"]`).getByRole("button", { name, exact: true });

const links = [1, 2, 3, 4, 5].map((n) => `http://127.0.0.1:18766/v${n}.mp4`).join(" ");
await page.locator("[data-url-field]").fill(links);
await page.locator("[data-url-field]").press("Enter");

const result = { peakActive: 0, paused: null, cancelled: null };
for (let i = 0; i < 480; i++) {
  await wait(250);
  const list = await rows();
  const active = list.filter((r) => r.section === "Active");
  result.peakActive = Math.max(result.peakActive, active.length);
  const moving = active.find((r) => /, (2\d|3\d|4\d) percent$/.test(r.label ?? ""));
  if (!result.paused && moving) {
    await button(moving.id, "Pause").click();
    await wait(800);
    result.paused = { id: moving.id, partsWhilePaused: parts(moving.id), label: (await rows()).find((r) => r.id === moving.id)?.label };
    await button(moving.id, "Resume").click();
  } else if (result.paused && !result.cancelled && moving && moving.id !== result.paused.id) {
    const before = parts(moving.id);
    await button(moving.id, "Cancel").click();
    await wait(1500);
    result.cancelled = { id: moving.id, partsBefore: before, folderGone: !existsSync(partial(moving.id)) };
  }
  const done = list.length === 5 && list.every((r) => r.section === "Completed" || r.section === "Canceled");
  if (done && result.cancelled) break;
}
const final = await rows();
result.sections = final.map((r) => `${r.id}: ${r.section}`);
result.files = readdirSync(join(home, "downloads"));
console.log(JSON.stringify(result, null, 2));
await browser.close();
