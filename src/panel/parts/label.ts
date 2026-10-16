import { labelLength, MAX_LABEL_CHARS } from "../../core/widgets";

/** The label a "Rótulo" field saves, from the text as typed: trimmed, an
 * empty one is null (the kind's name), and one over `MAX_LABEL_CHARS` code
 * points is refused (`"tooLong"`: the view shows `invalidLabel` in its
 * footer and puts the field back). Rust checks it again; this keeps the
 * clock's and the progress's fields alike. */
export function labelFromDraft(draft: string): string | null | "tooLong" {
  const trimmed = draft.trim();
  if (trimmed === "") return null;
  return labelLength(trimmed) > MAX_LABEL_CHARS ? "tooLong" : trimmed;
}
