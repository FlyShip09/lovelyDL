// Every screen state in mock mode, light and dark: a screenshot for review and an axe check.
// Screenshots land in docs/screenshots/mock/.

import AxeBuilder from "@axe-core/playwright";
import { expect, type Page, test } from "@playwright/test";

interface Screen {
  name: string;
  url: string;
  /** Steps after load, such as switching to a tab. */
  then?: (page: Page) => Promise<void>;
}

const screens: Screen[] = [
  { name: "empty", url: "/?scenario=empty&frozen=1" },
  { name: "busy", url: "/?scenario=busy&frozen=1" },
  { name: "preview", url: "/?scenario=preview&frozen=1" },
  { name: "ambiguous", url: "/?scenario=ambiguous&frozen=1" },
  {
    name: "options",
    url: "/?scenario=preview&frozen=1",
    then: (p) => p.getByRole("button", { name: "Options…" }).click(),
  },
  {
    name: "formats",
    url: "/?scenario=preview&frozen=1",
    then: (p) => p.getByRole("button", { name: "Advanced…" }).click(),
  },
  { name: "errors", url: "/?scenario=errors&frozen=1" },
  {
    name: "failed",
    url: "/?scenario=busy&frozen=1",
    then: (p) => p.getByRole("tab", { name: "Failed" }).click(),
  },
  {
    name: "history",
    url: "/?scenario=history&frozen=1",
    then: (p) => p.getByRole("tab", { name: "History" }).click(),
  },
  { name: "playlist", url: "/?scenario=playlist&frozen=1" },
  { name: "quit", url: "/?scenario=quit&frozen=1" },
  { name: "clipboard", url: "/?scenario=clipboard&frozen=1" },
  { name: "onboarding", url: "/?scenario=onboarding&frozen=1" },
  {
    name: "onboarding-tools",
    url: "/?scenario=onboarding&frozen=1",
    then: async (p) => {
      await p.getByRole("button", { name: "Continue" }).click();
      await p.waitForTimeout(500);
      await p.getByRole("button", { name: "Continue" }).click();
    },
  },
  {
    name: "menu",
    url: "/?scenario=busy&frozen=1",
    then: (p) => p.getByRole("button", { name: "More" }).click(),
  },
  {
    name: "context-menu",
    url: "/?scenario=busy&frozen=1",
    then: (p) => p.getByRole("row").first().click({ button: "right" }),
  },
  { name: "settings-general", url: "/?view=settings&scenario=busy" },
  {
    name: "settings-formats",
    url: "/?view=settings&scenario=busy",
    then: (p) => p.getByRole("tab", { name: "Formats" }).click(),
  },
  {
    name: "settings-network",
    url: "/?view=settings&scenario=busy",
    then: (p) => p.getByRole("tab", { name: "Network & Accounts" }).click(),
  },
  {
    name: "settings-components",
    url: "/?view=settings&scenario=busy",
    then: (p) => p.getByRole("tab", { name: "Components" }).click(),
  },
  {
    name: "settings-advanced",
    url: "/?view=settings&scenario=busy",
    then: (p) => p.getByRole("tab", { name: "Advanced" }).click(),
  },
];

for (const scheme of ["light", "dark"] as const) {
  for (const screen of screens) {
    test(`${screen.name} (${scheme})`, async ({ page }) => {
      const errors: string[] = [];
      page.on("pageerror", (e) => errors.push(e.message));
      await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
      await page.goto(screen.url);
      await page.waitForSelector("#root > *");
      await screen.then?.(page);
      // Let springs and fades settle.
      await page.waitForTimeout(700);
      await page.screenshot({ path: `docs/screenshots/mock/${screen.name}-${scheme}.png` });
      expect(errors).toEqual([]);
      const axe = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze();
      const found = axe.violations.map(
        (v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`,
      );
      expect(found).toEqual([]);
    });
  }
}

// Contrast themes (Windows' Increase Contrast): the system's own colors, real borders.
for (const name of ["busy", "preview", "errors", "playlist", "onboarding", "settings-general"]) {
  const screen = screens.find((s) => s.name === name);
  if (!screen) continue;
  test(`${name} (contrast)`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce", forcedColors: "active" });
    await page.goto(screen.url);
    await page.waitForSelector("#root > *");
    await screen.then?.(page);
    await page.waitForTimeout(700);
    await page.screenshot({ path: `docs/screenshots/mock/${name}-contrast.png` });
  });
}
