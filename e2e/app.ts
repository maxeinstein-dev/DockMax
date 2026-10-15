import { spawn, type ChildProcess } from "node:child_process";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import process from "node:process";
import { setTimeout as sleep } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { browser } from "@wdio/globals";

/** The test build's identifier (`src-tauri/tauri.e2e.conf.json`). */
export const E2E_IDENTIFIER = "dev.maxsueleinstein.dockmax.e2e";

/** Variable the test build reads its data folder from (`src-tauri/src/e2e.rs`). */
export const DATA_DIR_VAR = "DOCKMAX_E2E_DATA_DIR";

/** Port of the WebDriver server embedded in the test build. */
export const WEBDRIVER_PORT = 4445;

const srcTauri = fileURLToPath(new URL("../src-tauri", import.meta.url));

/**
 * The test build, as `pnpm test:e2e:build` leaves it. Its own name
 * (`mainBinaryName`) keeps it apart from the `dockmax.exe` that `tauri dev`
 * writes to the same folder.
 */
export const APP_PATH = join(
  process.env.CARGO_TARGET_DIR
    ? resolve(srcTauri, process.env.CARGO_TARGET_DIR)
    : join(srcTauri, "target"),
  "debug",
  process.platform === "win32" ? "dockmax-e2e.exe" : "dockmax-e2e",
);

/**
 * Fails unless `APP_PATH` is a test build: a normal build would ignore the
 * data folder and use, or wake, the user's DockMax.
 */
export function assertTestBuild() {
  let binary: Buffer;
  try {
    binary = readFileSync(APP_PATH);
  } catch {
    throw new Error(
      `Test build not found at ${APP_PATH}; run \`pnpm test:e2e:build\` first.`,
    );
  }
  if (!binary.includes(E2E_IDENTIFIER)) {
    throw new Error(
      `${APP_PATH} is not a test build (no ${E2E_IDENTIFIER} inside); run \`pnpm test:e2e:build\`.`,
    );
  }
}

/** Whether the WebDriver server of a running test build answers. */
async function serverUp(): Promise<boolean> {
  try {
    const response = await fetch(`http://127.0.0.1:${WEBDRIVER_PORT}/status`, {
      signal: AbortSignal.timeout(1000),
    });
    await response.body?.cancel();
    return response.ok;
  } catch {
    return false;
  }
}

async function waitFor(condition: () => Promise<boolean>, what: string) {
  const deadline = Date.now() + 30_000;
  while (!(await condition())) {
    if (Date.now() > deadline) throw new Error(`Timed out waiting for ${what}`);
    await sleep(250);
  }
}

/** The test build this worker started, still to be stopped. */
let started: ChildProcess | null = null;

/**
 * Quits the app as a user would and starts it again on the same data
 * folder, then opens a new session on it.
 *
 * `browser.reloadSession()` alone keeps the same process: the Tauri service
 * starts the app once per run and reuses it for every session. So the app is
 * closed by destroying its windows (Tauri exits once the last one is gone,
 * like with Sair in the tray) and started here with the environment the
 * service gave the first one.
 */
export async function restartApp() {
  for (const label of ["panel", "dock"]) {
    await browser.switchToWindow(label);
    // The last window takes the server down with the app: its answer may
    // never come.
    await browser.closeWindow().catch(() => undefined);
  }
  await waitFor(async () => !(await serverUp()), "the app to exit");

  started = spawn(APP_PATH, [], {
    env: { ...process.env, TAURI_WEBDRIVER_PORT: String(WEBDRIVER_PORT) },
    stdio: ["ignore", "inherit", "inherit"],
  });
  await waitFor(serverUp, "the restarted app's WebDriver server");
  await browser.reloadSession();
}

/** Stops the app `restartApp` started, if any; the service stops the first. */
export async function stopRestartedApp() {
  const child = started;
  started = null;
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  const exited = new Promise((done) => child.once("exit", done));
  child.kill();
  await exited;
}
