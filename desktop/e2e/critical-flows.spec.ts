import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("asterline.locale", "en-US");
    localStorage.setItem("asterline.theme", "light");
  });
  await page.goto("/");
  await expect(page.getByRole("textbox", { name: /Message the team/ })).toBeEnabled();
});

test("completes the primary collaboration and approval flow", async ({ page }) => {
  const composer = page.getByRole("textbox", { name: /Message the team/ });
  await composer.fill("Verify the desktop release contract");
  await composer.press("Control+Enter");

  await expect(page.getByText("Verify the desktop release contract")).toBeVisible();
  await expect(page.getByText(/I’m mapping that request/)).toBeVisible();

  await expect(page.getByText("Action needs your approval")).toBeVisible();
  await page.getByRole("button", { name: "Approve" }).click();
  await expect(page.getByText("Approval granted", { exact: true })).toBeVisible();
  await expect(page.getByText("Action needs your approval")).toBeHidden();
});

test("restores both panels and keeps utilities usable", async ({ page }) => {
  const sidebar = page.getByRole("complementary", { name: "Menu" });
  const inspector = page.getByRole("complementary", { name: "Inspector" });

  await sidebar.getByRole("button", { name: "Collapse sidebar" }).click();
  await expect(sidebar).toHaveClass(/is-closed/);
  await page.getByRole("main").getByRole("button", { name: "Open navigation" }).click();
  await expect(sidebar).toHaveClass(/is-open/);

  await inspector.getByRole("button", { name: "Details" }).click();
  await expect(inspector).toHaveClass(/is-closed/);
  await page.getByRole("main").getByRole("button", { name: "Details", exact: true }).click();
  await expect(inspector).toHaveClass(/is-open/);

  await page.getByRole("button", { name: "Logs" }).click();
  await expect(page.getByRole("complementary", { name: "Utilities" })).toBeVisible();
  await page.getByRole("button", { name: "Export diagnostics" }).click();
  await expect(page.getByText(/Diagnostics exported to/)).toBeVisible();
});

test("persists appearance and confirms team settings", async ({ page }) => {
  await page.getByRole("button", { name: "Theme" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");

  await page.getByRole("button", { name: "Team settings" }).first().click();
  const dialog = page.getByRole("dialog", { name: "Team settings" });
  await expect(dialog).toBeVisible();
  const teamName = dialog.getByRole("textbox", { name: "Team name" });
  await teamName.fill("Release Team");
  await dialog.getByRole("button", { name: "Save changes" }).click();
  await expect(page.getByText("Team settings confirmed by the runtime")).toBeVisible();
  await expect(page.getByText("Release Team").first()).toBeVisible();
});

test("queues a second message while streaming and pulls it back for editing", async ({ page }) => {
  const composer = page.getByRole("textbox", { name: /Message the team/ });
  // Both submissions go out back-to-back so the second lands inside the
  // first reply's streaming window — no wall-clock race.
  await composer.fill("First request");
  await composer.press("Enter");
  await composer.fill("Second request while busy");
  await composer.press("Enter");
  await expect(page.getByText(/Queued for Builder: 1/)).toBeVisible();

  // Pull the queued prompt back into the composer draft.
  await page.getByRole("button", { name: /Pull the last queued message/ }).click();
  await expect(composer).toHaveValue("Second request while busy");
});

test("opens the command palette from the composer and switches mode", async ({ page }) => {
  await page.keyboard.press("Control+k");
  const palette = page.getByRole("dialog", { name: "Command palette" });
  await expect(palette).toBeVisible();
  await palette.getByRole("textbox", { name: "Command palette" }).fill("/mode");
  await palette.getByRole("button", { name: /\/mode/ }).first().click();
  // The palette runs "/mode plan", switching the dispatch mode.
  await expect(page.locator(".mode-switcher").getByRole("button", { name: "Plan" })).toHaveClass(/active/);
});

test("opens the runs panel and shows the structured mode state", async ({ page }) => {
  await page.getByRole("button", { name: "Runs", exact: true }).click();
  const runs = page.getByRole("dialog", { name: "Runs" });
  await expect(runs).toBeVisible();
  // Run handles are scoped to the active conversation, matching the TUI.
  // The persisted database id is deliberately not exposed as the label.
  await expect(runs.getByText("run-1")).toBeVisible();
  await expect(runs.getByText(/implementing/)).toBeVisible();
});

test("target selector is a themed dropdown with keyboard support", async ({ page }) => {
  const composer = page.getByRole("textbox", { name: /Message the team/ });
  await composer.click();

  const trigger = page.getByRole("combobox", { name: "Target" });
  await trigger.click();
  const listbox = page.getByRole("listbox", { name: "Target" });
  await expect(listbox).toBeVisible();
  await expect(listbox.getByRole("option", { name: "Default", exact: true })).toHaveAttribute("aria-selected", "true");

  // Keyboard: arrow to Reviewer and commit with Enter.
  await trigger.press("ArrowDown");
  await trigger.press("ArrowDown");
  await trigger.press("ArrowDown");
  await trigger.press("Enter");
  await expect(trigger).toContainText("Reviewer");
  await expect(listbox).toBeHidden();
});
