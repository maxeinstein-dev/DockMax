import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import process from "node:process";
import type { TauriCapabilities } from "@wdio/tauri-service";
import {
  APP_PATH,
  DATA_DIR_VAR,
  WEBDRIVER_PORT,
  assertTestBuild,
  stopRestartedApp,
} from "./app.ts";

/** Parent of the run folders; only this suite writes here. */
const RUNS_DIR = join(tmpdir(), "dockmax-e2e");

const capabilities: TauriCapabilities[] = [
  {
    browserName: "tauri",
    "tauri:options": { application: APP_PATH },
    "wdio:tauriServiceOptions": {
      // The WebDriver server embedded in the test build: no msedgedriver to
      // keep in step with WebView2 (spec 0003 §2.8).
      driverProvider: "embedded",
      embeddedPort: WEBDRIVER_PORT,
      // Sessions start in the test build's hidden window, which drives the
      // native dock (spec 0011 H3); the panel is made when opened.
      windowLabel: "e2e",
      captureBackendLogs: true,
    },
  },
];

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: ["./specs/**/*.e2e.ts"],
  // One app at a time: every session drives the same test build.
  maxInstances: 1,
  capabilities,
  services: [["@wdio/tauri-service", {}]],
  framework: "mocha",
  mochaOpts: { ui: "bdd", timeout: 120_000 },
  reporters: ["spec"],
  logLevel: "warn",
  waitforTimeout: 10_000,

  /**
   * Runs before the Tauri service starts the app. Each run gets a new data
   * folder, passed down through the environment to the app and to the
   * workers (which restart it). Folders of earlier runs are removed here,
   * not at the end: the app is still running when `onComplete` runs, and the
   * last run's settings and logs stay for a look after a failure.
   */
  onPrepare() {
    assertTestBuild();
    rmSync(RUNS_DIR, { recursive: true, force: true });
    mkdirSync(RUNS_DIR, { recursive: true });
    const dataDir = mkdtempSync(join(RUNS_DIR, "run-"));
    process.env[DATA_DIR_VAR] = dataDir;
    // The suite reads the PT-BR texts, and "Sistema" would follow the
    // runner's Windows, in English on CI (spec 0019 G1). A file written
    // here also counts the welcome tour as shown (G2), so no tour window
    // opens in the middle of the specs.
    writeFileSync(
      join(dataDir, "settings.json"),
      JSON.stringify({ version: 1, language: "pt-BR" }),
    );
    console.log(`DockMax E2E data folder: ${dataDir}`);
  },

  /**
   * Stops the app a spec restarted. Only once the session is over: stopped
   * any earlier (in a mocha `after`), the runner's closing request finds no
   * server and the spec fails.
   */
  async afterSession() {
    await stopRestartedApp();
  },
};
