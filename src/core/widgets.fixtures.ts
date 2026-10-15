import type {
  StorageStatus,
  WidgetErrorCode,
  WidgetInstance,
  WidgetsSnapshot,
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
  config: { timeZone: null, hourCycle: "h23", showSeconds: false },
} satisfies WidgetInstance<"clock">;

const NOTE_DEFAULT = {
  id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6075",
  label: null,
  kind: "note",
  config: {},
} satisfies WidgetInstance<"note">;

/** Codes Rust can send: every `WidgetErrorCode` but the TS-only one. */
type RustErrorCode = Exclude<WidgetErrorCode, "notReady">;

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

  "error-codes": [
    "limitReached",
    "notCreatable",
    "notFound",
    "kindMismatch",
    "invalidLabel",
    "invalidTimeZone",
    "invalidPeriods",
    "invalidWorkday",
    "invalidTarget",
    "invalidFolder",
    "noteTooLarge",
    "invalidConfig",
    "unavailable",
    "writeFailed",
  ] satisfies RustErrorCode[],
};

export type WidgetFixtureName = keyof typeof WIDGET_FIXTURES;
