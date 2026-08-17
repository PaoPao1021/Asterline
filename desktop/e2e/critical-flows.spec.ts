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
