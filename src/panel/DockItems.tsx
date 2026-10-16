import { useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  CREATABLE_KINDS,
  MAX_WIDGETS,
  addWidget,
  dismissStorageNotice,
  getNote,
  isWidgetCommandError,
  moveWidget,
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
import PanelFooter from "./PanelFooter";
import IconButton from "./parts/IconButton";
import InlineNotice from "./parts/InlineNotice";
import PanelHeader from "./parts/PanelHeader";
import { PANEL_TITLE_ID } from "./parts/focus";
import "./DockItems.css";

/** Props of Configurações › Itens do dock (W4), a subview of the Settings
 * panel. It changes the list through `addWidget`, `moveWidget` and
 * `dismissStorageNotice` (`src/core/widgets.ts`); every accepted change
 * comes back in `snapshot` through `widgets-changed`, and a rejected one
 * shows its message (`t.widgets.errors[code]`). */
export interface DockItemsProps {
  /** The instances in dock order and the storage state (`useWidgets`);
   * null while loading, and for good when they can't be read. */
  snapshot: WidgetsSnapshot | null;
  /** Voltar: back to Configurações, where the "Itens do dock" row takes the
   * focus (`NavRow` `focusOnMount`). The view mounts with the focus on
   * Voltar. */
  onBack: () => void;
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
};

/** The kind's tile, static: 26 px in the list, 18 px on an add button.
 * Decorative: the name is next to it. */
function MiniTile({
  kind,
  small = false,
}: {
  kind: CreatableKind;
  small?: boolean;
}) {
  const glaze = WIDGET_REGISTRY[kind].glaze;
  return (
    <span
      className={`mini-tile ${glaze}${small ? " small" : ""}`}
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
  | { kind: "back" };

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
 * each with subir and descer, the add buttons with the count, and the
 * notice of a recovered or unavailable storage. Rust is the source of
 * truth: nothing here changes before `widgets-changed` says so.
 */
function DockItems({ snapshot, onBack }: DockItemsProps) {
  const t = useT();
  const [error, setError] = useState<WidgetErrorCode | null>(null);
  const [dismissing, setDismissing] = useState(false);
  const [noteLines, setNoteLines] = useState<Record<WidgetId, number | null>>(
    {},
  );
  const moveButtons = useRef(new Map<string, HTMLButtonElement>());
  const focusTarget = useRef<FocusTarget | null>(null);

  const storage = snapshot?.storage.state ?? null;
  const available = snapshot !== null && storage !== "unavailable";
  const widgets = available ? snapshot.widgets : [];
  const full = widgets.length >= MAX_WIDGETS;

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
  useLayoutEffect(restoreFocus, [snapshot]);

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
    moveWidget({ id, direction }).catch(fail);
  }

  function add(kind: CreatableKind) {
    setError(null);
    focusTarget.current = null;
    addWidget({ kind }).then((instance) => {
      focusTarget.current = { kind: "added", id: instance.id };
      restoreFocus();
    }, fail);
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
      <PanelHeader title={t.widgets.items.title} onBack={onBack} focusAction />
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
                  </li>
                );
              })}
            </ul>
          ) : (
            <p className="dock-items-empty">{t.widgets.items.empty}</p>
          )}
          <div
            className="dock-items-add"
            role="group"
            aria-labelledby="dock-items-add-label"
          >
            <div className="dock-items-add-head">
              <span id="dock-items-add-label" className="dock-items-add-label">
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
              {CREATABLE_KINDS.map((kind) => (
                <button
                  key={kind}
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
        </>
      ) : null}
      <PanelFooter alert={error && t.widgets.errors[error]}>
        {available ? t.widgets.items.footer : t.widgets.items.footerUnavailable}
      </PanelFooter>
    </>
  );
}

export default DockItems;
