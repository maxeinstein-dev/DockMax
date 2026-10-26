import { readFileSync } from "node:fs";
import { join } from "node:path";
import process from "node:process";
import { $, browser, expect } from "@wdio/globals";
import { ptBR as t } from "../../src/i18n/pt-BR.ts";
import { DATA_DIR_VAR, dockView, openSettings, restartApp } from "../app.ts";

/** The `data-mode` of the current window's `<html>`, set from the theme. */
function pageMode(): Promise<string | null> {
  return browser.execute(() => document.documentElement.dataset.mode ?? null);
}

async function waitForMode(mode: string) {
  await browser.waitUntil(async () => (await pageMode()) === mode, {
    timeoutMsg: `<html> never got data-mode="${mode}"`,
  });
}

/** An option of the Settings group with the visible label `group`. */
function settingsOption(group: string, option: string) {
  return $(
    `//*[@role="group"][@aria-labelledby=//*[normalize-space()="${group}"]/@id]` +
      `//button[normalize-space()="${option}"]`,
  );
}

describe("Theme", () => {
  it("applies the chosen theme and keeps it after a restart", async () => {
    await openSettings();
    const dark = settingsOption(t.settings.groups.theme, t.settings.theme.dark);
    await dark.click();
    await expect(dark).toHaveAttribute("aria-pressed", "true");
    await waitForMode("dark");
    // The native controls (the zone list, the scroll bars) turn dark too.
    expect(
      await browser.execute(
        () => getComputedStyle(document.documentElement).colorScheme,
      ),
    ).toBe("dark");

    // The test build writes to the run's folder, not to the user's DockMax.
    const dataDir = process.env[DATA_DIR_VAR];
    if (!dataDir) throw new Error(`${DATA_DIR_VAR} is not set`);
    const saved: unknown = JSON.parse(
      readFileSync(join(dataDir, "settings.json"), "utf8"),
    );
    expect(saved).toMatchObject({ theme: "dark" });

    // The dock is painted dark, before and after a restart.
    expect((await dockView()).dark).toBe(true);
    await restartApp();
    await browser.waitUntil(async () => (await dockView()).dark, {
      timeoutMsg: "the dock was not dark after the restart",
    });
  });
});
