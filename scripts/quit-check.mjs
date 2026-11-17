// Quit with a download running, then (second run) check it resumed after relaunch.
//
//   node scripts/quit-check.mjs start <link>   # starts it, closes the window, confirms Quit
//   node scripts/quit-check.mjs resume         # after relaunch: waits for it to finish
import { chromium } from "@playwright/test";

const [phase, link] = process.argv.slice(2);
const browser = await chromium.connectOverCDP("http://127.0.0.1:9333");
const page = browser.contexts().flatMap((c) => c.pages())[0];
await page.waitForSelector("[data-url-field]");
const labels = () => page.evaluate(() => [...document.querySelectorAll("[data-job]")].map((r) => r.getAttribute("aria-label")));

if (phase === "start") {
  await page.locator("[data-url-field]").fill(link);
  await page.locator("[data-url-field]").press("Enter");
  await page.getByRole("button", { name: "Download", exact: true }).click({ timeout: 30_000 });
  for (let i = 0; i < 120; i++) {
    await page.waitForTimeout(250);
    const [label] = await labels();
    if (label && /, ([2-6]\d) percent$/.test(label)) {
      console.log(`before quit: ${label}`);
      break;
    }
  }
  await page.getByRole("button", { name: "Close", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await dialog.waitFor();
  console.log(`dialog: ${(await dialog.innerText()).replace(/\n+/g, " | ")}`);
  await dialog.getByRole("button", { name: "Quit" }).click();
  console.log("quit pressed");
} else {
  for (let i = 0; i < 120; i++) {
    const [label] = await labels();
    if (i === 0) console.log(`after relaunch: ${label}`);
    if (label && /^[^,]+, [\d.]+ MB · /.test(label)) {
      console.log(`finished: ${label}`);
      break;
    }
    await page.waitForTimeout(500);
  }
}
await browser.close().catch(() => undefined);
