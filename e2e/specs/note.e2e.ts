import { $, browser, expect } from "@wdio/globals";
import { ptBR as t } from "../../src/i18n/pt-BR.ts";
import { openPanelFrom, restartApp } from "../app.ts";

/* E2E 3 (spec 0003 §2.8, Verificação 10): a note written in its panel is
 * still there after a restart.
 *
 * The run's database is new, so the dock has the seed (Relógio, Progresso do
 * tempo, Nota) and the note is empty. Only this spec writes to it. */

/** The text written, with characters past ASCII, as a user would type. */
const TEXT = "Comprar pão, café e maçã";

/** The seeded note's dock item: no label, so the kind's name. */
const NOTE = t.widgets.kinds.note;

/** Opens the note's panel from the dock and returns its editor. */
async function openNote() {
  const editor = () => $("textarea.note-editor");
  await openPanelFrom(NOTE, editor);
  return editor();
}

/** The save state in the note's footer (`data-state`). */
function saveState() {
  return $(".note-save");
}

describe("Note", () => {
  it("keeps the text after a restart", async () => {
    const editor = await openNote();
    await expect(editor).toHaveValue("");

    await editor.setValue(TEXT);
    // Saved about 500 ms after the last keystroke: wait for that save to
    // start and end, not for the "Salvo" shown before any typing.
    await browser.waitUntil(
      async () => (await saveState().getAttribute("data-state")) !== "idle",
      { timeout: 5_000, timeoutMsg: "the note never started saving" },
    );
    await browser.waitUntil(
      async () => (await saveState().getAttribute("data-state")) === "idle",
      { timeoutMsg: "the note was never saved" },
    );
    await expect(saveState()).toHaveText(t.widgets.note.saveStates.idle);

    await restartApp();

    const reopened = await openNote();
    await expect(reopened).toHaveValue(TEXT);
  });
});
