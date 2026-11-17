// Drives the running app through WebView2's remote-debugging port.
//
//   WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333 npx tauri dev
//   node scripts/drive-app.mjs add "https://www.youtube.com/watch?v=SkVqJ1SGeL0"
//   node scripts/drive-app.mjs state
//   node scripts/drive-app.mjs eval "document.title"
//
// Prints console output from the page while it runs.
import { chromium } from "@playwright/test";

const [command = "state", arg = ""] = process.argv.slice(2);
const browser = await chromium.connectOverCDP("http://127.0.0.1:9333");
const page = browser.contexts().flatMap((c) => c.pages()).find((p) => !p.url().includes("view=settings"));
if (!page) throw new Error("no app page found");
page.on("console", (m) => console.log(`[console.${m.type()}] ${m.text()}`));
page.on("pageerror", (e) => console.log(`[pageerror] ${e.message}`));

const text = async () => (await page.locator("main").innerText()).replace(/\n+/g, " | ").slice(0, 600);

if (command === "add") {
  const field = page.locator("[data-url-field]");
  await field.fill(arg);
  await field.press("Enter");
  for (let i = 0; i < 20; i++) {
    await page.waitForTimeout(1000);
    console.log(`t+${i + 1}s: ${await text()}`);
  }
} else if (command === "eval") {
  console.log(await page.evaluate(arg));
} else {
  console.log(await text());
}
await browser.close();
