import type { ComponentType } from "react";
import {
  CREATABLE_KINDS,
  type CreatableKind,
  type WidgetInstance,
} from "../core/widgets";
import ActivityPanel from "./activity/ActivityPanel";
import ActivitySettings from "./activity/ActivitySettings";
import ActivityTile from "./activity/ActivityTile";
import CalendarPanel from "./calendar/CalendarPanel";
import CalendarSettings from "./calendar/CalendarSettings";
import CalendarTile from "./calendar/CalendarTile";
import ClockPanel from "./clock/ClockPanel";
import ClockSettings from "./clock/ClockSettings";
import ClockTile from "./clock/ClockTile";
import type {
  WidgetPanelProps,
  WidgetSettingsProps,
  WidgetTileProps,
} from "./contract";
import NetworkPanel from "./network/NetworkPanel";
import NetworkSettings from "./network/NetworkSettings";
import NetworkTile from "./network/NetworkTile";
import NotePanel from "./notes/NotePanel";
import NoteSettings from "./notes/NoteSettings";
import NoteTile from "./notes/NoteTile";
import ProgressPanel from "./time-progress/ProgressPanel";
import ProgressSettings from "./time-progress/ProgressSettings";
import ProgressTile from "./time-progress/ProgressTile";

/* Each kind the dock can hold, with its presentation (spec 0003 §2.6). The
 * lanes fill the components (W1 clock, W2 time progress, W3 note); the host
 * (W5) reads only this file. Shortcut and Files have no entry until phase
 * 3: `toggle_panel` refuses them, and the list never holds them. */

/** The static glaze class of each kind's tile (tokens.css, The Own Glaze
 * Rule). */
export type GlazeClass =
  | "t-clock"
  | "t-progress"
  | "t-notes"
  | "t-activity"
  | "t-network"
  | "t-calendar";

export interface WidgetDefinition<K extends CreatableKind> {
  kind: K;
  /** Key of the kind's name in `t.widgets.kinds` (and of its short name in
   * `t.widgets.addNames`). Show an instance's name with `widgetName`. */
  nameKey: K;
  /** The tile's glaze, a class on the tile next to `dock-tile`. */
  glaze: GlazeClass;
  /** The live face inside the glaze. */
  Tile: ComponentType<WidgetTileProps<K>>;
  /** The main view of the panel. */
  Panel: ComponentType<WidgetPanelProps<K>>;
  /** The "Ajustes" view of the panel. */
  Settings: ComponentType<WidgetSettingsProps<K>>;
}

export const WIDGET_REGISTRY: {
  readonly [K in CreatableKind]: WidgetDefinition<K>;
} = {
  clock: {
    kind: "clock",
    nameKey: "clock",
    glaze: "t-clock",
    Tile: ClockTile,
    Panel: ClockPanel,
    Settings: ClockSettings,
  },
  "time-progress": {
    kind: "time-progress",
    nameKey: "time-progress",
    glaze: "t-progress",
    Tile: ProgressTile,
    Panel: ProgressPanel,
    Settings: ProgressSettings,
  },
  note: {
    kind: "note",
    nameKey: "note",
    glaze: "t-notes",
    Tile: NoteTile,
    Panel: NotePanel,
    Settings: NoteSettings,
  },
  activity: {
    kind: "activity",
    nameKey: "activity",
    glaze: "t-activity",
    Tile: ActivityTile,
    Panel: ActivityPanel,
    Settings: ActivitySettings,
  },
  network: {
    kind: "network",
    nameKey: "network",
    glaze: "t-network",
    Tile: NetworkTile,
    Panel: NetworkPanel,
    Settings: NetworkSettings,
  },
  calendar: {
    kind: "calendar",
    nameKey: "calendar",
    glaze: "t-calendar",
    Tile: CalendarTile,
    Panel: CalendarPanel,
    Settings: CalendarSettings,
  },
};

const CREATABLE: ReadonlySet<string> = new Set(CREATABLE_KINDS);

/** Whether the dock can show `instance` in this phase (always true for what
 * `list_widgets` returns: Rust hides the other kinds). */
export function isCreatableInstance(
  instance: WidgetInstance,
): instance is WidgetInstance<CreatableKind> {
  return CREATABLE.has(instance.kind);
}

/** The definition of `instance`'s kind, typed for it: with an instance of
 * any creatable kind, its components take that same instance. */
export function definitionOf<K extends CreatableKind>(
  instance: WidgetInstance<K>,
): WidgetDefinition<K> {
  // Sound: the entry of a kind holds that kind's components.
  return WIDGET_REGISTRY[instance.kind] as unknown as WidgetDefinition<K>;
}
