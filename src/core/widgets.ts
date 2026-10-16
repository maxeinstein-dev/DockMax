import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { errorKind, log } from "./log";

/* Widget instances (spec 0003 §2.1, §2.5–§2.7). Must match
 * `src-tauri/src/domain/widgets/`; the JSON in
 * `src-tauri/tests/fixtures/widgets/` pins the shapes on both sides
 * (`widgets.contract.test.ts`). Rust is the source of truth: the windows
 * read the instances with `list_widgets`, apply what `widgets-changed`
 * carries, and only the panel changes them. */

/** Most widget instances the dock holds; checked in Rust. */
export const MAX_WIDGETS = 12;

/** Largest note text accepted, in UTF-8 bytes; checked in Rust. */
export const MAX_NOTE_BYTES = 64 * 1024;

/** Longest instance label accepted, in Unicode scalar values (code points),
 * not UTF-16 units: `label.length` counts an emoji outside the BMP as 2, Rust
 * as 1, so count with `labelLength`. Checked in Rust. */
export const MAX_LABEL_CHARS = 40;

/** Length of `label` as Rust counts it for `MAX_LABEL_CHARS`: Unicode
 * scalar values (code points), not UTF-16 units and not graphemes. An emoji
 * outside the BMP is 1 (`.length` says 2); "é" written as "e" plus a
 * combining accent is 2. A lone surrogate counts as 1, as `[...label]`
 * yields it; Rust never receives one (the IPC carries UTF-8). */
export function labelLength(label: string): number {
  // A string's iterator walks it by code points.
  return Array.from(label).length;
}

/** The kinds of widget: a closed set, no third-party widgets. */
export type WidgetKind =
  "clock" | "time-progress" | "note" | "shortcut" | "files";

/* Each closed set below has a tuple with every value, in the order of the
 * `ALL` of the Rust enum. `widgets.contract.test.ts` checks that each tuple
 * is exactly its type and matches `enums.json`, which Rust checks against
 * its enums: a value added on one side alone breaks one of the two tests. */

/** Every kind, in the order of `WidgetKind::ALL` in Rust. */
export const WIDGET_KINDS = [
  "clock",
  "time-progress",
  "note",
  "shortcut",
  "files",
] as const;

/** The kinds `add_widget` accepts in this phase; Shortcut and Files exist
 * only in the domain until phase 3. Must match `WidgetKind::creatable`. */
export const CREATABLE_KINDS: readonly WidgetKind[] = [
  "clock",
  "time-progress",
  "note",
];

/** Stable ID of an instance: a UUID v7 made in Rust, never reused and never
 * changed by reordering, changing or restarting. */
export type WidgetId = string;

/** How hours are counted, named as in `Intl.DateTimeFormat`'s `hourCycle`. */
export type HourCycle = "h23" | "h12";

export const HOUR_CYCLES = ["h23", "h12"] as const;

/** How a clock's dock icon shows the time: `digital` (`HH:mm`, or `h:mm`
 * in 12 h without AM/PM) or `analog` (white hands on the glaze). */
export type ClockIconStyle = "digital" | "analog";

export const CLOCK_ICON_STYLES = ["digital", "analog"] as const;

export interface ClockConfig {
  /** IANA time zone name, such as `America/Sao_Paulo`; null follows the
   * system time zone. */
  timeZone: string | null;
  hourCycle: HourCycle;
  /** Seconds in the panel only; the dock icon never shows them. */
  showSeconds: boolean;
  /** How the dock icon shows the time. Rust reads a stored clock without
   * it as `digital`; the interface always gets and sends it. */
  iconStyle: ClockIconStyle;
}

export type Period = "day" | "week" | "month" | "year" | "workday";

export const PERIODS = ["day", "week", "month", "year", "workday"] as const;

/** First day of the week for the `week` period. */
export type WeekStart = "monday" | "sunday";

export const WEEK_STARTS = ["monday", "sunday"] as const;

/** Workday hours, both on the same day, as local `HH:mm`. */
export interface Workday {
  start: string;
  end: string;
}

export interface TimeProgressConfig {
  /** Periods in the order the panel shows them; the dock icon's ring shows
   * the first one. */
  periods: Period[];
  weekStart: WeekStart;
  /** Used only when `periods` has `workday`. */
  workday: Workday;
}

/** A note has nothing to set: its text is read with `get_note`. Always
 * `{}`. */
export type NoteConfig = Record<string, never>;

/** What a shortcut opens: an absolute Windows path or an http(s) URL. */
export type ShortcutTarget =
  { type: "path"; path: string } | { type: "url"; url: string };

/** Every `type` of a `ShortcutTarget`. */
export const SHORTCUT_TARGET_TYPES = ["path", "url"] as const;

export interface ShortcutConfig {
  target: ShortcutTarget;
}

export interface FilesConfig {
  /** Absolute Windows path of the folder. */
  folder: string;
}

/** The config of each kind. */
export interface WidgetConfigs {
  clock: ClockConfig;
  "time-progress": TimeProgressConfig;
  note: NoteConfig;
  shortcut: ShortcutConfig;
  files: FilesConfig;
}

/** A kind's config tagged with the kind, as `update_widget` takes it:
 * `{ kind: "clock", config: {...} }`. */
export type WidgetConfig<K extends WidgetKind = WidgetKind> = {
  [P in K]: { kind: P; config: WidgetConfigs[P] };
}[K];

/** A widget in the dock. There is no position: a list of instances is
 * always in dock order. */
export type WidgetInstance<K extends WidgetKind = WidgetKind> = {
  [P in K]: {
    id: WidgetId;
    /** Shown instead of the kind's name; null uses the kind's name. */
    label: string | null;
    kind: P;
    config: WidgetConfigs[P];
  };
}[K];

/** Why the widgets can't be used in this run. */
export type UnavailableReason =
  "newerVersion" | "migrationFailed" | "openFailed";

export const UNAVAILABLE_REASONS = [
  "newerVersion",
  "migrationFailed",
  "openFailed",
] as const;

/** Whether the widget database can be used in this run (spec 0003 §2.4
 * item 7). `recovered`: a corrupt database was set aside and a new one made;
 * the notice stays until `dismiss_storage_notice`. `unavailable`: the dock
 * shows only Settings and About. */
export type StorageStatus =
  | { state: "ok" }
  | { state: "recovered" }
  | { state: "unavailable"; reason: UnavailableReason };

/** Every `state` of a `StorageStatus`. */
export const STORAGE_STATES = ["ok", "recovered", "unavailable"] as const;

/** What `list_widgets` returns and `widgets-changed` carries. */
export interface WidgetsSnapshot {
  storage: StorageStatus;
  /** Every instance, in dock order. */
  widgets: WidgetInstance[];
}

/** One place towards the start (`up`) or the end (`down`) of the dock. */
export type MoveDirection = "up" | "down";

export const MOVE_DIRECTIONS = ["up", "down"] as const;

/** Why a widget command failed; must match `WidgetErrorCode` in
 * `src-tauri/src/domain/widgets/mod.rs`, plus `notReady`. */
export type WidgetErrorCode =
  | "limitReached"
  | "notCreatable"
  | "notFound"
  | "kindMismatch"
  | "invalidLabel"
  | "invalidTimeZone"
  | "invalidPeriods"
  | "invalidWorkday"
  | "invalidTarget"
  | "invalidFolder"
  | "noteTooLarge"
  /** A stored config has another version or is not valid JSON, or the
   * config given to `update_widget` is not exactly the canonical one (a
   * missing, unknown or mistyped field). */
  | "invalidConfig"
  /** The widget database can't be used in this run. */
  | "unavailable"
  | "writeFailed"
  /** TS only: the command never ran (payload refused by Tauri, state
   * missing). */
  | "notReady";

/** Every code, from a record so a new one can't be left out. */
export const WIDGET_ERROR_CODES: readonly WidgetErrorCode[] = Object.keys({
  limitReached: true,
  notCreatable: true,
  notFound: true,
  kindMismatch: true,
  invalidLabel: true,
  invalidTimeZone: true,
  invalidPeriods: true,
  invalidWorkday: true,
  invalidTarget: true,
  invalidFolder: true,
  noteTooLarge: true,
  invalidConfig: true,
  unavailable: true,
  writeFailed: true,
  notReady: true,
} satisfies Record<WidgetErrorCode, true>) as WidgetErrorCode[];

/** What a widget command rejects with. No path, no value: only the code. */
export interface WidgetCommandError {
  code: WidgetErrorCode;
}

const ERROR_CODES: ReadonlySet<string> = new Set(WIDGET_ERROR_CODES);

/** Whether `error` is the `{ code }` a widget command rejects with. */
export function isWidgetCommandError(
  error: unknown,
): error is WidgetCommandError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string" &&
    ERROR_CODES.has(error.code)
  );
}

/** The widget commands. The dock only reads (`list_widgets`); the panel
 * reads and writes. */
export const WIDGET_COMMANDS = {
  list: "list_widgets",
  add: "add_widget",
  remove: "remove_widget",
  move: "move_widget",
  update: "update_widget",
  getNote: "get_note",
  saveNote: "save_note",
  dismissStorageNotice: "dismiss_storage_notice",
} as const;

export type WidgetCommand =
  (typeof WIDGET_COMMANDS)[keyof typeof WIDGET_COMMANDS];

/** Event every window receives with a `WidgetsSnapshot` after a change. */
export const WIDGETS_CHANGED_EVENT = "widgets-changed";

/** Arguments of `add_widget`: the kind must be in `CREATABLE_KINDS`. */
export interface AddWidgetArgs {
  kind: WidgetKind;
}

/** Arguments of `remove_widget`. A note's text goes with it. */
export interface RemoveWidgetArgs {
  id: WidgetId;
}

/** Arguments of `move_widget`. Moving the first up or the last down does
 * nothing. */
export interface MoveWidgetArgs {
  id: WidgetId;
  direction: MoveDirection;
}

/** Arguments of `update_widget`: the full label and config. `config.kind`
 * must be the instance's kind (`kindMismatch` otherwise). */
export interface UpdateWidgetArgs {
  id: WidgetId;
  label: string | null;
  config: WidgetConfig;
}

/** Arguments of `get_note`. */
export interface GetNoteArgs {
  id: WidgetId;
}

/** Arguments of `save_note`: the body is at most `MAX_NOTE_BYTES` in UTF-8. */
export interface SaveNoteArgs {
  id: WidgetId;
  body: string;
}

/** Arguments and result of each command, by name. */
export interface WidgetCommandSignatures {
  list_widgets: { args: Record<string, never>; result: WidgetsSnapshot };
  add_widget: { args: AddWidgetArgs; result: WidgetInstance };
  remove_widget: { args: RemoveWidgetArgs; result: void };
  move_widget: { args: MoveWidgetArgs; result: void };
  update_widget: { args: UpdateWidgetArgs; result: WidgetInstance };
  get_note: { args: GetNoteArgs; result: string };
  save_note: { args: SaveNoteArgs; result: void };
  dismiss_storage_notice: { args: Record<string, never>; result: void };
}

type WidgetArgs<C extends WidgetCommand> = WidgetCommandSignatures[C]["args"];
type WidgetResult<C extends WidgetCommand> =
  WidgetCommandSignatures[C]["result"];

/** Runs `command`. A known `{ code }` from Rust passes through; any other
 * failure (payload refused by Tauri, state missing) has no code of its own:
 * `logFailure` logs its kind and it rejects with `notReady`. */
function run<C extends WidgetCommand>(
  command: C,
  args: WidgetArgs<C>,
  logFailure: (error: unknown) => void,
): Promise<WidgetResult<C>> {
  return invoke<WidgetResult<C>>(command, { ...args }).catch(
    (error: unknown) => {
      if (isWidgetCommandError(error)) throw error;
      logFailure(error);
      const notReady: WidgetCommandError = { code: "notReady" };
      throw notReady;
    },
  );
}

/* Every command below rejects with a `WidgetCommandError` only, and nothing
 * changes when it does. The log messages name the action and the kind of
 * error: never an ID, a label, a config or a note's text. */

/** The instances in dock order, with the state of the storage. */
export function listWidgets(): Promise<WidgetsSnapshot> {
  return run(WIDGET_COMMANDS.list, {}, (error) =>
    log.warn(`widgets: list failed: ${errorKind(error)}`),
  );
}

/** Adds an instance of `kind` (one of `CREATABLE_KINDS`) at the end of the
 * dock and returns it. */
export function addWidget(args: AddWidgetArgs): Promise<WidgetInstance> {
  return run(WIDGET_COMMANDS.add, args, (error) =>
    log.warn(`widgets: add failed: ${errorKind(error)}`),
  );
}

/** Removes an instance; a note's text goes with it. */
export function removeWidget(args: RemoveWidgetArgs): Promise<void> {
  return run(WIDGET_COMMANDS.remove, args, (error) =>
    log.warn(`widgets: remove failed: ${errorKind(error)}`),
  );
}

/** Moves an instance one place up or down the dock. */
export function moveWidget(args: MoveWidgetArgs): Promise<void> {
  return run(WIDGET_COMMANDS.move, args, (error) =>
    log.warn(`widgets: move failed: ${errorKind(error)}`),
  );
}

/** Replaces an instance's label and config, and returns the instance. */
export function updateWidget(args: UpdateWidgetArgs): Promise<WidgetInstance> {
  return run(WIDGET_COMMANDS.update, args, (error) =>
    log.warn(`widgets: update failed: ${errorKind(error)}`),
  );
}

/** The text of a note. */
export function getNote(args: GetNoteArgs): Promise<string> {
  return run(WIDGET_COMMANDS.getNote, args, (error) =>
    log.warn(`widgets: note read failed: ${errorKind(error)}`),
  );
}

/** Saves the full text of a note. */
export function saveNote(args: SaveNoteArgs): Promise<void> {
  return run(WIDGET_COMMANDS.saveNote, args, (error) =>
    log.warn(`widgets: note save failed: ${errorKind(error)}`),
  );
}

/** Dismisses the notice of a recovered database. */
export function dismissStorageNotice(): Promise<void> {
  return run(WIDGET_COMMANDS.dismissStorageNotice, {}, (error) =>
    log.warn(`widgets: notice dismiss failed: ${errorKind(error)}`),
  );
}

/** The instances and storage state, kept current by `widgets-changed`; null
 * while loading, and for good if they can't be read (the dock then shows
 * only Settings and About). */
export function useWidgets(): WidgetsSnapshot | null {
  const [snapshot, setSnapshot] = useState<WidgetsSnapshot | null>(null);

  useEffect(() => {
    let active = true;
    // A change may arrive before the initial read answers; the read is
    // older then, so it is dropped.
    let changed = false;

    const unlisten = listen<WidgetsSnapshot>(WIDGETS_CHANGED_EVENT, (event) => {
      if (!active) return;
      changed = true;
      setSnapshot(event.payload);
    });

    listWidgets().then(
      (current) => {
        if (active && !changed) setSnapshot(current);
      },
      () => {
        // `listWidgets` already logged the kind of an unexpected failure.
        if (active) log.warn("widgets: read failed");
      },
    );

    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);

  return snapshot;
}
