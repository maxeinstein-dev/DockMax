import { getNote, type WidgetId } from "../../core/widgets";

/* What the note views of this window share about each instance, in memory
 * only: the saves still running. The text never leaves this module in a
 * log, and never reaches the browser's storage. */

/** Per instance, what must end before its note is read again. */
const pending = new Map<WidgetId, Promise<void>>();

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
