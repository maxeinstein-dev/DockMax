import { $, browser, expect } from "@wdio/globals";
import { ptBR as t } from "../../src/i18n/pt-BR.ts";
import { restartApp } from "../app.ts";

/* The clock's Ajustes (spec 0003 §2.2): the zone, the seconds and the icon
 * style chosen in the panel reach the panel, the dock and Itens do dock, and
 * are still there after a restart (2026-10-16 manual test).
 *
 * The run's database is new, so the dock has the seed (Relógio, Progresso do
 * tempo, Nota). Only this spec changes the seeded clock; the others never
 * look at it. */

const ZONE = "Asia/Tokyo";

/** The seeded clock's dock item: no label, so the kind's name. */
function clockItem() {
  return $(
    `nav[aria-label="${t.dock.label}"] button[aria-label="${t.widgets.kinds.clock}"]`,
  );
}

/** An option of the segmented group with the visible label `group`. */
function option(group: string, name: string) {
  return $(
    `//*[@role="group"][@aria-labelledby=//*[normalize-space()="${group}"]/@id]` +
      `//button[normalize-space()="${name}"]`,
  );
}

/** Opens the clock's panel from the dock, on its main view. */
async function openClock() {
  await browser.switchToWindow("dock");
  await clockItem().click();
  await browser.switchToWindow("panel");
  await $(".clock-time").waitForDisplayed();
}

/** The zone list's option values, in order. */
function zoneValues(): Promise<string[]> {
  return browser.execute(() =>
    Array.from(document.querySelectorAll("select option"), (o) =>
      o.getAttribute("value"),
    ).filter((value): value is string => value !== null),
  );
}

describe("Clock", () => {
  it("applies and keeps the zone, the seconds and the icon style", async () => {
    await openClock();
    await expect($(".clock-seconds")).not.toBeExisting();
    await $(`button=${t.widgets.common.adjust}`).click();

    const zone = $(`select`);
    await zone.waitForDisplayed();
    // The system zone first, then the zones by their offset now.
    const values = await zoneValues();
    expect(values[0]).toBe("");
    expect(values.indexOf("Pacific/Honolulu")).toBeLessThan(
      values.indexOf("America/Sao_Paulo"),
    );
    expect(values.indexOf("America/Sao_Paulo")).toBeLessThan(
      values.indexOf(ZONE),
    );
    await expect($(`select option[value="${ZONE}"]`)).toHaveText(
      `UTC+9 · Tokyo · ${ZONE}`,
    );

    // The embedded WebDriver sends untrusted events, which never open or
    // change a native <select>: the choice is made as the list makes it,
    // a new value and a `change` event.
    await browser.execute((value: string) => {
      const select = document.querySelector("select");
      if (!select) throw new Error("no zone list");
      select.value = value;
      select.dispatchEvent(new Event("change", { bubbles: true }));
    }, ZONE);
    const show = option(
      t.widgets.clock.seconds,
      t.widgets.clock.secondsOptions.show,
    );
    await show.click();
    await expect(show).toHaveAttribute("aria-pressed", "true");
    const analog = option(
      t.widgets.clock.iconStyle,
      t.widgets.clock.iconStyles.analog,
    );
    await analog.click();
    await expect(analog).toHaveAttribute("aria-pressed", "true");
    await expect($(".panel-foot")).toHaveText(
      expect.stringContaining(t.widgets.common.footer),
    );

    // The main view, from the instance Rust sent back: the seconds are
    // there at once, as two digits (2026-10-19 release retest: Mostrar,
    // then Voltar, with no other change in between).
    await $(`button[aria-label="${t.widgets.common.back}"]`).click();
    await expect($(".clock-seconds")).toBeDisplayed();
    await expect($(".clock-seconds")).toHaveText(
      expect.stringMatching(/^:\d{2}$/),
    );
    await expect($(".panel-foot")).toHaveText(
      expect.stringContaining("Tokyo · UTC+9"),
    );

    // The native controls (this list, the scroll bars) follow the theme.
    const [scheme, mode, dark] = await browser.execute(() => [
      getComputedStyle(document.documentElement).colorScheme,
      document.documentElement.dataset.mode ?? null,
      matchMedia("(prefers-color-scheme: dark)").matches,
    ]);
    expect(scheme).toBe(mode ?? (dark ? "dark" : "light"));

    // The dock icon turned analog.
    await browser.switchToWindow("dock");
    await expect(clockItem().$(".clock-tile-dial")).toBeExisting();

    // Itens do dock names the zone.
    await $(
      `nav[aria-label="${t.dock.label}"] button[aria-label="${t.dock.items.settings}"]`,
    ).click();
    await browser.switchToWindow("panel");
    await $(`button*=${t.widgets.items.title}`).click();
    await expect(
      $(`ul[aria-label="${t.widgets.items.listLabel}"] li`),
    ).toHaveText(expect.stringContaining("Tokyo"));

    await restartApp();

    await openClock();
    await expect($(".clock-seconds")).toBeDisplayed();
    await expect($(".panel-foot")).toHaveText(
      expect.stringContaining("Tokyo · UTC+9"),
    );
    await browser.switchToWindow("dock");
    await expect(clockItem().$(".clock-tile-dial")).toBeExisting();
  });
});
