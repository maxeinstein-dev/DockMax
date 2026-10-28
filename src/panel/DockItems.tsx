import { inWorkspace, type Workspace } from "../core/workspaces";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  CREATABLE_KINDS,
  MAX_WIDGETS,
  addWidget,
  dismissStorageNotice,
  getNote,
  isWidgetCommandError,
  moveWidget,
  removeWidget,
  shareWidget,
  type CreatableKind,
  type MoveDirection,
  type WidgetErrorCode,
  type WidgetId,
  type WidgetInstance,
  type WidgetsSnapshot,
} from "../core/widgets";
import { format, plural, useT, type Messages } from "../i18n";
import { WIDGET_REGISTRY, isCreatableInstance } from "../widgets/registry";
import { widgetName } from "../widgets/names";
import { draftOf, forgetNote, readNote } from "../widgets/notes/memory";
import PanelFooter from "./PanelFooter";
import IconButton from "./parts/IconButton";
import InlineConfirm from "./parts/InlineConfirm";
import InlineNotice from "./parts/InlineNotice";
import PanelHeader from "./parts/PanelHeader";
import { PANEL_TITLE_ID } from "./parts/focus";
import "./DockItems.css";

/** Props of Configurações › Itens do dock (W4), a subview of the Settings
 * panel. It changes the list through `addWidget`, `moveWidget`,
 * `removeWidget` and `dismissStorageNotice` (`src/core/widgets.ts`); every accepted change
 * comes back in `snapshot` through `widgets-changed`, and a rejected one
 * shows its message (`t.widgets.errors[code]`). */
export interface DockItemsProps {
  /** The instances in dock order and the storage state (`useWidgets`);
   * null while loading, and for good when they can't be read. */
  snapshot: WidgetsSnapshot | null;
  /** The workspace being edited (spec 0013): only its widgets list, in its
   * order, and every change goes to it. Without one, the whole list and the
   * workspace in use. */
  workspace?: Workspace | null;
  /** Voltar: back to Configurações, where the "Itens do dock" row takes the
   * focus (`NavRow` `focusOnMount`). The view mounts with the focus on
   * Voltar. */
  /** Back to the main view of the panel; none in the Configurações app,
   * where the views are sections. */
  onBack?: () => void;
  /** In Configurações › Widgets, where the catalog adds: the list is "Em
   * uso", without the add buttons. */
  catalog?: boolean;
}

/** The glyph of each kind on its mini tile (DESIGN.md "Dock item": Mini;
 * paths from the approved mock, the dock's own set). */
const GLYPHS: Readonly<Record<CreatableKind, readonly string[]>> = {
  clock: ["M3.5 12a8.5 8.5 0 1 0 17 0a8.5 8.5 0 1 0 -17 0", "M12 7.5V12l3 2"],
  "time-progress": [
    "M3.5 12a8.5 8.5 0 1 0 17 0a8.5 8.5 0 1 0 -17 0",
    "M12 3.5a8.5 8.5 0 0 1 8 11.4",
  ],
  note: [
    "M7.5 3.5h9a2.5 2.5 0 0 1 2.5 2.5v12a2.5 2.5 0 0 1 -2.5 2.5h-9a2.5 2.5 0 0 1 -2.5 -2.5v-12a2.5 2.5 0 0 1 2.5 -2.5z",
    "M8.5 8.5h7M8.5 12h7M8.5 15.5h4",
  ],
  activity: ["M7 19V11M12 19V5M17 19v-5"],
  network: ["M8.5 4.5v15M4.5 15.5l4 4 4-4M15.5 19.5v-15M11.5 8.5l4-4 4 4"],
  calendar: ["M5 6.5h14v13H5zM5 10.5h14M9 4v4M15 4v4"],
  cycle: ["M3.5 12a8.5 8.5 0 1 0 17 0a8.5 8.5 0 1 0 -17 0"],
  pomodoro: ["M5 13.5a7 7 0 1 0 14 0a7 7 0 1 0 -14 0M12 13.5V10M10 3.5h4"],
  todo: ["M4.5 7l1.5 1.5 3-3M4.5 14l1.5 1.5 3-3M12 7.5h7.5M12 14.5h7.5"],
  water: ["M12 3.5c3 4 5.5 7 5.5 10a5.5 5.5 0 0 1 -11 0c0 -3 2.5 -6 5.5 -10z"],
  countdown: [
    "M7 4h10M7 20h10M8 4c0 4 4 5 4 8s-4 4-4 8M16 4c0 4-4 5-4 8s4 4 4 8",
  ],
};

/** The kind's tile, static: 26 px in the list, 18 px on an add button and
 * in the miniatures of the themes (`StyleGallery`).
 * Decorative: the name is next to it. */
export function MiniTile({
  kind,
  small = false,
  wide = false,
}: {
  kind: CreatableKind;
  small?: boolean;
  /** Two icons wide, as a wide tile in the dock (spec 0017). */
  wide?: boolean;
}) {
  const glaze = WIDGET_REGISTRY[kind].glaze;
  return (
    <span
      className={`mini-tile ${glaze}${small ? " small" : ""}${wide ? " wide" : ""}`}
      aria-hidden="true"
    >
      <svg viewBox="0 0 24 24">
        {GLYPHS[kind].map((d, i) => (
          // The progress ring's track is dimmed (the mock's opacity .4).
          <path
            key={d}
            d={d}
            className={
              kind === "time-progress" && i === 0 ? "track" : undefined
            }
          />
        ))}
      </svg>
    </span>
  );
}

/** Lines of a note's text, not counting trailing line breaks: 0 for an
 * empty note. */
function lineCount(body: string): number {
  const trimmed = body.replace(/[\r\n]+$/, "");
  return trimmed === "" ? 0 : trimmed.split(/\r\n|\r|\n/).length;
}

/** The city part of an IANA zone name: `America/Sao_Paulo` → `Sao
 * Paulo`. */
function cityOf(timeZone: string): string {
  return (timeZone.split("/").pop() ?? timeZone).replace(/_/g, " ");
}

/** What the subtitle says of the instance itself (zone, periods, lines),
 * or null when there is nothing to say (a note not read yet or
 * unreadable). */
function detailOf(
  t: Messages,
  instance: WidgetInstance,
  noteLines: Readonly<Record<WidgetId, number | null>>,
): string | null {
  switch (instance.kind) {
    case "clock":
      return instance.config.timeZone === null
        ? t.widgets.clock.systemZoneSubtitle
        : cityOf(instance.config.timeZone);
    case "time-progress":
      return instance.config.periods
        .map((period) => t.widgets.progress.periods[period])
        .join(", ");
    case "note": {
      const lines = noteLines[instance.id];
      if (lines === undefined || lines === null) return null;
      return lines === 0
        ? t.widgets.note.empty
        : plural(t.widgets.note.lines, lines);
    }
    default:
      return null;
  }
}

/** The subtitle under the name: the detail; with a label, the kind before
 * it (`Relógio · Lisboa`), since the name no longer says it. */
function subtitleOf(
  t: Messages,
  instance: WidgetInstance,
  noteLines: Readonly<Record<WidgetId, number | null>>,
): string | null {
  const detail = detailOf(t, instance, noteLines);
  if (instance.label === null) return detail;
  const kind = t.widgets.kinds[instance.kind];
  return detail === null
    ? kind
    : format(t.widgets.common.labelledSubtitle, { kind, detail });
}

/** Where the focus goes once the snapshot that answers an action comes:
 * the control it was on may have moved, turned off or gone. */
type FocusTarget =
  /** Subir or descer of a moved item; its other button when this one
   * turned off at an end. */
  | { kind: "move"; id: WidgetId; direction: MoveDirection }
  /** The new item's subir, when the add buttons turned off at the
   * limit. */
  | { kind: "added"; id: WidgetId }
  /** Voltar, once the dismissed notice is gone. */
  | { kind: "back" }
  /** Once the removed item is gone: the remove button of the item now at
   * its place, or of the one before it at the end, or the first add
   * button when the list is empty. */
  | { kind: "removed"; id: WidgetId; index: number }
  /** The item's remove button, back from a cancelled or failed
   * confirmation. */
  | { kind: "remove"; id: WidgetId };

const OTHER: Readonly<Record<MoveDirection, MoveDirection>> = {
  up: "down",
  down: "up",
};

/** Whether the focus is nowhere useful: on the page, outside it or on a
 * control that turned off. */
function focusLost(): boolean {
  const active = document.activeElement;
  return (
    active === null ||
    active === document.body ||
    !active.isConnected ||
    (active instanceof HTMLButtonElement && active.disabled)
  );
}

/**
 * Configurações › Itens do dock (spec 0003 §2.3 item 1, §2.4 item 7;
 * DESIGN.md "Configurações › Itens do dock"): the widgets in dock order,
 * each with subir, descer and remover (a note with text asks first, in its
 * row), the add buttons with the count, and the
 * notice of a recovered or unavailable storage. Rust is the source of
 * truth: nothing here changes before `widgets-changed` says so.
 */
function DockItems({ snapshot, workspace, onBack, catalog }: DockItemsProps) {
  const t = useT();
  const [error, setError] = useState<WidgetErrorCode | null>(null);
  const [dismissing, setDismissing] = useState(false);
  const [noteLines, setNoteLines] = useState<Record<WidgetId, number | null>>(
    {},
  );
  /** The note whose removal waits for the inline confirmation, if any. */
  const [confirming, setConfirming] = useState<WidgetId | null>(null);
  /** The item being removed, or the note read before asking: its remove
   * button turns off. */
  const [removing, setRemoving] = useState<WidgetId | null>(null);
  /** Subir, descer and remover of each item, by `id:direction` and
   * `id:remove`. */
  const moveButtons = useRef(new Map<string, HTMLButtonElement>());
  const addButton = useRef<HTMLButtonElement>(null);
  const focusTarget = useRef<FocusTarget | null>(null);

  const storage = snapshot?.storage.state ?? null;
  const available = snapshot !== null && storage !== "unavailable";
  const widgets = available
    ? inWorkspace(snapshot.widgets, workspace, "widget")
    : [];
  const space = workspace?.id;
  const full = widgets.length >= MAX_WIDGETS;
  /** The widgets other workspaces show and this one doesn't (spec 0013
   * C3): one click shows them here too. */
  const others =
    available && workspace
      ? snapshot.widgets.filter(
          (instance) => !widgets.some((shown) => shown.id === instance.id),
        )
      : [];

  /** Moves the focus to `focusTarget` when the snapshot allows it. */
  function restoreFocus() {
    const target = focusTarget.current;
    if (target === null) return;
    switch (target.kind) {
      case "move": {
        const button =
          moveButtons.current.get(`${target.id}:${target.direction}`) ?? null;
        if (button === null) return;
        const other = moveButtons.current.get(
          `${target.id}:${OTHER[target.direction]}`,
        );
        (button.disabled && other ? other : button).focus();
        break;
      }
      case "added": {
        const button = moveButtons.current.get(`${target.id}:up`) ?? null;
        // Not in the list yet: wait for its snapshot.
        if (button === null) return;
        if (focusLost()) button.focus();
        break;
      }
      case "removed": {
        // Still listed: wait for the snapshot without it.
        if (widgets.some((instance) => instance.id === target.id)) return;
        const next = widgets[Math.min(target.index, widgets.length - 1)];
        const button = next
          ? moveButtons.current.get(`${next.id}:remove`)
          : addButton.current;
        if (focusLost()) button?.focus();
        break;
      }
      case "remove": {
        const button = moveButtons.current.get(`${target.id}:remove`);
        if (button === undefined) return;
        button.focus();
        break;
      }
      case "back": {
        if (storage === "recovered") return;
        // The Voltar button comes right before the title (`PanelHeader`).
        const back =
          document.getElementById(PANEL_TITLE_ID)?.previousElementSibling;
        if (focusLost() && back instanceof HTMLButtonElement) back.focus();
        break;
      }
    }
    focusTarget.current = null;
  }

  // Runs after each new snapshot, before paint, so the focus never shows
  // on the wrong control. `restoreFocus` reads only refs and `storage`.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useLayoutEffect(restoreFocus, [snapshot, confirming, removing]);

  // The lines of each note, for its subtitle: read once per note.
  const noteIds = widgets
    .filter((instance) => instance.kind === "note")
    .map((instance) => instance.id);
  const noteKey = noteIds.join(",");
  useEffect(() => {
    let active = true;
    for (const id of noteKey === "" ? [] : noteKey.split(",")) {
      if (id in noteLines) continue;
      getNote({ id }).then(
        (body) => {
          if (active)
            setNoteLines((lines) => ({ ...lines, [id]: lineCount(body) }));
        },
        () => {
          // Unreadable (or gone): the subtitle says nothing of its lines.
          if (active) setNoteLines((lines) => ({ ...lines, [id]: null }));
        },
      );
    }
    return () => {
      active = false;
    };
    // Only a new note needs reading; `noteLines` is what was read.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [noteKey]);

  function fail(reason: unknown) {
    focusTarget.current = null;
    setError(isWidgetCommandError(reason) ? reason.code : "notReady");
  }

  function move(id: WidgetId, direction: MoveDirection) {
    setError(null);
    // Applied when the reordered snapshot comes (`widgets-changed`), which
    // may be before or after the command answers.
    focusTarget.current = { kind: "move", id, direction };
    moveWidget({ id, direction, workspace: space }).catch(fail);
  }

  function add(kind: CreatableKind) {
    setError(null);
    focusTarget.current = null;
    addWidget({ kind, workspace: space }).then((instance) => {
      focusTarget.current = { kind: "added", id: instance.id };
      restoreFocus();
    }, fail);
  }

  function share(id: WidgetId) {
    if (space === undefined) return;
    setError(null);
    focusTarget.current = null;
    shareWidget({ id, workspace: space }).then(() => {
      focusTarget.current = { kind: "added", id };
      restoreFocus();
    }, fail);
  }

  /** Removes `instance` (`remove_widget`). A note with text, stored or
   * typed and not saved yet (`draftOf`), or one that can't be read, asks
   * first in its row; the others go at once. */
  function askToRemove(instance: WidgetInstance, index: number) {
    setError(null);
    if (instance.kind !== "note") {
      remove(instance.id, index);
      return;
    }
    if (draftOf(instance.id) !== null) {
      setConfirming(instance.id);
      return;
    }
    setRemoving(instance.id);
    // After the saves still running, so a text just typed counts.
    readNote(instance.id).then(
      (body) => {
        if (body === "") {
          remove(instance.id, index);
        } else {
          setRemoving(null);
          setConfirming(instance.id);
        }
      },
      () => {
        setRemoving(null);
        setConfirming(instance.id);
      },
    );
  }

  function remove(id: WidgetId, index: number) {
    setError(null);
    setRemoving(id);
    focusTarget.current = { kind: "removed", id, index };
    removeWidget({ id, workspace: space }).then(
      () => {
        // The unsaved text of a note goes with it.
        forgetNote(id);
        // The focus moves once the snapshot without it comes (the layout
        // effect runs again on `removing` and on the snapshot).
        setConfirming(null);
        setRemoving(null);
      },
      (reason: unknown) => {
        fail(reason);
        focusTarget.current = { kind: "remove", id };
        setConfirming(null);
        setRemoving(null);
      },
    );
  }

  function cancelRemove(id: WidgetId) {
    focusTarget.current = { kind: "remove", id };
    setConfirming(null);
  }

  function dismiss() {
    setError(null);
    setDismissing(true);
    focusTarget.current = { kind: "back" };
    dismissStorageNotice().then(
      () => {
        setDismissing(false);
        restoreFocus();
      },
      (reason: unknown) => {
        setDismissing(false);
        fail(reason);
      },
    );
  }

  return (
    <>
      {onBack ? (
        <PanelHeader
          title={t.widgets.items.title}
          onBack={onBack}
          focusAction
        />
      ) : (
        <PanelHeader
          title={catalog ? t.widgets.catalog.inUse : t.widgets.items.title}
        />
      )}
      {storage === "recovered" ? (
        <InlineNotice
          title={t.widgets.items.recovered.title}
          detail={t.widgets.items.recovered.detail}
          action={{
            label: t.widgets.items.recovered.dismiss,
            onClick: dismiss,
            disabled: dismissing,
          }}
        />
      ) : null}
      {storage === "unavailable" ? (
        <InlineNotice
          title={t.widgets.items.unavailableNotice.title}
          detail={t.widgets.items.unavailableNotice.detail}
        />
      ) : null}
      {available ? (
        <>
          {widgets.length > 0 ? (
            <ul
              className="dock-items-list"
              aria-label={t.widgets.items.listLabel}
            >
              {widgets.map((instance, index) => {
                const name = widgetName(t, instance);
                const subtitle = subtitleOf(t, instance, noteLines);
                return (
                  <li key={instance.id} className="dock-items-row">
                    {isCreatableInstance(instance) ? (
                      <MiniTile kind={instance.kind} />
                    ) : (
                      <span />
                    )}
                    <div className="dock-items-text">
                      <div className="dock-items-name">{name}</div>
                      {subtitle ? (
                        <div className="dock-items-detail">{subtitle}</div>
                      ) : null}
                    </div>
                    {(["up", "down"] as const).map((direction) => (
                      <IconButton
                        key={direction}
                        ref={(button) => {
                          const key = `${instance.id}:${direction}`;
                          if (button) moveButtons.current.set(key, button);
                          else moveButtons.current.delete(key);
                        }}
                        icon={direction}
                        label={format(
                          direction === "up"
                            ? t.widgets.common.moveUp
                            : t.widgets.common.moveDown,
                          { name },
                        )}
                        disabled={
                          direction === "up"
                            ? index === 0
                            : index === widgets.length - 1
                        }
                        onClick={() => move(instance.id, direction)}
                      />
                    ))}
                    {confirming === instance.id ? (
                      <div className="dock-items-confirm">
                        <InlineConfirm
                          title={t.widgets.note.confirmRemove.title}
                          detail={t.widgets.note.confirmRemove.detail}
                          cancelLabel={t.widgets.note.confirmRemove.cancel}
                          confirmLabel={t.widgets.note.confirmRemove.confirm}
                          onCancel={() => cancelRemove(instance.id)}
                          onConfirm={() => remove(instance.id, index)}
                          busy={removing === instance.id}
                        />
                      </div>
                    ) : (
                      <IconButton
                        ref={(button) => {
                          const key = `${instance.id}:remove`;
                          if (button) moveButtons.current.set(key, button);
                          else moveButtons.current.delete(key);
                        }}
                        icon="trash"
                        tone="danger"
                        label={format(t.widgets.common.removeNamed, { name })}
                        disabled={removing === instance.id}
                        onClick={() => askToRemove(instance, index)}
                      />
                    )}
                  </li>
                );
              })}
            </ul>
          ) : (
            <p className="dock-items-empty">{t.widgets.items.empty}</p>
          )}
          {others.length > 0 ? (
            <div
              className="dock-items-add"
              role="group"
              aria-labelledby="dock-items-share-label"
            >
              <div className="dock-items-add-head">
                <span
                  id="dock-items-share-label"
                  className="dock-items-add-label"
                >
                  {t.widgets.items.fromOthers}
                </span>
              </div>
              <div className="dock-items-add-buttons">
                {others.map((instance) => (
                  <button
                    key={instance.id}
                    type="button"
                    className="secondary-button"
                    disabled={full}
                    onClick={() => share(instance.id)}
                  >
                    {isCreatableInstance(instance) ? (
                      <MiniTile kind={instance.kind} small />
                    ) : null}
                    {widgetName(t, instance)}
                  </button>
                ))}
              </div>
            </div>
          ) : null}
          {catalog ? null : (
            <div
              className="dock-items-add"
              role="group"
              aria-labelledby="dock-items-add-label"
            >
              <div className="dock-items-add-head">
                <span
                  id="dock-items-add-label"
                  className="dock-items-add-label"
                >
                  {t.widgets.items.add}
                </span>
                <span className="dock-items-count">
                  {full
                    ? format(t.widgets.items.limit, { max: MAX_WIDGETS })
                    : format(t.widgets.items.count, {
                        count: widgets.length,
                        max: MAX_WIDGETS,
                      })}
                </span>
              </div>
              <div className="dock-items-add-buttons">
                {CREATABLE_KINDS.map((kind, index) => (
                  <button
                    key={kind}
                    ref={index === 0 ? addButton : undefined}
                    type="button"
                    className="secondary-button"
                    disabled={full}
                    onClick={() => add(kind)}
                  >
                    <MiniTile kind={kind} small />
                    {t.widgets.addNames[kind]}
                  </button>
                ))}
              </div>
            </div>
          )}
        </>
      ) : null}
      <PanelFooter alert={error && t.widgets.errors[error]}>
        {available ? t.widgets.items.footer : t.widgets.items.footerUnavailable}
      </PanelFooter>
    </>
  );
}

export default DockItems;
