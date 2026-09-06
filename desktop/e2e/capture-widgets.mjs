import { chromium } from "@playwright/test";

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
await page.addInitScript(() => {
  localStorage.setItem("asterline.locale", "en-US");
  localStorage.setItem("asterline.theme", "dark");
});
await page.goto("http://127.0.0.1:1420/");
await page.getByRole("textbox", { name: /Message the team/ }).waitFor();

// 1. Full workspace, dropdown open on the target selector.
await page.getByRole("combobox", { name: "Target" }).click();
await page.waitForTimeout(400);
await page.screenshot({ path: "../output/widget-review/dropdown-open.png" });
await page.keyboard.press("Escape");

// 2. Notices + mode switcher close-up.
await page.screenshot({ path: "../output/widget-review/workspace-dark.png", clip: { x: 280, y: 60, width: 860, height: 840 } });

// 3. Settings modal with field dropdowns.
await page.getByRole("button", { name: "Team settings" }).first().click();
await page.waitForTimeout(300);
await page.getByRole("dialog", { name: "Team settings" }).getByRole("combobox", { name: "Default recipient" }).click();
await page.waitForTimeout(300);
await page.screenshot({ path: "../output/widget-review/settings-dropdown.png" });

await browser.close();
console.log("captured");
