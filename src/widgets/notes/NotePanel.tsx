import { useEffect, useRef, useState } from "react";
import { getNote, saveNote, type WidgetId } from "../../core/widgets";
import { useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import { PANEL_TITLE_ID } from "../../panel/parts/focus";
import InlineNotice from "../../panel/parts/InlineNotice";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import { createAutosave, type Autosave, type AutosaveStatus } from "./autosave";
import "./notes.css";

/** Where the reading of the note stands, for one instance. */
type Read =
  | { id: WidgetId; state: "ready"; text: string }
  /** `get_note` failed (a text that isn't UTF-8, the storage gone): the
   * editor stays locked, so no save can replace the stored text (spec
   * §2.10, N7). */
  | { id: WidgetId; state: "unreadable" };

/** The main view of a note panel (spec 0003 §2.2, Nota): the text in a
 * 210 px editor that saves itself, and the save state in the footer. No
 * editor until `get_note` answers. */
function NotePanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"note">) {
  const t = useT();
  const [read, setRead] = useState<Read | null>(null);
  const { id } = instance;

  useEffect(() => {
    let active = true;
    getNote({ id }).then(
      (text) => {
        if (active) setRead({ id, state: "ready", text });
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
        onAdjust={onAdjust}
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
        <NoteEditor key={id} id={id} savedText={current.text} />
      )}
    </>
  );
}

const IDLE: AutosaveStatus = { state: "idle", error: null };

/** The editor of a note that was read, and the footer with its save
 * state. Saves about 500 ms after the last keystroke (`createAutosave`,
 * with its 10 s deadline), and at once when the editor or the window loses
 * the focus, when the window hides and when it unmounts (another panel,
 * Ajustes, closing). */
function NoteEditor({ id, savedText }: { id: WidgetId; savedText: string }) {
  const t = useT();
  const [text, setText] = useState(savedText);
  const [status, setStatus] = useState<AutosaveStatus>(IDLE);
  const autosave = useRef<Autosave | null>(null);

  useEffect(() => {
    const instance = createAutosave({
      save: (body) => saveNote({ id, body }),
      savedText,
    });
    autosave.current = instance;
    const unsubscribe = instance.subscribe(() =>
      setStatus(instance.getStatus()),
    );
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
      // Keep what was typed: save it first (`flush` always resolves),
      // then stop the timers.
      void instance.flush().then(() => instance.dispose());
    };
  }, [id, savedText]);

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
          autosave.current?.change(event.target.value);
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
