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

/** Longest instance label accepted, in characters; checked in Rust. */
export const MAX_LABEL_CHARS = 40;

/** The kinds of widget: a closed set, no third-party widgets. */
export type WidgetKind =
  "clock" | "time-progress" | "note" | "shortcut" | "files";

/** Every kind, in the order of `WidgetKind::ALL` in Rust. */
export const WIDGET_KINDS: readonly WidgetKind[] = [
  "clock",
  "time-progress",
  "note",
  "shortcut",
  "files",
];

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

export interface ClockConfig {
  /** IANA time zone name, such as `America/Sao_Paulo`; null follows the
   * system time zone. */
  timeZone: string | null;
  hourCycle: HourCycle;
  /** Seconds in the panel only; the dock icon never shows them. */
  showSeconds: boolean;
}

export type Period = "day" | "week" | "month" | "year" | "workday";

/** First day of the week for the `week` period. */
export type WeekStart = "monday" | "sunday";

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

/** Whether the widget database can be used in this run (spec 0003 §2.4
 * item 7). `recovered`: a corrupt database was set aside and a new one made;
 * the notice stays until `dismiss_storage_notice`. `unavailable`: the dock
 * shows only Settings and About. */
export type StorageStatus =
  | { state: "ok" }
  | { state: "recovered" }
  | { state: "unavailable"; reason: UnavailableReason };

/** What `list_widgets` returns and `widgets-changed` carries. */
export interface WidgetsSnapshot {
  storage: StorageStatus;
  /** Every instance, in dock order. */
  widgets: WidgetInstance[];
}

/** One place towards the start (`up`) or the end (`down`) of the dock. */
export type MoveDirection = "up" | "down";

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
  /** A stored config has another version or is not valid JSON. */
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
