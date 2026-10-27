import { browser, expect } from "@wdio/globals";
import {
  launcherView,
  pressLauncherKey,
  toggleLauncher,
  typeInLauncher,
} from "../app.ts";

/* The launcher (spec 0012 L1): opens, searches the Apps folder, moves the
 * selection and closes with Esc. It never presses Enter, which would open a
 * program on the machine. The programs differ from one Windows to another
 * (and by language), so it looks only at what holds anywhere. */

/** Waits until the launcher's view passes `check`. */
async function until(
  check: (view: Awaited<ReturnType<typeof launcherView>>) => boolean,
  message: string,
) {
  await browser.waitUntil(async () => check(await launcherView()), {
    timeoutMsg: message,
  });
}

describe("Launcher", () => {
  it("opens empty and reads the Apps folder", async () => {
    await toggleLauncher();
    await until((view) => view.open, "the launcher never opened");
    // Nothing typed and nothing opened yet in this run: the hint, once the
    // Apps folder is read.
    await until(
      (view) => view.note === "hint",
      "the launcher never read the Apps folder",
    );
    expect((await launcherView()).query).toBe("");
  });

  it("finds programs and moves the selection with the arrows", async () => {
    // Every Windows has programs with an "a" in a word's start.
    await typeInLauncher("a");
    await until(
      (view) => view.results.length >= 2,
      "nothing was found for 'a'",
    );
    const found = await launcherView();
    expect(found.selected).toBe(0);
    expect(found.results.length).toBeLessThanOrEqual(8);

    await pressLauncherKey("down");
    await until((view) => view.selected === 1, "Down did not move on");
    await pressLauncherKey("up");
    await until((view) => view.selected === 0, "Up did not move back");
  });

  it("says when nothing is found", async () => {
    await typeInLauncher("zqxjzqxj");
    await until(
      (view) => view.note === "nothingFound" && view.results.length === 0,
      "a query with no match still had results",
    );
  });

  it("closes with Esc, and opens empty again", async () => {
    await pressLauncherKey("escape");
    await until((view) => !view.open, "Esc did not close the launcher");

    await toggleLauncher();
    await until((view) => view.open, "the launcher did not open again");
    expect((await launcherView()).query).toBe("");
    await pressLauncherKey("escape");
    await until((view) => !view.open, "Esc did not close it again");
  });
});
