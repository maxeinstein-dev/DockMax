import {
  CLOCK_ICON_STYLES,
  HOUR_CYCLES,
  MOVE_DIRECTIONS,
  PERIODS,
  SHORTCUT_TARGET_TYPES,
  STORAGE_STATES,
  UNAVAILABLE_REASONS,
  WEEK_STARTS,
  WIDGET_ERROR_CODES,
  WIDGET_KINDS,
  WIDGETS_CHANGED_EVENT,
  type StorageStatus,
  type WidgetCommand,
  type WidgetCommandSignatures,
  type WidgetInstance,
  type WidgetsSnapshot,
} from "./widgets";

/* The contract fixtures, typed: each entry is the JSON file of the same
 * name in `src-tauri/tests/fixtures/widgets/`, which is exactly what Rust
 * serializes. `satisfies` makes a change to the TS types break here, and
 * `widgets.contract.test.ts` makes a change to the JSON (that is, to Rust)
 * break there. */

const CLOCK_DEFAULT = {
  id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6071",
  label: null,
  kind: "clock",
  config: {
    timeZone: null,
    hourCycle: "h23",
    showSeconds: false,
    iconStyle: "digital",
  },
} satisfies WidgetInstance<"clock">;

const NOTE_DEFAULT = {
  id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6075",
  label: null,
  kind: "note",
  config: {},
} satisfies WidgetInstance<"note">;

/** A workspace (spec 0013) the commands that take one may name. */
const WORKSPACE_ID = "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6079";

/** One example of the arguments of each widget command, with every key.
 * The keys are what `invoke` sends: `satisfies` makes a key too many a type
 * error, and `Required<>` a key left out (even an optional one), so the
 * argument names below are those of the `*Args` types. */
export const COMMAND_ARGS_SAMPLES = {
  list_widgets: {},
  add_widget: { kind: "clock", workspace: WORKSPACE_ID },
  remove_widget: { id: CLOCK_DEFAULT.id, workspace: WORKSPACE_ID },
  move_widget: {
    id: CLOCK_DEFAULT.id,
    direction: "up",
    workspace: WORKSPACE_ID,
  },
  update_widget: {
    id: NOTE_DEFAULT.id,
    label: null,
    config: { kind: "note", config: {} },
  },
  get_note: { id: NOTE_DEFAULT.id },
  save_note: { id: NOTE_DEFAULT.id, body: "" },
  dismiss_storage_notice: {},
} satisfies {
  [C in WidgetCommand]: Required<WidgetCommandSignatures[C]["args"]>;
};

export const WIDGET_FIXTURES = {
  "clock-default": CLOCK_DEFAULT,

  "clock-sample": {
    id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6072",
    label: "São Paulo",
    kind: "clock",
    config: {
      timeZone: "America/Sao_Paulo",
      hourCycle: "h12",
      showSeconds: true,
      iconStyle: "analog",
    },
  } satisfies WidgetInstance<"clock">,

  "time-progress-default": {
    id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6073",
    label: null,
    kind: "time-progress",
    config: {
      periods: ["day", "week", "month", "year"],
      weekStart: "monday",
      workday: { start: "09:00", end: "18:00" },
    },
  } satisfies WidgetInstance<"time-progress">,

  "time-progress-sample": {
    id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6074",
    label: "Trabalho",
    kind: "time-progress",
    config: {
      periods: ["workday", "week", "year"],
      weekStart: "sunday",
      workday: { start: "08:30", end: "17:15" },
    },
  } satisfies WidgetInstance<"time-progress">,

  "note-default": NOTE_DEFAULT,

  "shortcut-path": {
    id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6076",
    label: "Projetos",
    kind: "shortcut",
    config: {
      target: { type: "path", path: "C:\\Users\\Public\\Documents" },
    },
  } satisfies WidgetInstance<"shortcut">,

  "shortcut-url": {
    id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6077",
    label: null,
    kind: "shortcut",
    config: { target: { type: "url", url: "https://example.com/docs" } },
  } satisfies WidgetInstance<"shortcut">,

  "files-sample": {
    id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6078",
    label: null,
    kind: "files",
    config: { folder: "\\\\server\\share\\Downloads" },
  } satisfies WidgetInstance<"files">,

  "snapshot-sample": {
    storage: { state: "ok" },
    widgets: [CLOCK_DEFAULT, { ...NOTE_DEFAULT, label: "Mercado" }],
  } satisfies WidgetsSnapshot,

  "storage-statuses": [
    { state: "ok" },
    { state: "recovered" },
    { state: "unavailable", reason: "newerVersion" },
    { state: "unavailable", reason: "migrationFailed" },
    { state: "unavailable", reason: "openFailed" },
  ] satisfies StorageStatus[],

  /* Every value of each closed set; `WidgetErrorCode` without the TS-only
   * `notReady`, which Rust never sends. */
  enums: {
    WidgetKind: WIDGET_KINDS,
    HourCycle: HOUR_CYCLES,
    ClockIconStyle: CLOCK_ICON_STYLES,
    Period: PERIODS,
    WeekStart: WEEK_STARTS,
    MoveDirection: MOVE_DIRECTIONS,
    UnavailableReason: UNAVAILABLE_REASONS,
    StorageState: STORAGE_STATES,
    ShortcutTargetType: SHORTCUT_TARGET_TYPES,
    WidgetErrorCode: WIDGET_ERROR_CODES.filter((code) => code !== "notReady"),
  },

  /* The event and, for each command, the names of its arguments. */
  ipc: {
    event: WIDGETS_CHANGED_EVENT,
    commands: Object.fromEntries(
      Object.entries(COMMAND_ARGS_SAMPLES).map(([command, args]) => [
        command,
        Object.keys(args),
      ]),
    ),
  },
};

export type WidgetFixtureName = keyof typeof WIDGET_FIXTURES;
