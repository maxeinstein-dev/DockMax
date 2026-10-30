import { $, browser, expect } from "@wdio/globals";
import { ptBR as t } from "../../src/i18n/pt-BR.ts";
import {
  closePanel,
  dockView,
  hasWindow,
  openPanelFrom,
  processMemory,
  restartApp,
} from "../app.ts";

/* M1 of study 0002: the app's own process at rest (the native dock, the
 * launcher and the rest of the Rust side, without the WebView2 processes)
 * stays under a ceiling. A panel is opened and closed first, as a day of use
 * does; the panel window is destroyed `KEEP_WARM` (45 s) after it closes
 * (spec 0011 H2), and only then is the memory read.
 *
 * The test build is a debug build with the hidden `e2e` window, so it uses
 * more than a release build: the ceiling is for this build only, set from
 * runs on the GitHub runner with room to spare. */

/** The most private memory the process may hold at rest. */
const CEILING_MB = 100_000;

const MB = 1024 * 1024;

/** The seeded clock's dock item: no label, so the kind's name. */
const CLOCK = t.widgets.kinds.clock;

describe("Memory", () => {
  it("stays under the ceiling at rest, with the panel closed", async function () {
    // The panel's 45 s, the settling and a restart.
    this.timeout(240_000);

    // A new process: what the specs before this one opened (Configurações,
    // the launcher) is not what a day at rest looks like.
    await restartApp();
    await browser.waitUntil(async () => (await dockView()).items.length > 0, {
      timeout: 30_000,
      timeoutMsg: "the dock never showed its items",
    });

    await openPanelFrom(CLOCK, () => $(".clock-time"));
    await closePanel();
    await browser.waitUntil(async () => !(await hasWindow("panel")), {
      timeout: 120_000,
      interval: 2_000,
      timeoutMsg: "the panel window was never destroyed after closing",
    });
    // What the destroyed window let go of is given back to the system.
    await browser.pause(10_000);

    const megabytes = (await processMemory()) / MB;
    console.log(
      `Dockyard E2E: private memory at rest ${megabytes.toFixed(1)} MB`,
    );
    expect(megabytes).toBeLessThan(CEILING_MB);
  });
});
