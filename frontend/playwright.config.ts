import { defineConfig, devices } from "@playwright/test";

const viewportMatrix = [
  { name: "960x720-scale-100", width: 960, height: 720, scale: 1 },
  { name: "960x720-scale-125", width: 960, height: 720, scale: 1.25 },
  { name: "960x720-scale-150", width: 960, height: 720, scale: 1.5 },
  { name: "1280x820-scale-100", width: 1280, height: 820, scale: 1 },
  { name: "1280x820-scale-125", width: 1280, height: 820, scale: 1.25 },
  { name: "1280x820-scale-150", width: 1280, height: 820, scale: 1.5 },
  { name: "1920x1080-scale-100", width: 1920, height: 1080, scale: 1 },
  { name: "1920x1080-scale-125", width: 1920, height: 1080, scale: 1.25 },
  { name: "1920x1080-scale-150", width: 1920, height: 1080, scale: 1.5 },
] as const;

export default defineConfig({
  testDir: "./visual-tests",
  fullyParallel: false,
  forbidOnly: true,
  retries: 0,
  reporter: "line",
  outputDir: "./test-results/visual",
  expect: {
    toHaveScreenshot: {
      animations: "disabled",
      caret: "hide",
      maxDiffPixelRatio: 0.006,
    },
  },
  use: {
    ...devices["Desktop Edge"],
    baseURL: "http://127.0.0.1:4174",
    channel: "msedge",
    headless: true,
    locale: "zh-CN",
    colorScheme: "light",
    reducedMotion: "reduce",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  projects: viewportMatrix.map(({ name, width, height, scale }) => ({
    name,
    use: {
      viewport: { width, height },
      deviceScaleFactor: scale,
    },
  })),
  webServer: {
    command: "vite --config playwright.vite.config.ts",
    url: "http://127.0.0.1:4174",
    reuseExistingServer: false,
    stdout: "pipe",
    stderr: "pipe",
  },
});
