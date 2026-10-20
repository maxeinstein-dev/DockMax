import { describe, expect, expectTypeOf, it } from "vitest";
import {
  CLOCK_ICON_STYLES,
  CREATABLE_KINDS,
  HOUR_CYCLES,
  MOVE_DIRECTIONS,
  PERIODS,
  SHORTCUT_TARGET_TYPES,
  STORAGE_STATES,
  UNAVAILABLE_REASONS,
  WEEK_STARTS,
  WIDGET_COMMANDS,
  WIDGET_ERROR_CODES,
  WIDGET_KINDS,
  WIDGETS_CHANGED_EVENT,
  type ClockIconStyle,
  type HourCycle,
  type MoveDirection,
  type Period,
  type ShortcutTarget,
  type StorageStatus,
  type UnavailableReason,
  type WeekStart,
  type WidgetCommand,
  type WidgetCommandSignatures,
  type WidgetKind,
} from "./widgets";
import { readSources } from "../test/sources";
import { COMMAND_ARGS_SAMPLES, WIDGET_FIXTURES } from "./widgets.fixtures";

/* The Rust × TS contract (spec 0003 §2.6). The JSON files are what Rust
 * serializes (a Rust test compares them with the serialization); the TS
 * fixtures are typed against `widgets.ts`. Changing one side without the
 * other breaks this test or the Rust one. */

const DIR = "/src-tauri/tests/fixtures/widgets/";

const files = readSources("src-tauri/tests/fixtures/widgets", /\.json$/);

/** File name without folder or `.json`, as the keys of `WIDGET_FIXTURES`. */
function nameOf(path: string): string {
  return path.slice(DIR.length, -".json".length);
}

const parsed = new Map(
  Object.entries(files).map(([path, text]) => [
    nameOf(path),
    JSON.parse(text) as unknown,
  ]),
);

describe("widget contract fixtures", () => {
  it("finds the JSON fixtures", () => {
    expect(parsed.size).toBeGreaterThan(0);
  });

  it("has one TS fixture per JSON file and no other", () => {
    expect([...parsed.keys()].sort()).toEqual(
      Object.keys(WIDGET_FIXTURES).sort(),
    );
  });

  it.each(Object.entries(WIDGET_FIXTURES))(
    "%s matches its JSON file",
    (name, fixture) => {
      expect(parsed.get(name)).toEqual(fixture);
    },
  );
});

describe("widget closed sets", () => {
  const enums = parsed.get("enums") as Record<string, string[]>;

  it("lists each set once, with no value twice", () => {
    expect(Object.keys(enums).sort()).toEqual(
      Object.keys(WIDGET_FIXTURES.enums).sort(),
    );
    for (const [name, values] of Object.entries(enums)) {
      expect(new Set(values).size, name).toBe(values.length);
    }
  });

  it("has a tuple that is exactly each type", () => {
    expectTypeOf<WidgetKind>().toEqualTypeOf<(typeof WIDGET_KINDS)[number]>();
    expectTypeOf<HourCycle>().toEqualTypeOf<(typeof HOUR_CYCLES)[number]>();
    expectTypeOf<ClockIconStyle>().toEqualTypeOf<
      (typeof CLOCK_ICON_STYLES)[number]
    >();
    expectTypeOf<Period>().toEqualTypeOf<(typeof PERIODS)[number]>();
    expectTypeOf<WeekStart>().toEqualTypeOf<(typeof WEEK_STARTS)[number]>();
    expectTypeOf<MoveDirection>().toEqualTypeOf<
      (typeof MOVE_DIRECTIONS)[number]
    >();
    expectTypeOf<UnavailableReason>().toEqualTypeOf<
      (typeof UNAVAILABLE_REASONS)[number]
    >();
    expectTypeOf<StorageStatus["state"]>().toEqualTypeOf<
      (typeof STORAGE_STATES)[number]
    >();
    expectTypeOf<ShortcutTarget["type"]>().toEqualTypeOf<
      (typeof SHORTCUT_TARGET_TYPES)[number]
    >();
  });

  it("matches the Rust enums, in the order of their ALL", () => {
    expect(enums).toEqual({
      WidgetKind: [...WIDGET_KINDS],
      HourCycle: [...HOUR_CYCLES],
      ClockIconStyle: [...CLOCK_ICON_STYLES],
      Period: [...PERIODS],
      WeekStart: [...WEEK_STARTS],
      MoveDirection: [...MOVE_DIRECTIONS],
      UnavailableReason: [...UNAVAILABLE_REASONS],
      StorageState: [...STORAGE_STATES],
      ShortcutTargetType: [...SHORTCUT_TARGET_TYPES],
      WidgetErrorCode: WIDGET_ERROR_CODES.filter((code) => code !== "notReady"),
    });
  });

  it("sends every error code Rust sends, plus the TS-only notReady", () => {
    expect([...enums.WidgetErrorCode!, "notReady"].sort()).toEqual(
      [...WIDGET_ERROR_CODES].sort(),
    );
  });
});

describe("widget commands and event", () => {
  const ipc = parsed.get("ipc") as {
    event: string;
    commands: Record<string, string[]>;
  };

  it("listens to the event Rust emits", () => {
    expect(ipc.event).toBe(WIDGETS_CHANGED_EVENT);
  });

  it("invokes exactly the commands Rust has", () => {
    expect(Object.keys(ipc.commands).sort()).toEqual(
      Object.values(WIDGET_COMMANDS).sort(),
    );
  });

  it("sends each command the argument names Rust reads", () => {
    for (const [command, args] of Object.entries(COMMAND_ARGS_SAMPLES)) {
      expect(Object.keys(args), command).toEqual(ipc.commands[command]);
    }
  });
});

describe("widget contract constants", () => {
  it("only clock, time progress and note can be created", () => {
    expect(CREATABLE_KINDS).toEqual(["clock", "time-progress", "note"]);
    expect(CREATABLE_KINDS.every((kind) => WIDGET_KINDS.includes(kind))).toBe(
      true,
    );
  });

  it("describes every command and nothing else", () => {
    expectTypeOf<
      keyof WidgetCommandSignatures
    >().toEqualTypeOf<WidgetCommand>();
  });
});
