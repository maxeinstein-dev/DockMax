import { useEffect, useRef, useState, type RefObject } from "react";
import { saveNote, type WidgetId } from "../../core/widgets";
import { useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import { PANEL_TITLE_ID } from "../../panel/parts/focus";
import InlineNotice from "../../panel/parts/InlineNotice";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import {
  createAutosave,
  type Autosave,
  type AutosaveDraft,
  type AutosaveStatus,
} from "./autosave";
import { draftOf, holdReads, keepDraft, readNote } from "./memory";
import "./notes.css";

/** Where the reading of the note stands, for one instance. */
type Read =
  | {
      id: WidgetId;
      state: "ready";
      text: string;
      /** Text left unsaved by the last editor of this instance. */
      draft: AutosaveDraft | null;
    }
  /** `get_note` failed (a text that isn't UTF-8, the storage gone): the
   * editor stays locked, so no save can replace the stored text (spec
   * §2.10, N7). */
  | { id: WidgetId; state: "unreadable" };

/** Saves what the editor holds and resolves once done: true when saved,
 * false when the save failed; null when there is nothing to save. */
type Flush = () => Promise<boolean> | null;

/** The main view of a note panel (spec 0003 §2.2, Nota): the text in a
 * 210 px editor that saves itself, and the save state in the footer. No
 * editor until `get_note` answers. "Ajustes" waits for the pending text to
 * be saved first: Ajustes reads the stored note to know whether removing
 * it must ask, and must never read a version older than the screen's (an
 * empty one would remove the note without asking). When that save fails,
 * Ajustes doesn't open: the view stays, with the text and the error, and
 * the next click tries again. */
function NotePanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"note">) {
  const t = useT();
  const [read, setRead] = useState<Read | null>(null);
  const { id } = instance;
  const flush = useRef<Flush | null>(null);
  const mounted = useRef(true);
  const adjusting = useRef(false);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const adjust = () => {
    if (adjusting.current) return;
    const saving = flush.current?.() ?? null;
    if (saving === null) {
      onAdjust();
      return;
    }
    adjusting.current = true;
    void saving.then((saved) => {
      adjusting.current = false;
      if (saved && mounted.current) onAdjust();
    });
  };

  useEffect(() => {
    let active = true;
    readNote(id).then(
      (text) => {
        if (active) setRead({ id, state: "ready", text, draft: draftOf(id) });
      },
      () => {
        // Only the kind of an unexpected failure was logged (`getNote`);
        // the text never is.
        if (active) setRead({ id, state: "unreadable" });
      },
    );
    return () => {
      active = false;
    };
  }, [id]);

  // A read of another instance counts as none.
  const current = read?.id === id ? read : null;

  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={adjust}
        focusAction={returnedFromSettings}
      />
      {current === null ? (
        <>
          <div className="note-placeholder" aria-hidden="true" />
          <PanelFooter alert={null}>{null}</PanelFooter>
        </>
      ) : current.state === "unreadable" ? (
        <>
          <div className="note-locked">
            <InlineNotice title={t.widgets.note.unreadable} />
          </div>
          <PanelFooter alert={null}>{null}</PanelFooter>
        </>
      ) : (
        <NoteEditor
          key={id}
          id={id}
          savedText={current.text}
          draft={current.draft}
          flushRef={flush}
        />
      )}
    </>
  );
}

const IDLE: AutosaveStatus = { state: "idle", error: null };

/** The status an editor starts in from `draft`, as `createAutosave` sets
 * it; null when there is nothing unsaved to bring back. */
function draftStatus(
  draft: AutosaveDraft | null,
  savedText: string,
): AutosaveStatus | null {
  if (draft === null || draft.text === savedText) return null;
  return draft.error === null
    ? { state: "pending", error: null }
    : { state: "error", error: draft.error };
}

/** The editor of a note that was read, and the footer with its save
 * state. Saves about 500 ms after the last keystroke (`createAutosave`,
 * with its 10 s deadline), and at once when the editor or the window loses
 * the focus, when the window hides and when it unmounts (another panel,
 * Ajustes, closing). While the text isn't saved, it is kept in memory
 * (`keepDraft`): an editor of the same note mounted later starts from it,
 * with its error. */
function NoteEditor({
  id,
  savedText,
  draft,
  flushRef,
}: {
  id: WidgetId;
  savedText: string;
  draft: AutosaveDraft | null;
  /** Set to this editor's `Flush` while it is mounted. */
  flushRef: RefObject<Flush | null>;
}) {
  const t = useT();
  const initial = draft?.text ?? savedText;
  const [text, setText] = useState(initial);
  const [status, setStatus] = useState<AutosaveStatus>(
    () => draftStatus(draft, savedText) ?? IDLE,
  );
  const autosave = useRef<Autosave | null>(null);
  /** The text on screen, for the draft. */
  const shown = useRef(initial);
  /** Keeps the draft in step with the save: set while unsaved. */
  const syncDraft = useRef<() => void>(() => {});

  useEffect(() => {
    const instance = createAutosave({
      save: (body) => saveNote({ id, body }),
      savedText,
      draft,
    });
    autosave.current = instance;
    const sync = () => {
      const { state, error } = instance.getStatus();
      keepDraft(
        id,
        state === "idle" && error === null
          ? null
          : { text: shown.current, error },
      );
    };
    syncDraft.current = sync;
    sync();
    flushRef.current = () => {
      const { state, error } = instance.getStatus();
      if (state === "idle" && error === null) return null;
      return instance.flush().then(() => instance.getStatus().error === null);
    };
    const unsubscribe = instance.subscribe(() => {
      setStatus(instance.getStatus());
      sync();
    });
    const flush = () => void instance.flush();
    const onVisibility = () => {
      if (document.visibilityState === "hidden") flush();
    };
    window.addEventListener("blur", flush);
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      window.removeEventListener("blur", flush);
      document.removeEventListener("visibilitychange", onVisibility);
      unsubscribe();
      autosave.current = null;
      flushRef.current = null;
      syncDraft.current = () => {};
      // Keep what was typed: save it first (`flush` always resolves),
      // then stop the timers. The next read of this note (shown again,
      // Ajustes) waits for it.
      holdReads(
        id,
        instance.flush().then(() => {
          sync();
          instance.dispose();
        }),
      );
    };
  }, [id, savedText, draft, flushRef]);

  const error = status.error;
  const state = error === null ? status.state : "error";
  const caption =
    error === null ? t.widgets.note.saveStates[state] : t.widgets.errors[error];

  return (
    <>
      <textarea
        className="field-input note-editor"
        aria-labelledby={PANEL_TITLE_ID}
        placeholder={t.widgets.note.placeholder}
        value={text}
        onChange={(event) => {
          setText(event.target.value);
          shown.current = event.target.value;
          autosave.current?.change(event.target.value);
          syncDraft.current();
        }}
        onBlur={() => void autosave.current?.flush()}
      />
      {/* A footer that reports failures (`alert`), so it keeps room for a
       * two-line error; the error stays in the caption, with its dot. */}
      <PanelFooter alert={null}>
        <span className="note-save" data-state={state} role="status">
          {caption}
        </span>
      </PanelFooter>
    </>
  );
}

export default NotePanel;
