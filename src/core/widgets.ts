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
  | "clock"
  | "time-progress"
  | "note"
  | "shortcut"
  | "files"
  | "activity"
  | "network"
  | "calendar"
  | "cycle"
  | "countdown"
  | "pomodoro"
  | "todo"
  | "water"
  | "media"
  | "claude";

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
  "activity",
  "network",
  "calendar",
  "cycle",
  "countdown",
  "pomodoro",
  "todo",
  "water",
  "media",
  "claude",
] as const;

/** The kinds `add_widget` accepts in this phase. */
export type CreatableKind = Exclude<WidgetKind, "shortcut" | "files">;

/** The kinds `add_widget` accepts in this phase, in the order of the add
 * buttons; Shortcut and Files exist only in the domain until phase 3. Must
 * match `WidgetKind::creatable`. */
export const CREATABLE_KINDS: readonly CreatableKind[] = [
  "clock",
  "time-progress",
  "note",
  "activity",
  "network",
  "calendar",
  "cycle",
  "countdown",
  "pomodoro",
  "todo",
  "water",
  "media",
  "claude",
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
  /** In icons: the first period's ring, two rings, or three bars. */
  width: WidgetWidth;
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

/** What the Atividade shows (spec 0017 F1). */
export type ActivityShow = "both" | "cpu" | "memory";

export const ACTIVITY_SHOWS = ["both", "cpu", "memory"] as const;

/** How the Atividade draws its values in the dock. */
export type ActivityStyle = "bars" | "rings" | "numbers";

export const ACTIVITY_STYLES = ["bars", "rings", "numbers"] as const;

/** The widths, in icons, of the kinds that show more when wider: time
 * progress, Atividade and Atividade de rede (spec 0017 F1b); checked in
 * Rust. The others are always 1 icon wide. */
export const WIDGET_WIDTHS = [1, 2, 3] as const;

export type WidgetWidth = (typeof WIDGET_WIDTHS)[number];

export interface ActivityConfig {
  show: ActivityShow;
  style: ActivityStyle;
  /** In icons. */
  width: WidgetWidth;
}

/** Atividade de rede: one icon shows the two rates, two add the graph. */
export interface NetworkConfig {
  /** In icons. */
  width: WidgetWidth;
}

/** The Calendário (spec 0017 F2a): its width, and the calendars it leaves
 * out; every other calendar of the connected accounts shows. */
/** The widths of the Calendário: three was too small to read; two rise
 * to three under the pointer. */
export const CALENDAR_WIDTHS = [1, 2] as const;

export type CalendarWidth = (typeof CALENDAR_WIDTHS)[number];

/** How far ahead the Calendário's card looks (F2a-4): `auto` is the day,
 * or, after 18h, tomorrow until noon. */
export type CardPeriod = "auto" | "day" | "twoDays" | "threeDays" | "week";

export const CARD_PERIODS = [
  "auto",
  "day",
  "twoDays",
  "threeDays",
  "week",
] as const;

export interface CalendarConfig {
  /** In icons: the next meeting's time; its time and title (rising to
   * three icons under the pointer, with the one after). */
  width: CalendarWidth;
  /** Google's ids of the calendars not shown; at most 200. */
  hidden: string[];
  /** Whether the panel shows Google Tasks' tasks (F2a-3). */
  tasks: boolean;
  /** The tasks' order given in the panel, by title: these first, then
   * the others in Google's order. */
  taskOrder: string[];
  /** The card over the dock: how far ahead it looks, and whether it lists
   * the day's tasks under the meetings. */
  cardPeriod: CardPeriod;
  cardTasks: boolean;
}

/** The Ciclo (spec 0017 F2b): the symbol of the marks in her calendar, the
 * calendar (null: all of them) and the sensitive window, in days. */
export interface CycleConfig {
  symbol: string;
  calendar: string | null;
  /** 1 to 10; checked in Rust. */
  sensitive: number;
  /** The Windows notices kept (F2b-2). */
  notices: CycleNotices;
  /** The author's own reminders (F2b-2). */
  reminders: CycleReminder[];
}

/** The sensitive windows the settings offer, in days. */
export const CYCLE_WINDOWS = [3, 4, 5, 6, 7] as const;

/** The Ciclo's notices: when the sensitive window opens, the eve of the
 * expected start, and the day itself. They show on the dock (a badge and a
 * bounce of the tile); `windows` adds a notice of Windows, off by default. */
export interface CycleNotices {
  sensitive: boolean;
  eve: boolean;
  day: boolean;
  windows: boolean;
}

/** What a reminder counts from. Must match `Anchor` in Rust. */
export type ReminderAnchor = "sensitive" | "period";

export const REMINDER_ANCHORS = ["sensitive", "period"] as const;

/** A reminder: `text`, `days` before (negative) or after its `anchor`. */
export interface CycleReminder {
  text: string;
  anchor: ReminderAnchor;
  /** -10 to 10; checked in Rust. */
  days: number;
}

/** Most reminders and the longest text of one; checked in Rust. */
export const MAX_REMINDERS = 10;
export const MAX_REMINDER_CHARS = 120;
/** How far from its phase a reminder may fall, in days. */
export const REMINDER_DAYS = 10;

/** The Contagem regressiva (spec 0017 F2c): the day, written `2026-12-20`
 * (null until picked), and whether it comes back every year. Its name is
 * the widget's label. */
export interface CountdownConfig {
  date: string | null;
  yearly: boolean;
}

/** The Pomodoro (spec 0017 F2c-2): minutes of focus (1 to 120), of a pause
 * and of the long one (1 to 60), focuses before it (2 to 8), whether the
 * next stage starts by itself, and the notice of Windows. Checked in Rust. */
export interface PomodoroConfig {
  focus: number;
  short: number;
  long: number;
  every: number;
  autoStart: boolean;
  windows: boolean;
}

/** The values the Pomodoro's settings offer. */
export const POMODORO_CHOICES = {
  focus: [15, 20, 25, 30, 40, 45, 50, 60, 90],
  short: [3, 5, 10, 15],
  long: [10, 15, 20, 30],
  every: [2, 3, 4, 5, 6],
} as const;

/** A task of the Lista de tarefas: an ID the panel makes, its text and
 * whether it is done. */
export interface TodoItem {
  id: string;
  text: string;
  done: boolean;
}

/** The Lista de tarefas (spec 0017 F3): the tasks, in the author's order.
 * Checked in Rust. */
export interface TodoConfig {
  items: TodoItem[];
}

/** Most tasks in a list, and the longest text of one. */
export const MAX_TASKS = 200;
export const MAX_TASK_CHARS = 200;

/** The Beber água (spec 0017 F3-2): the goal in cups (1 to 30), a cup in
 * milliliters (50 to 1000), the reminder in minutes (0: off, or 15 to 240),
 * and the cups of `day` (a local day number), the last at `last` (Unix
 * seconds). Checked in Rust. */
export interface WaterConfig {
  goal: number;
  cup: number;
  remind: number;
  count: number;
  day: number;
  last: number | null;
}

/** The values the Beber água's settings offer. */
export const WATER_CHOICES = {
  goal: [4, 5, 6, 7, 8, 9, 10, 12, 14, 16],
  cup: [150, 200, 250, 300, 350, 400, 500],
  remind: [0, 30, 45, 60, 90, 120],
} as const;

/** The Tocando agora (spec 0017 F4): only the width, 1 to 3 icons; what
 * plays comes from Windows. */
export interface MediaConfig {
  width: WidgetWidth;
}

/** The widths the Uso do Claude takes: the five hours' ring, or both
 * limits. Must match `claude::WIDTHS`. */
export const CLAUDE_WIDTHS = [1, 2] as const;

export type ClaudeWidth = (typeof CLAUDE_WIDTHS)[number];

/** The Uso do Claude (spec 0017 F5): only the width; the usage comes from
 * Anthropic, read in Rust. */
export interface ClaudeConfig {
  width: ClaudeWidth;
}

/** The config of each kind. */
export interface WidgetConfigs {
  clock: ClockConfig;
  "time-progress": TimeProgressConfig;
  note: NoteConfig;
  shortcut: ShortcutConfig;
  files: FilesConfig;
  activity: ActivityConfig;
  network: NetworkConfig;
  calendar: CalendarConfig;
  cycle: CycleConfig;
  countdown: CountdownConfig;
  pomodoro: PomodoroConfig;
  todo: TodoConfig;
  water: WaterConfig;
  media: MediaConfig;
  claude: ClaudeConfig;
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
  /** A width the kind doesn't accept. */
  | "invalidWidth"
  /** A hidden calendar or a task of the order empty, too long or with a
   * control character, or too many. */
  | "invalidCalendars"
  /** The Ciclo's symbol, calendar or window out of bounds. */
  | "invalidCycle"
  /** The date of the Contagem regressiva not a day written `2026-12-20`. */
  | "invalidCountdown"
  /** The times of the Pomodoro out of their ranges. */
  | "invalidPomodoro"
  /** A task empty, too long, with a control character or an ID used
   * twice, or too many tasks. */
  | "invalidTasks"
  /** The goal, cup, reminder or count of the Beber água out of range. */
  | "invalidWater"
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
  invalidWidth: true,
  invalidCalendars: true,
  invalidCycle: true,
  invalidCountdown: true,
  invalidPomodoro: true,
  invalidTasks: true,
  invalidWater: true,
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
  share: "share_widget",
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
  /** The workspace it goes to (spec 0013); without it, the one in use. */
  workspace?: string;
}

/** Arguments of `remove_widget`. A note's text goes with it. */
export interface RemoveWidgetArgs {
  id: WidgetId;
  /** The workspace it leaves; with no other one showing it, it is removed
   * for good. Without it, the one in use. */
  workspace?: string;
}

/** Arguments of `share_widget` (spec 0013 C3): the instance shows in
 * `workspace` too, still one instance. */
export interface ShareWidgetArgs {
  id: WidgetId;
  workspace: string;
}

/** Arguments of `move_widget`. Moving the first up or the last down does
 * nothing. */
export interface MoveWidgetArgs {
  id: WidgetId;
  direction: MoveDirection;
  /** The workspace whose order changes; without it, the one in use. */
  workspace?: string;
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
  share_widget: { args: ShareWidgetArgs; result: void };
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

/** Shows an instance in another workspace too, at its end. */
export function shareWidget(args: ShareWidgetArgs): Promise<void> {
  return run(WIDGET_COMMANDS.share, args, (error) =>
    log.warn(`widgets: share failed: ${errorKind(error)}`),
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
