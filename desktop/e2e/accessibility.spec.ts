import { expect, test } from "@playwright/test";
import { createRequire } from "node:module";

const require_ = createRequire(import.meta.url);

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("asterline.locale", "en-US");
    localStorage.setItem("asterline.theme", "dark");
  });
  await page.goto("/");
  await expect(page.getByRole("textbox", { name: /Message the team/ })).toBeEnabled();
});

async function runAxe(page: import("@playwright/test").Page) {
  // Contrast checks are timing-sensitive: freeze transitions and wait for
  // fonts so the scan measures the settled visual state, not a mid-animation
  // frame.
  await page.addStyleTag({
    content: "*, *::before, *::after { transition: none !important; animation: none !important; }",
  });
  await page.evaluate(() => document.fonts.ready);
  await page.addScriptTag({ path: require_.resolve("axe-core/axe.min.js") });
  return page.evaluate(() =>
    window.axe.run(document, {
      // Rules requiring the full a11y tree or pointer payloads that the demo
      // page cannot provide are out of scope here.
      resultTypes: ["violations"],
      rules: {
        // Chromium's autocomplete/region heuristics noise for app shells.
        region: { enabled: false },
        "landmark-one-main": { enabled: false },
        "page-has-heading-one": { enabled: false },
      },
    }),
  );
}

test("workspace surface passes the accessibility scan", async ({ page }) => {
  const results = await runAxe(page);
  const serious = results.violations.filter(({ impact }) => impact === "serious" || impact === "critical");
  expect(
    serious.map(({ id, impact, nodes }) => `${id}(${impact}) x${nodes.length}`),
    JSON.stringify(
      serious.flatMap(({ id, help, nodes }) => nodes.map((node) => ({ id, help, html: node.html }))),
      null,
      2,
    ),
  ).toEqual([]);
});

test("command palette and runs panel pass the accessibility scan", async ({ page }) => {
  const assertClean = (results: Awaited<ReturnType<typeof runAxe>>) => {
    const serious = results.violations.filter(({ impact }) => impact === "serious" || impact === "critical");
    expect(
      serious.map(({ id, impact, nodes }) => `${id}(${impact}) x${nodes.length}`),
      JSON.stringify(
        serious.flatMap(({ id, help, nodes }) => nodes.map((node) => ({ id, help, html: node.html }))),
        null,
        2,
      ),
    ).toEqual([]);
  };

  await page.getByRole("button", { name: "Runs", exact: true }).click();
  await page.getByRole("dialog", { name: "Runs" }).waitFor();
  assertClean(await runAxe(page));

  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+k");
  await page.getByRole("dialog", { name: "Command palette" }).waitFor();
  assertClean(await runAxe(page));
});
