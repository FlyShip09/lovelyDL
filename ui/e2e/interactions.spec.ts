// Interactions in mock mode: what a person does with the mouse and keyboard.

import { expect, type Page, test } from "@playwright/test";

const queuedTitles = (page: Page) =>
  page
    .getByRole("rowgroup", { name: "Queued" })
    .getByRole("row")
    .evaluateAll((rows) => rows.map((r) => r.querySelector(".selectable")?.textContent ?? ""));

test("dragging a queued row reorders the queue", async ({ page }) => {
  await page.goto("/?scenario=busy&frozen=1");
  const queued = page.getByRole("rowgroup", { name: "Queued" });
  await queued.waitFor();
  expect(await queuedTitles(page)).toEqual([
    "Night train through the Alps in winter",
    "Field recording: rain on a tin roof",
  ]);
  const [first, second] = [queued.getByRole("row").nth(0), queued.getByRole("row").nth(1)];
  const a = await first.boundingBox();
  const b = await second.boundingBox();
  if (!a || !b) throw new Error("rows not laid out");
  await page.mouse.move(b.x + 200, b.y + b.height / 2);
  await page.mouse.down();
  await page.mouse.move(b.x + 200, a.y + 4, { steps: 12 });
  await page.mouse.up();
  await page.waitForTimeout(500);
  expect(await queuedTitles(page)).toEqual([
    "Field recording: rain on a tin roof",
    "Night train through the Alps in winter",
  ]);
});

test("Alt+arrows reorder the queue from the keyboard", async ({ page }) => {
  await page.goto("/?scenario=busy&frozen=1");
  const queued = page.getByRole("rowgroup", { name: "Queued" });
  await queued.getByRole("row").nth(1).click();
  await page.keyboard.press("Alt+ArrowUp");
  expect(await queuedTitles(page)).toEqual([
    "Field recording: rain on a tin roof",
    "Night train through the Alps in winter",
  ]);
});

test("History keeps focus on the selected row while arrowing through it", async ({ page }) => {
  await page.goto("/?scenario=history");
  await page.getByRole("tab", { name: "History" }).click();
  const grid = page.getByRole("grid", { name: "History" });
  await grid.getByRole("row").first().click();
  for (let i = 0; i < 5; i++) await page.keyboard.press("ArrowDown");
  // Focus moves on the next frame, once the virtual list has rendered the row.
  await expect
    .poll(() => page.evaluate(() => document.activeElement?.getAttribute("aria-selected")))
    .toBe("true");
});

test("Ctrl+V anywhere adds the link and Enter downloads it", async ({ page }) => {
  await page.goto("/?scenario=empty");
  await page.getByText("Nothing downloading").waitFor();
  await page.evaluate(() => {
    const data = new DataTransfer();
    data.setData("text/plain", "https://www.youtube.com/watch?v=abc");
    document.body.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true }));
  });
  await expect(page.getByRole("button", { name: "Download", exact: true })).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("rowgroup", { name: "Active" }).or(page.getByRole("rowgroup", { name: "Queued" })),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Download", exact: true })).toHaveCount(0);
});

test("a preview card on its way out takes no second click", async ({ page }) => {
  await page.goto("/?scenario=preview");
  const card = page.locator("section[aria-label]").first();
  await card.getByRole("button", { name: "Download", exact: true }).click();
  // Still on screen for its exit animation, but inert, so a double-click cannot start it twice.
  await expect(page.locator("section[inert]")).toHaveCount(1);
  await expect(page.locator("section[aria-label]")).toHaveCount(0);
});

test("a link that is not a link says so", async ({ page }) => {
  await page.goto("/?scenario=empty");
  const field = page.locator("[data-url-field]");
  await field.fill("just some words");
  await field.press("Enter");
  await expect(page.getByRole("alert")).toContainText("No link found");
});

test("arrow keys move the selection and Delete removes a finished row", async ({ page }) => {
  await page.goto("/?scenario=busy&frozen=1");
  const grid = page.getByRole("grid", { name: "Downloads" });
  await grid
    .getByRole("row")
    .first()
    .click({ position: { x: 300, y: 10 } });
  await page.keyboard.press("End");
  const last = grid.getByRole("row").last();
  await expect(last).toHaveAttribute("aria-selected", "true");
  await expect(last).toContainText("The history of the paperback");
  await page.keyboard.press("Delete");
  await expect(page.getByText("The history of the paperback")).toHaveCount(0);
});

test("sheets close with Escape and give focus back", async ({ page }) => {
  await page.goto("/?scenario=preview&frozen=1");
  const options = page.getByRole("button", { name: "Options…" });
  await options.click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(options).toBeFocused();
});

test("the format pop-up changes what the preview will produce", async ({ page }) => {
  await page.goto("/?scenario=preview&frozen=1");
  await page.getByRole("button", { name: /^Format:/ }).click();
  await page.getByRole("menuitemcheckbox", { name: "Audio only: MP3" }).click();
  await expect(page.getByText(/AAC · MP3/)).toBeVisible();
  await expect(page.getByRole("button", { name: /^Quality:/ })).toHaveCount(0);
});

test("the playlist sheet selects ranges with shift-click", async ({ page }) => {
  await page.goto("/?scenario=playlist&frozen=1");
  const sheet = page.getByRole("dialog");
  await sheet.getByRole("button", { name: "Select none" }).click();
  await expect(sheet.getByRole("button", { name: /^Download 0 videos$/ })).toBeDisabled();
  await sheet.getByRole("option").nth(0).click();
  await sheet
    .getByRole("option")
    .nth(4)
    .click({ modifiers: ["Shift"] });
  // Five rows, one of them already downloaded and skipped.
  await expect(sheet.getByRole("button", { name: "Download 4 videos" })).toBeEnabled();
});

test("select all with a search picks only the matches", async ({ page }) => {
  await page.goto("/?scenario=playlist&frozen=1");
  const sheet = page.getByRole("dialog");
  await sheet.getByRole("button", { name: "Select none" }).click();
  await sheet.getByRole("searchbox").fill("extended");
  await sheet.getByRole("button", { name: "Select all" }).click();
  await sheet.getByRole("searchbox").fill("");
  await expect(sheet.getByRole("button", { name: "Download 8 videos" })).toBeEnabled();
});
