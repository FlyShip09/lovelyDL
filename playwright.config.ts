import { defineConfig } from "@playwright/test";

// The app's web view is WebView2, which is Chromium; screenshots and accessibility checks run
// in Playwright's Chromium against mock mode (ui/src/ipc/mock.ts).
export default defineConfig({
  testDir: "ui/e2e",
  outputDir: "ui/test-results",
  fullyParallel: true,
  reporter: [["list"]],
  use: {
    baseURL: "http://localhost:5173",
    viewport: { width: 1000, height: 660 },
    deviceScaleFactor: 2,
  },
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
  webServer: {
    command: "npm run dev",
    url: "http://localhost:5173",
    reuseExistingServer: true,
    timeout: 60_000,
  },
});
