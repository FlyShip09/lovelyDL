// Live playlist check in the real app: open a playlist, search, pick two items, choose M4A
// audio, download, and wait for both rows to finish.
//
//   node scripts/playlist-check.mjs <playlist link> <search text>
import { chromium } from "@playwright/test";

const [link, query] = process.argv.slice(2);
const browser = await chromium.connectOverCDP("http://127.0.0.1:9333");
const page = browser.contexts().flatMap((c) => c.pages())[0];
await page.locator("[data-url-field]").fill(link);
await page.locator("[data-url-field]").press("Enter");
const sheet = page.getByRole("dialog");
await sheet.waitFor({ timeout: 60_000 });
await sheet.getByText(/\d+ videos?$/).waitFor({ timeout: 60_000 });
console.log("sheet:", (await sheet.locator("h2").innerText()), "|", await sheet.getByText(/videos?$/).first().innerText());
await sheet.getByRole("button", { name: "Select none" }).click();
await sheet.getByRole("searchbox").fill(query);
const options = sheet.getByRole("option");
const count = await options.count();
for (let i = 0; i < Math.min(2, count); i++) await options.nth(i).click();
await sheet.getByRole("button", { name: /^Format:/ }).click();
await page.getByRole("menuitemcheckbox", { name: "Audio only: M4A" }).click();
const go = sheet.getByRole("button", { name: /^Download \d+ videos?$/ });
console.log("button:", await go.innerText());
await go.click();
for (let i = 0; i < 240; i++) {
  await page.waitForTimeout(500);
  const labels = await page.evaluate(() =>
    [...document.querySelectorAll("[data-job]")].map((r) => [r.closest("[role=rowgroup]")?.getAttribute("aria-label"), r.getAttribute("aria-label")]),
  );
  const mine = labels.filter(([, l]) => l?.toLowerCase().includes(query.toLowerCase()));
  if (i % 10 === 0) console.log(JSON.stringify(mine));
  if (mine.length >= 2 && mine.every(([section]) => section === "Completed" || section === "Failed")) {
    console.log("done:", JSON.stringify(mine));
    break;
  }
}
await browser.close();
