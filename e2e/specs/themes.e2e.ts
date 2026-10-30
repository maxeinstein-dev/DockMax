import { readFileSync } from "node:fs";
import { join } from "node:path";
import process from "node:process";
import { $, browser, expect } from "@wdio/globals";
import { ptBR as t } from "../../src/i18n/pt-BR.ts";
import { DATA_DIR_VAR, openSettings } from "../app.ts";

const look = t.settings.look;

/** The dock's style and icon pack in the run's settings.json. */
function savedDock(): { style: string; iconPack: string | null } {
  const dataDir = process.env[DATA_DIR_VAR];
  if (!dataDir) throw new Error(`${DATA_DIR_VAR} is not set`);
  const saved = JSON.parse(
    readFileSync(join(dataDir, "settings.json"), "utf8"),
  ) as { dock: { style: string; iconPack?: string | null } };
  return { style: saved.dock.style, iconPack: saved.dock.iconPack ?? null };
}

async function waitForDock(style: string, iconPack: string | null) {
  await browser.waitUntil(
    () => {
      const dock = savedDock();
      return dock.style === style && dock.iconPack === iconPack;
    },
    {
      timeoutMsg: `settings.json never had the dock in ${style} / ${iconPack}`,
    },
  );
}

/** A card of the Settings group with the visible label `group`. */
function card(group: string, name: string) {
  return $(
    `//*[@role="group"][@aria-labelledby=//*[normalize-space()="${group}"]/@id]` +
      `//button[normalize-space()="${name}"]`,
  );
}

describe("Themes", () => {
  it("tries a theme, keeps it, and goes back from the next one", async () => {
    await openSettings();
    await $(`button*=${t.settings.app.sections.look}`).click();

    // A theme is applied at once, and asked to be kept (spec 0020).
    const neon = card(look.themes, look.themeNames.neon);
    await neon.click();
    await waitForDock("neon", "neon");
    const trial = $(`[role="group"][aria-label="${look.themeKeep}"]`);
    await expect(trial).toBeDisplayed();
    await $(`button=${look.themeKeepButton}`).click();
    await expect(trial).not.toBeDisplayed();
    await expect(neon).toHaveAttribute("aria-pressed", "true");

    // Another one, tried and given back: the dock is Neon again.
    await card(look.themes, look.themeNames.glass).click();
    await waitForDock("liquidGlass", "vidro");
    await $(`button=${look.themeRevertButton}`).click();
    await waitForDock("neon", "neon");
    await expect(neon).toHaveAttribute("aria-pressed", "true");
  });
});
