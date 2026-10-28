import { useEffect, useRef, useState } from "react";
import {
  isWidgetCommandError,
  type WidgetErrorCode,
  type WidgetInstance,
} from "../../core/widgets";
import { labelFromDraft } from "../../panel/parts/label";
import type { WidgetSettingsProps, WidgetUpdate } from "../contract";

type SystemKind =
  | "activity"
  | "network"
  | "calendar"
  | "cycle"
  | "countdown"
  | "pomodoro"
  | "todo";

/** The error code of a refused command; anything else counts as a failed
 * write (the contract rejects only with a `WidgetCommandError`). */
function errorCode(error: unknown): WidgetErrorCode {
  return isWidgetCommandError(error) ? error.code : "writeFailed";
}

function updateOf<K extends SystemKind>(
  instance: WidgetInstance<K>,
): WidgetUpdate<K> {
  return { label: instance.label, config: instance.config } as WidgetUpdate<K>;
}

/** The state of a system widget's "Ajustes" view, as the progress's: every
 * change is saved at once, label and config in full; a refusal shows its
 * message and puts every field back to `instance`, the last accepted one.
 * The label saves on blur or Enter. */
export function useSystemSettings<K extends SystemKind>({
  instance,
  onUpdate,
  onRemove,
}: Pick<WidgetSettingsProps<K>, "instance" | "onUpdate" | "onRemove">) {
  const [shown, setShown] = useState<WidgetUpdate<K>>(() => updateOf(instance));
  const [labelDraft, setLabelDraft] = useState(instance.label ?? "");
  const [error, setError] = useState<WidgetErrorCode | null>(null);
  const [removing, setRemoving] = useState(false);

  // A new accepted instance replaces what is shown, compared by value.
  const acceptedKey = JSON.stringify(updateOf(instance));
  const [syncedKey, setSyncedKey] = useState(acceptedKey);
  if (syncedKey !== acceptedKey) {
    setSyncedKey(acceptedKey);
    setShown(updateOf(instance));
    setLabelDraft(instance.label ?? "");
  }

  const latest = useRef(instance);
  useEffect(() => {
    latest.current = instance;
  });

  async function save(next: WidgetUpdate<K>) {
    setError(null);
    setShown(next);
    try {
      await onUpdate(next);
    } catch (refusal) {
      setShown(updateOf(latest.current));
      setLabelDraft(latest.current.label ?? "");
      setError(errorCode(refusal));
    }
  }

  function saveConfig(config: Partial<WidgetUpdate<K>["config"]>) {
    void save({ ...shown, config: { ...shown.config, ...config } });
  }

  function commitLabel() {
    const label = labelFromDraft(labelDraft);
    if (label === "tooLong") {
      setLabelDraft(shown.label ?? "");
      setError("invalidLabel");
      return;
    }
    setLabelDraft(label ?? "");
    if (label !== shown.label) void save({ ...shown, label });
  }

  async function remove() {
    setError(null);
    setRemoving(true);
    try {
      await onRemove();
    } catch (refusal) {
      setRemoving(false);
      setError(errorCode(refusal));
    }
  }

  return {
    shown,
    labelDraft,
    setLabelDraft,
    commitLabel,
    saveConfig,
    remove,
    removing,
    error,
  };
}
