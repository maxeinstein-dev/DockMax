import { useEffect, useState } from "react";
import {
  MAX_LABEL_CHARS,
  isWidgetCommandError,
  labelLength,
  type WidgetErrorCode,
  type WidgetId,
} from "../../core/widgets";
import { useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import InlineConfirm from "../../panel/parts/InlineConfirm";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import TextField from "../../panel/parts/TextField";
import type { WidgetSettingsProps } from "../contract";
import { readNote } from "./memory";

/** The code of a failed command; anything else reads as `writeFailed`. */
function codeOf(error: unknown): WidgetErrorCode {
  return isWidgetCommandError(error) ? error.code : "writeFailed";
}

/** Whether the stored note has text, for one instance: null while it is
 * read, or when it can't be (both ask before removing). Read once the saves
 * of the main view have ended. */
function useHasText(id: WidgetId): boolean | null {
  const [read, setRead] = useState<{ id: WidgetId; hasText: boolean } | null>(
    null,
  );
  useEffect(() => {
    let active = true;
    readNote(id).then(
      (text) => {
        if (active) setRead({ id, hasText: text !== "" });
      },
      () => {
        // Unknown: the removal asks, as for a note with text.
      },
    );
    return () => {
      active = false;
    };
  }, [id]);
  return read?.id === id ? read.hasText : null;
}

/** The "Ajustes" view of a note panel (spec 0003 §2.2, §2.3): the label and
 * "Remover do dock", which asks first when the note has text (or when that
 * isn't known yet); an empty note goes without asking. */
function NoteSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"note">) {
  const t = useT();
  const hasText = useHasText(instance.id);
  const accepted = instance.label ?? "";
  const [draft, setDraft] = useState(accepted);
  // The label Rust last accepted, as the draft started from it: a label
  // changed elsewhere (`widgets-changed`) replaces the draft.
  const [shown, setShown] = useState(accepted);
  if (shown !== accepted) {
    setShown(accepted);
    setDraft(accepted);
  }
  const [labelError, setLabelError] = useState<WidgetErrorCode | null>(null);
  const [removeError, setRemoveError] = useState<WidgetErrorCode | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [removing, setRemoving] = useState(false);
  // Back from a cancelled confirmation: the remove button takes the focus.
  const [cancelled, setCancelled] = useState(false);

  const commitLabel = () => {
    const label = draft.trim() === "" ? null : draft;
    if (label === instance.label) {
      setLabelError(null);
      return;
    }
    if (label !== null && labelLength(label) > MAX_LABEL_CHARS) {
      setLabelError("invalidLabel");
      return;
    }
    setLabelError(null);
    onUpdate({ label, config: {} }).catch((error: unknown) => {
      setLabelError(codeOf(error));
      setDraft(accepted);
    });
  };

  const remove = () => {
    setRemoveError(null);
    setRemoving(true);
    // Once removed, the host closes the panel.
    onRemove().catch((error: unknown) => {
      setRemoving(false);
      setConfirming(false);
      setCancelled(true);
      setRemoveError(codeOf(error));
    });
  };

  const error = labelError ?? removeError;

  return (
    <>
      <PanelHeader
        title={t.widgets.note.settingsTitle}
        onBack={onBack}
        focusAction
      />
      <TextField
        label={t.widgets.common.label}
        placeholder={t.widgets.kinds.note}
        value={draft}
        onChange={(value) => {
          setDraft(value);
          if (labelError === "invalidLabel") setLabelError(null);
        }}
        onCommit={commitLabel}
        invalid={labelError !== null}
      />
      {confirming ? (
        <InlineConfirm
          title={t.widgets.note.confirmRemove.title}
          detail={t.widgets.note.confirmRemove.detail}
          cancelLabel={t.widgets.note.confirmRemove.cancel}
          confirmLabel={t.widgets.note.confirmRemove.confirm}
          onCancel={() => {
            setConfirming(false);
            setCancelled(true);
          }}
          onConfirm={remove}
          busy={removing}
        />
      ) : (
        <RemoveButton
          // It mounts again after the confirmation: focused if cancelled.
          focusOnMount={cancelled}
          disabled={removing}
          onClick={() => {
            if (hasText === false) remove();
            else setConfirming(true);
          }}
        />
      )}
      <PanelFooter alert={error && t.widgets.errors[error]}>
        {t.widgets.common.footer}
      </PanelFooter>
    </>
  );
}

export default NoteSettings;
