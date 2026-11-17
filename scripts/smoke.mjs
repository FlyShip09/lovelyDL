// Live smoke test of the real app: onboarding from empty folders (when shown), then an MP4,
// an MP3, subtitles with a time range, a site other than YouTube, and two playlist items.
// Every link is freely licensed (Blender open movies, a public-domain 1960 TV ad).
//
//   LOVELYDL_HOME=.dev/home-fresh WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333 target/release/lovelyDL.exe
//   node scripts/smoke.mjs
//
// Prints one line per case; inspect the files afterwards with ffprobe.
import { chromium } from "@playwright/test";

const browser = await chromium.connectOverCDP("http://127.0.0.1:9333");
const page = browser
  .contexts()
  .flatMap((c) => c.pages())
  .find((p) => !p.url().includes("view=settings"));
if (!page) throw new Error("no app page found");
page.on("pageerror", (e) => console.log(`[pageerror] ${e.message}`));
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);
const results = [];

/** Waits for an onboarding step to arrive and the previous one to finish sliding out. */
async function step(title) {
  await page.getByRole("heading", { name: title }).waitFor();
  await page.waitForFunction(() => !document.querySelector("[inert]"));
}

async function onboarding() {
  const welcome = page.getByRole("heading", { name: "Welcome to lovelyDL" });
  if (!(await welcome.isVisible())) return log("onboarding: not shown");
  await page.getByRole("button", { name: "Continue" }).click();
  await step("Where should downloads go?");
  await page.getByRole("button", { name: "Continue" }).click();
  await step("Get the tools");
  const started = Date.now();
  await page.getByRole("button", { name: "Install", exact: true }).click();
  await page.getByRole("button", { name: "Continue" }).waitFor({ timeout: 900_000 });
  const tools = (await page.locator("body").innerText())
    .split("\n")
    .filter((l) => /^\d|yt-dlp|ffmpeg|Deno/.test(l));
  log(`onboarding: tools ready in ${Math.round((Date.now() - started) / 1000)} s: ${tools.join(" | ")}`);
  await page.getByRole("button", { name: "Continue" }).click();
  await step("You're set");
  await page.getByRole("button", { name: "Start using lovelyDL" }).click();
  await page.locator("[data-url-field]").waitFor();
}

const jobIds = () =>
  page.evaluate(() => [...document.querySelectorAll("[data-job]")].map((r) => r.getAttribute("data-job")));

/** Waits until every new row has finished, then reports each one's section and label. */
async function settle(name, before, expected) {
  for (let i = 0; i < 1200; i++) {
    await page.waitForTimeout(500);
    const rows = await page.evaluate(
      (known) =>
        [...document.querySelectorAll("[data-job]")]
          .filter((r) => !known.includes(r.getAttribute("data-job")))
          .map((r) => [
            r.closest("[role=rowgroup]")?.getAttribute("aria-label") ?? "",
            r.getAttribute("aria-label") ?? "",
          ]),
      before,
    );
    if (rows.length >= expected && rows.every(([section]) => /Completed|Failed/.test(section))) {
      for (const [section, label] of rows) {
        log(`${name}: ${section} · ${label}`);
        results.push([name, section, label]);
      }
      return;
    }
  }
  log(`${name}: did not finish in 10 minutes`);
  results.push([name, "Timeout", ""]);
}

async function download(name, link, { format, quality, options } = {}) {
  const field = page.locator("[data-url-field]");
  await field.fill(link);
  await field.press("Enter");
  const go = page.getByRole("button", { name: "Download", exact: true });
  await go.waitFor({ timeout: 120_000 });
  const pick = async (popup, item) => {
    await page.getByRole("button", { name: new RegExp(`^${popup}:`) }).click();
    await page.getByRole("menuitemcheckbox", { name: item, exact: true }).click();
  };
  if (format) await pick("Format", format);
  if (quality) await pick("Quality", quality);
  if (options) {
    await page.getByRole("button", { name: "Options…", exact: true }).click();
    const sheet = page.getByRole("dialog");
    await options(sheet);
    await sheet.getByRole("button", { name: "Use these options" }).click();
    await sheet.waitFor({ state: "detached" });
  }
  await page.waitForTimeout(1500); // let the exact size and resolution arrive
  const card = (await page.locator("section[aria-label]").first().innerText()).replace(/\n+/g, " | ");
  log(`${name}: preview ${card}`);
  const before = await jobIds();
  await go.click();
  await settle(name, before, 1);
}

async function playlist(name, link, query) {
  const field = page.locator("[data-url-field]");
  await field.fill(link);
  await field.press("Enter");
  const sheet = page.getByRole("dialog");
  await sheet
    .getByText(/\d+ videos?$/)
    .first()
    .waitFor({ timeout: 120_000 });
  await sheet.getByRole("button", { name: "Select none" }).click();
  await sheet.getByRole("searchbox").fill(query);
  const options = sheet.getByRole("option");
  for (let i = 0; i < Math.min(2, await options.count()); i++) await options.nth(i).click();
  await sheet.getByRole("button", { name: /^Format:/ }).click();
  await page.getByRole("menuitemcheckbox", { name: "Audio only: M4A" }).click();
  const go = sheet.getByRole("button", { name: /^Download \d+ videos?$/ });
  log(`${name}: ${await go.innerText()}`);
  const before = await jobIds();
  await go.click();
  await settle(name, before, 2);
}

// Pass case names to run only those: node scripts/smoke.mjs playlist
const only = process.argv.slice(2);
const want = (name) => only.length === 0 || only.includes(name);

await onboarding();
const caminandes = "https://www.youtube.com/watch?v=SkVqJ1SGeL0";
if (want("mp4")) await download("mp4", caminandes, { format: "Compatible MP4", quality: "480p" });
if (want("mp3")) await download("mp3", caminandes, { format: "Audio only: MP3" });
if (want("subtitles"))
  await download("subtitles", "https://www.youtube.com/watch?v=eRsGyueVLvQ", {
    quality: "480p",
    options: async (sheet) => {
      await sheet.getByRole("checkbox", { name: /^Download subtitles/ }).click();
      await sheet.getByRole("textbox", { name: "Subtitle languages" }).fill("en");
      await sheet.getByRole("textbox", { name: "Start time" }).fill("2:00");
      await sheet.getByRole("textbox", { name: "End time" }).fill("2:30");
    },
  });
if (want("other"))
  await download(
    "other site",
    "https://archive.org/details/Election_Ads/Commercial-Nixon1960ElectionAdToughonDefense.mpg",
  );
if (want("playlist"))
  await playlist("playlist", "https://www.youtube.com/playlist?list=PL6B3937A5D230E335", "Caminandes");

const failed = results.filter(([, section]) => section !== "Completed");
log(
  failed.length
    ? `${failed.length} of ${results.length} did not complete`
    : `all ${results.length} completed`,
);
await browser.close();
process.exit(failed.length ? 1 : 0);
