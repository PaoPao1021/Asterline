import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  outputDir: "../output/playwright/results",
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  // The mock-runtime queue scenario depends on a timing window; one local
  // retry keeps machine-load flake from masking genuine regressions. CI
  // retries twice.
  retries: process.env.CI ? 2 : 1,
  reporter: [
    ["line"],
    ["html", { outputFolder: "../output/playwright/report", open: "never" }],
  ],
  use: {
    baseURL: "http://127.0.0.1:1420",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    video: "retain-on-failure",
  },
  projects: [
    {
      name: "desktop-chromium",
      use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 } },
    },
  ],
  webServer: {
    // `npx vite` instead of `pnpm dev` so the web server also starts in
    // plain shells where pnpm is not on PATH.
    command: "npx vite --host 127.0.0.1 --port 1420 --strictPort",
    url: "http://127.0.0.1:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
