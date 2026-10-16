import { getNote, type WidgetId } from "../../core/widgets";
import type { AutosaveDraft } from "./autosave";

/* What the note views of this window share about each instance, in memory
 * only: the saves still running, and the text not saved yet. The text
 * never leaves this module in a log, and never reaches the browser's
 * storage. */

/** Per instance, what must end before its note is read again. */
const pending = new Map<WidgetId, Promise<void>>();

/** Per instance, the text on screen while it isn't saved (failed or
 * waiting), so a panel closed and shown again brings it back. */
const drafts = new Map<WidgetId, AutosaveDraft>();

/** The unsaved text of `id`, if any. */
export function draftOf(id: WidgetId): AutosaveDraft | null {
  return drafts.get(id) ?? null;
}

/** Keeps the unsaved text of `id`; null once it is saved (or typed back to
 * the stored text). */
export function keepDraft(id: WidgetId, draft: AutosaveDraft | null): void {
  if (draft === null) drafts.delete(id);
  else drafts.set(id, draft);
}

/** Forgets `id`: the instance left the dock (and, in tests, between
 * cases). */
export function forgetNote(id: WidgetId): void {
  drafts.delete(id);
  pending.delete(id);
}

/** Holds the reads of `id` until `work` ends (it must always resolve: an
 * editor's `flush()`, bounded by the save's deadline). */
export function holdReads(id: WidgetId, work: Promise<void>): void {
  const all = Promise.all([pending.get(id), work]).then(() => {
    if (pending.get(id) === all) pending.delete(id);
  });
  pending.set(id, all);
}

/** Reads the stored note of `id` (`get_note`) once the saves of `id` still
 * running have ended: an editor shown again must never start from a text
 * older than the one just typed, or its next save would overwrite it. */
export async function readNote(id: WidgetId): Promise<string> {
  // A save that starts while waiting (text typed during the last one)
  // holds the read too.
  for (let work = pending.get(id); work; work = pending.get(id)) await work;
  return getNote({ id });
}
