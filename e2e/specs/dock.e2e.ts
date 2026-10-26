import { closeSync, existsSync, openSync, readSync } from "node:fs";
import { join } from "node:path";
import process from "node:process";
import { browser, expect } from "@wdio/globals";
import { ptBR as t } from "../../src/i18n/pt-BR.ts";
import { DATA_DIR_VAR, dockView } from "../app.ts";

/* E2E 1 (spec 0003 §2.8): the dock opens, and the widget database is made in
 * the run's data folder.
 *
 * The Tauri service starts one app per run, on one data folder, and starts it
 * again for the next spec file if an earlier one (`theme.e2e.ts`) stopped
 * it. So this spec only reads: it changes nothing that another spec looks
 * at, and looks at nothing another spec changes (the theme, or widgets a
 * later spec may add to the dock). It passes in any order. */

/** First bytes of every SQLite database file. */
const SQLITE_HEADER = "SQLite format 3\0";

/** The run's data folder, where the test build keeps all its files. */
function dataDir(): string {
  const dir = process.env[DATA_DIR_VAR];
  if (!dir) throw new Error(`${DATA_DIR_VAR} is not set`);
  return dir;
}

/** The first bytes of `path` as text, or null if it doesn't exist yet. The
 * app keeps the file open; SQLite on Windows lets others read it. */
function header(path: string): string | null {
  if (!existsSync(path)) return null;
  const fd = openSync(path, "r");
  try {
    const bytes = Buffer.alloc(SQLITE_HEADER.length);
    const read = readSync(fd, bytes, 0, bytes.length, 0);
    return bytes.subarray(0, read).toString("latin1");
  } finally {
    closeSync(fd);
  }
}

describe("Dock", () => {
  it("opens with the seeded widgets, and Configurações in its menu", async () => {
    // The run's database is new: the seed (Relógio, Progresso do tempo,
    // Nota). The dock reads its items once started: waited for, not assumed.
    const seed = [
      t.widgets.kinds.clock,
      t.widgets.kinds["time-progress"],
      t.widgets.kinds.note,
    ];
    await browser.waitUntil(
      async () => {
        const names = (await dockView()).items.map((item) => item.name);
        return seed.every((name) => names.includes(name));
      },
      { timeoutMsg: "the dock never showed the seeded widgets" },
    );
    // Configurações and Sobre left the dock: the app opens from the dock's
    // menu, with Sobre inside (2026-10-26).
    const names = (await dockView()).items.map((item) => item.name);
    expect(names).not.toContain(t.dock.items.settings);
    expect(names).not.toContain(t.dock.items.about);
  });

  it("makes the widget database in the run's data folder", async () => {
    // The same folder as `settings.json` (ADR 0008), which exists only after
    // a change: the run's folder, never the user's DockMax.
    const database = join(dataDir(), "dockmax.db");

    // Opened, and migrated, by the startup plugin before any window exists
    // (`src-tauri/src/lib.rs`), so it is there by now; the wait is a margin.
    await browser.waitUntil(
      () => Promise.resolve(header(database) === SQLITE_HEADER),
      { timeoutMsg: `${database} was never made as a SQLite database` },
    );
  });
});
