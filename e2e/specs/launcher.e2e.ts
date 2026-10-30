import { browser, expect } from "@wdio/globals";
import {
  launcherView,
  pressLauncherKey,
  toggleLauncher,
  typeInLauncher,
} from "../app.ts";

/* The launcher (spec 0012): opens, searches the Apps folder, moves the
 * selection and closes with Esc (L1); finds a sum's result, a keyword's web
 * search and the settings pages (L2). It never presses Enter, which would
 * open a program, a page or the browser on the machine. The programs differ
 * from one Windows to another (and by language), so it looks only at what
 * holds anywhere. */

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

  it("offers the web search when nothing else is found", async () => {
    await typeInLauncher("zqxjzqxj");
    await until(
      (view) =>
        view.results.length === 1 &&
        view.results[0] === "Buscar “zqxjzqxj”" &&
        view.subtitles[0] === "Google · busca na web",
      "a query with no match did not offer the default search",
    );
  });

  it("shows a sum's result first", async () => {
    await typeInLauncher("2+2*3");
    await until(
      (view) =>
        view.results[0] === "= 8" &&
        view.subtitles[0]?.startsWith("Calculadora") === true,
      "the sum's result was not first",
    );
  });

  it("searches the web by keyword, alone", async () => {
    await typeInLauncher("yt lo-fi");
    await until(
      (view) =>
        view.results.length === 1 &&
        view.results[0] === "Buscar “lo-fi”" &&
        view.subtitles[0] === "YouTube · busca na web",
      "the keyword did not make one search",
    );
  });

  it("finds the settings pages by other words", async () => {
    await typeInLauncher("wifi");
    await until(
      (view) =>
        view.results.some(
          (name, at) =>
            name === "Wi-Fi" &&
            view.subtitles[at] === "Configurações do Windows",
        ),
      "'wifi' did not find the Wi-Fi page",
    );
    await typeInLauncher("aparencia");
    await until(
      (view) =>
        view.results.some(
          (name, at) =>
            name === "Aparência do dock" &&
            view.subtitles[at] === "Configurações do Dockyard",
        ),
      "'aparencia' did not find the dock's look",
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
