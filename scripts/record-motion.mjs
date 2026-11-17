// Records the key motion flows in mock mode and turns each into a contact sheet of frames,
// for checking timing and continuity frame by frame.
//
//   npm run dev &   # or let it run
//   node scripts/record-motion.mjs        # writes docs/screenshots/motion/<flow>.png
import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { mkdirSync, readdirSync, renameSync, rmSync } from "node:fs";
import { join } from "node:path";

const out = "docs/screenshots/motion";
const tmp = ".dev/motion";
mkdirSync(out, { recursive: true });
rmSync(tmp, { recursive: true, force: true });

const flows = {
  // Paste a link: activity line, preview card grows in, thumbnail fades in; Return starts it.
  "paste-and-download": async (page) => {
    await page.goto("/?scenario=empty");
    await page.getByText("Nothing downloading").waitFor();
    await page.waitForTimeout(300);
    await page.evaluate(() => {
      const data = new DataTransfer();
      data.setData("text/plain", "https://www.youtube.com/watch?v=abc");
      document.body.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true }));
    });
    await page.getByRole("button", { name: "Download", exact: true }).waitFor();
    await page.waitForTimeout(600);
    await page.keyboard.press("Enter");
    await page.waitForTimeout(1500);
  },
  // A sheet slides down from under the toolbar and back up.
  sheet: async (page) => {
    await page.goto("/?scenario=preview&frozen=1");
    await page.getByRole("button", { name: "Options…" }).waitFor();
    await page.waitForTimeout(300);
    await page.getByRole("button", { name: "Options…" }).click();
    await page.waitForTimeout(700);
    await page.keyboard.press("Escape");
    await page.waitForTimeout(700);
  },
  // The sidebar slides away and the content moves over; then back.
  sidebar: async (page) => {
    await page.goto("/?scenario=busy&frozen=1");
    await page.getByRole("tab", { name: "History" }).waitFor();
    await page.waitForTimeout(300);
    await page.getByRole("tab", { name: "History" }).click();
    await page.waitForTimeout(400);
    await page.getByRole("tab", { name: "Downloads" }).click();
    await page.waitForTimeout(400);
    await page.keyboard.press("Control+Alt+s");
    await page.waitForTimeout(600);
    await page.keyboard.press("Control+Alt+s");
    await page.waitForTimeout(600);
  },
};

const browser = await chromium.launch();
for (const [name, run] of Object.entries(flows)) {
  const context = await browser.newContext({
    baseURL: "http://localhost:5173",
    viewport: { width: 1000, height: 660 },
    recordVideo: { dir: join(tmp, name), size: { width: 1000, height: 660 } },
    reducedMotion: "no-preference",
  });
  const page = await context.newPage();
  await run(page);
  await context.close();
  const video = readdirSync(join(tmp, name)).find((f) => f.endsWith(".webm"));
  const src = join(tmp, name, video);
  renameSync(src, join(tmp, `${name}.webm`));
  // 20 frames a second, 6 across: each row of the sheet is 300 ms of motion.
  execFileSync("ffmpeg", [
    "-y", "-loglevel", "error", "-i", join(tmp, `${name}.webm`),
    "-vf", "fps=20,scale=320:-1,tile=6x8:padding=4:color=gray",
    "-frames:v", "1", join(out, `${name}.png`),
  ]);
  console.log(`${name}: ${join(out, `${name}.png`)}`);
}
await browser.close();
