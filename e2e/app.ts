import { spawn, type ChildProcess } from "node:child_process";
import { createServer } from "node:net";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import process from "node:process";
import { setTimeout as sleep } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { $, browser } from "@wdio/globals";
import { ptBR as t } from "../src/i18n/pt-BR.ts";
import type { ChainablePromiseElement } from "webdriverio";

/** One item of the native dock, as `e2e_dock_view` describes it. */
export interface DockItem {
  name: string;
  /** What its face draws: `app`, `glyph`, `time`, `dial` or `ring`. */
  face: string;
}

/** What the native dock shows (`e2e_dock_view`, test build only). */
export interface DockView {
  items: DockItem[];
  dark: boolean;
}

/** Calls a command of the app from the hidden `e2e` window. */
async function invoke<T>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  await browser.switchToWindow("e2e");
  return browser.execute(
    (name: string, payload: Record<string, unknown>) =>
      (
        window as unknown as {
          __TAURI_INTERNALS__: {
            invoke: (c: string, a: Record<string, unknown>) => Promise<unknown>;
          };
        }
      ).__TAURI_INTERNALS__.invoke(name, payload),
    command,
    args,
  ) as Promise<T>;
}

/** What the native dock shows now. The dock is drawn natively (spec 0011
 * H3): there is no page of it to look into, so the test build tells. */
export function dockView(): Promise<DockView> {
  return invoke<DockView>("e2e_dock_view");
}

/** Presses the dock's item named `name`, as a click would. */
export async function pressDockItem(name: string) {
  await invoke("e2e_press_dock", { name });
}

/** What the launcher shows (`e2e_launcher_view`, test build only). */
export interface LauncherView {
  open: boolean;
  query: string;
  /** The names of the results, in order. */
  results: string[];
  /** What each result shows under its name ("App", "Calculadora…"). */
  subtitles: string[];
  selected: number | null;
  /** What the list says besides the rows: `none`, `reading`, `hint` or
   * `nothingFound`. */
  note: string;
}

/** What the launcher shows now (spec 0012 L1): a native window. */
export function launcherView(): Promise<LauncherView> {
  return invoke<LauncherView>("e2e_launcher_view");
}

/** Opens the launcher, or closes it, as Alt+Space does. */
export async function toggleLauncher() {
  await invoke("e2e_launcher_toggle");
}

/** Types `text` into the launcher's box (replacing what was there). */
export async function typeInLauncher(text: string) {
  await invoke("e2e_launcher_type", { text });
}

/** Presses `up`, `down` or `escape` in the launcher's box. */
export async function pressLauncherKey(key: "up" | "down" | "escape") {
  await invoke("e2e_launcher_key", { key });
}

/** The private memory of the app's own process, in bytes: the native dock
 * and the rest of the Rust side, without the WebView2 processes
 * (`e2e_memory`, test build only). */
export function processMemory(): Promise<number> {
  return invoke<number>("e2e_memory");
}

/** Closes the open panel as its Esc does (`hide_panel`). */
export async function closePanel() {
  await browser.switchToWindow("panel");
  await browser.execute(() =>
    (
      window as unknown as {
        __TAURI_INTERNALS__: { invoke: (c: string) => Promise<unknown> };
      }
    ).__TAURI_INTERNALS__.invoke("hide_panel"),
  );
}

/** Whether the app has a window labelled `label` now. */
export async function hasWindow(label: string): Promise<boolean> {
  return (await browser.getWindowHandles()).includes(label);
}

/** The test build's identifier (`src-tauri/tauri.e2e.conf.json`). */
export const E2E_IDENTIFIER = "dev.maxsueleinstein.dockmax.e2e";

/** Variable the test build reads its data folder from (`src-tauri/src/e2e.rs`). */
export const DATA_DIR_VAR = "DOCKMAX_E2E_DATA_DIR";

/** Port of the WebDriver server embedded in the test build. */
export const WEBDRIVER_PORT = 4445;

const srcTauri = fileURLToPath(new URL("../src-tauri", import.meta.url));

/**
 * The test build, as `pnpm test:e2e:build` leaves it. Its own name
 * (`mainBinaryName`) keeps it apart from the `dockyard.exe` that `tauri dev`
 * writes to the same folder.
 */
export const APP_PATH = join(
  process.env.CARGO_TARGET_DIR
    ? resolve(srcTauri, process.env.CARGO_TARGET_DIR)
    : join(srcTauri, "target"),
  "debug",
  process.platform === "win32" ? "dockyard-e2e.exe" : "dockyard-e2e",
);

/**
 * Fails unless `APP_PATH` is a test build: a normal build would ignore the
 * data folder and use, or wake, the user's Dockyard. The identifier proves the
 * e2e config; the name of the data folder variable, read only by the code of
 * the `e2e` feature (`src-tauri/src/e2e.rs`), proves the feature.
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
  if (!binary.includes(DATA_DIR_VAR)) {
    throw new Error(
      `${APP_PATH} has the test identifier but was built without \`--features e2e\` (no ${DATA_DIR_VAR} inside), so it would ignore the run's data folder; run \`pnpm test:e2e:build\`.`,
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

/**
 * Whether the WebDriver port can be bound right now. A server that stopped
 * answering is not a port that is free: on a busy CI runner the old process
 * still held it when the next one started, and the new app, unable to bind,
 * exited and left the test waiting for a server that never came.
 */
function portFree(): Promise<boolean> {
  return new Promise((done) => {
    const probe = createServer();
    probe.once("error", () => done(false));
    probe.listen(WEBDRIVER_PORT, "127.0.0.1", () => {
      probe.close(() => done(true));
    });
  });
}

/** Starts the test build, and reports whether its server came up before the
 * process exited. */
async function launch(): Promise<ChildProcess | null> {
  const child = spawn(APP_PATH, [], {
    env: { ...process.env, TAURI_WEBDRIVER_PORT: String(WEBDRIVER_PORT) },
    stdio: ["ignore", "inherit", "inherit"],
  });
  let exited = false;
  child.once("exit", () => {
    exited = true;
  });
  const deadline = Date.now() + 30_000;
  while (!exited && Date.now() < deadline) {
    if (await serverUp()) return child;
    await sleep(250);
  }
  if (!exited) child.kill();
  return null;
}

/**
 * Presses a dock item until the panel it opens shows `ready`. Right after a
 * restart the dock may not have read its items yet, and the panel window is
 * made on demand (spec 0011 H2): so the press is repeated while the panel is
 * not there, and only a panel that never opens fails the test.
 */
export async function openPanelFrom(
  item: string,
  ready: () => ChainablePromiseElement,
) {
  await browser.waitUntil(
    async () => {
      try {
        await browser.switchToWindow("panel");
        if (await ready().isDisplayed()) return true;
      } catch {
        // Not made yet.
      }
      await pressDockItem(item).catch(() => undefined);
      try {
        await browser.switchToWindow("panel");
      } catch {
        return false;
      }
      return ready().isDisplayed();
    },
    {
      timeout: 30_000,
      interval: 750,
      timeoutMsg: "the panel never opened from the dock",
    },
  );
}

/**
 * Opens the Configurações app from the dock's item and switches to its
 * window, made on demand (spec 0011 H1): the click is repeated until the
 * window is there, as right after a restart a click may open nothing.
 */
export async function openSettings() {
  await browser.waitUntil(
    async () => {
      await pressDockItem(t.dock.items.settings).catch(() => undefined);
      try {
        await browser.switchToWindow("settings");
      } catch {
        return false;
      }
      return $(".settings-app").isDisplayed();
    },
    {
      timeout: 30_000,
      interval: 750,
      timeoutMsg: "the Configurações app never opened from the dock",
    },
  );
}

/** The test build this worker started, still to be stopped. */
let started: ChildProcess | null = null;

/** Closes every window the app has (the Configurações app, the panel, …),
 * the hidden `e2e` one last: Tauri exits with its last window. */
async function closeAllWindows() {
  const handles = await browser.getWindowHandles().catch(() => [] as string[]);
  const order = [
    ...handles.filter((handle) => handle !== "e2e"),
    ...handles.filter((handle) => handle === "e2e"),
  ];
  for (const handle of order) {
    const there = await browser
      .switchToWindow(handle)
      .then(() => true)
      .catch(() => false);
    if (!there) continue;
    // The last window takes the server down with the app: its answer may
    // never come.
    await browser.closeWindow().catch(() => undefined);
  }
}

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
  // Every window goes, again until the app is gone: the dock makes the panel
  // window ahead of a click when the pointer rests on an item, so one can be
  // born while the others close, and a hidden panel lives on for
  // `KEEP_WARM` (45 s), past the wait below.
  await waitFor(async () => {
    if (!(await serverUp())) return true;
    await closeAllWindows();
    return !(await serverUp());
  }, "the app to exit");
  await waitFor(portFree, "the WebDriver port to be released");

  // The port can be taken again between the probe and the app's own bind
  // (the old process lingering): a few launches, each given the full time.
  for (let attempt = 1; started === null; attempt++) {
    started = await launch();
    if (started === null && attempt >= 3) {
      throw new Error(
        "The restarted app never brought its WebDriver server up",
      );
    }
    if (started === null) await waitFor(portFree, "the WebDriver port");
  }
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
