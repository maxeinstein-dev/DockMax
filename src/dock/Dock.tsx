import {
  Fragment,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type FocusEvent,
  type KeyboardEvent,
  type MouseEvent,
  type PointerEvent,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  ICON_SIZES,
  dropIcons,
  launchApp,
  placeApp,
  useAppIcon,
  useApps,
  type AppsSnapshot,
  type PinnedApp,
} from "../core/apps";
import { DEFAULT_DOCK_LOOK, dockSize, useAppearance } from "../core/appearance";
import { DEFAULT_LOOK } from "../core/settings";
import { OPEN_FLYOUT_COMMAND, type OpenFlyoutArgs } from "../core/system";
import {
  DOCK_RELEASED_EVENT,
  DOCK_REQUEST_PANEL_EVENT,
  SET_DOCK_LAYOUT_COMMAND,
  type DockRequestPanel,
  type SetDockLayoutArgs,
} from "../core/dock";
import { ICONS_CHANGED_EVENT } from "../core/iconPacks";
import {
  activateApp,
  activateOpen,
  launchOpen,
  useOpenApps,
  type OpenOther,
} from "../core/openApps";
import { errorKind, log } from "../core/log";
import {
  PANEL_CHANGED_EVENT,
  appPanel,
  otherPanel,
  type LogicalRect,
  panelKey,
  samePanel,
  widgetPanel,
  type PanelChanged,
  type PanelId,
  type SystemPanelId,
} from "../core/panels";
import {
  useWidgets,
  type CreatableKind,
  type StorageStatus,
  type WidgetInstance,
  type WidgetsSnapshot,
} from "../core/widgets";
import { format, useT, type Messages } from "../i18n";
import { widgetName } from "../widgets/names";
import { MINUTE_MS, useNow } from "../widgets/now";
import { definitionOf, isCreatableInstance } from "../widgets/registry";
import { DRAG_THRESHOLD, dragOffset, dropIndex, dropMark } from "./drag";
import Glyph from "./Glyph";
import { clockText } from "./status";
import {
  dockLayout,
  dockWindowSize,
  placeDock,
  tipAnchor,
  type DockShape,
} from "./layout";
import { hoverConfigOf, magnificationOf, magnifyRole } from "./magnification";
import "./Dock.css";

/** The system items, always last and never removed (spec 0003 §2.1 item
 * 2); with the Windows taskbar hidden altogether, the tray is one more
 * (ADR 0009). */
function systemItems(tray: boolean): readonly SystemPanelId[] {
  return tray ? ["settings", "about", "tray"] : ["settings", "about"];
}

/** Static glaze class of each system item's tile (tokens.css, The Own Glaze
 * Rule). */
const GLAZE: Record<SystemPanelId, string> = {
  settings: "t-settings",
  about: "t-about",
  tray: "t-tray",
};

/** One item of the dock. */
interface DockEntry {
  /** The panel it opens; for a pinned program, its menu (the program itself
   * opens on a click). */
  panelId: PanelId;
  /** `panelKey` (`app:<id>` for a program): React key and identity across
   * changes of the list. */
  key: string;
  /** The item's name: its `aria-label` (plus "(com aviso)" while badged)
   * and its label pill. Never the time (spec 0003 §2.3 item 4). */
  name: string;
  /** The widget it shows, for its live face; null for a system item. */
  instance: WidgetInstance<CreatableKind> | null;
  /** The program it opens; null for a widget, a system item or an open
   * program that is not pinned. */
  app: PinnedApp | null;
  /** The open program that is not pinned that it stands for (spec 0009 D2);
   * null for the others. */
  other: OpenOther | null;
}

/** The pinned programs the dock shows: none while they are loading, or with
 * the storage unavailable (spec 0006 §3.2). */
function pinnedApps(snapshot: AppsSnapshot | null): readonly PinnedApp[] {
  return snapshot === null || snapshot.storage.state === "unavailable"
    ? []
    : snapshot.apps;
}

/** The dock's items in order: the pinned programs, the widgets in the order
 * chosen, then Configurações and Sobre (spec 0003 §2.1 item 2, spec 0006
 * §3.4). Without a readable list, or with the storage unavailable, only the
 * system items. */
function dockEntries(
  t: Messages,
  snapshot: WidgetsSnapshot | null,
  apps: readonly PinnedApp[],
  systems: readonly SystemPanelId[],
  others: readonly OpenOther[],
): DockEntry[] {
  const widgets =
    snapshot === null || snapshot.storage.state === "unavailable"
      ? []
      : snapshot.widgets.filter(isCreatableInstance);
  return [
    ...apps.map((app) => ({
      panelId: appPanel(app.id),
      key: panelKey(appPanel(app.id)),
      name: app.name,
      instance: null,
      app,
      other: null,
    })),
    ...widgets.map((instance) => {
      const panelId = widgetPanel(instance.id);
      return {
        panelId,
        key: panelKey(panelId),
        name: widgetName(t, instance),
        instance,
        app: null,
        other: null,
      };
    }),
    ...others.map((other) => {
      const panelId = otherPanel(other.key);
      return {
        panelId,
        key: panelKey(panelId),
        name: other.name,
        instance: null,
        app: null,
        other,
      };
    }),
    ...systems.map((panelId) => ({
      panelId,
      key: panelId,
      name: t.dock.items[panelId],
      instance: null,
      app: null,
      other: null,
    })),
  ];
}

/** The size of each group of items: a separator stands between the groups
 * that have items (the programs, the widgets, the open programs that are not
 * pinned, the system items). */
function groupSizes(
  entries: readonly DockEntry[],
  apps: number,
  others: number,
  systems: number,
): number[] {
  const widgets = entries.length - apps - others - systems;
  return [apps, widgets, others, systems].filter((size) => size > 0);
}

/** Whether the Configurações icon carries the attention badge: while the
 * database was recovered (until the notice is dismissed) or is unavailable
 * (spec 0003 §2.11). */
function needsAttention(storage: StorageStatus | undefined): boolean {
  return storage !== undefined && storage.state !== "ok";
}

/** A program's icon on its plate, or the first letter of its name while the
 * icon is not read yet and when Windows has none. */
function AppFace({
  appId,
  name,
  variant,
}: {
  appId: string;
  name: string;
  variant: string;
}) {
  const icon = useAppIcon(appId, ICON_SIZES.dock, variant);
  return icon === null ? (
    <span className="dock-app-letter" aria-hidden="true">
      {Array.from(name.trim())[0]?.toUpperCase() ?? "?"}
    </span>
  ) : (
    <img className="dock-app-icon" src={icon} alt="" draggable={false} />
  );
}

/** The face inside an item's glaze: a program's icon, a widget's live face,
 * or a system item's glyph. */
function TileFace({
  entry,
  now,
  iconVariant,
}: {
  entry: DockEntry;
  now: Date;
  iconVariant: string;
}) {
  if (entry.app !== null) {
    return (
      <AppFace
        appId={entry.app.appId}
        name={entry.app.name}
        variant={iconVariant}
      />
    );
  }
  if (entry.other !== null) {
    return (
      <AppFace
        appId={entry.other.appId}
        name={entry.other.name}
        variant={iconVariant}
      />
    );
  }
  if (entry.instance === null) {
    return <Glyph id={entry.panelId as SystemPanelId} />;
  }
  const { Tile } = definitionOf(entry.instance);
  return <Tile instance={entry.instance} now={now} />;
}

function glazeOf(entry: DockEntry): string {
  if (entry.app !== null || entry.other !== null) return "t-app";
  return entry.instance === null
    ? GLAZE[entry.panelId as SystemPanelId]
    : definitionOf(entry.instance).glaze;
}

function reducedMotionQuery(): MediaQueryList | null {
  return typeof window.matchMedia === "function"
    ? window.matchMedia("(prefers-reduced-motion: reduce)")
    : null;
}

/** Whether Windows asks for reduced motion, following changes. */
function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(
    () => reducedMotionQuery()?.matches ?? false,
  );
  useEffect(() => {
    const media = reducedMotionQuery();
    if (!media || typeof media.addEventListener !== "function") return;
    const onChange = () => setReduced(media.matches);
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, []);
  return reduced;
}

function viewportSize() {
  return { width: window.innerWidth, height: window.innerHeight };
}

/** The window's client area, following resizes (Rust sizes the window from
 * the reported layout). */
function useViewport() {
  const [viewport, setViewport] = useState(viewportSize);
  useEffect(() => {
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => {
      setViewport((current) => {
        const next = viewportSize();
        return next.width === current.width && next.height === current.height
          ? current
          : next;
      });
    });
    observer.observe(document.documentElement);
    return () => observer.disconnect();
  }, []);
  return viewport;
}

/** Index of the item under the pointer, or null between items. */
function itemIndex(bar: HTMLElement, target: EventTarget): number | null {
  const button =
    target instanceof Element ? target.closest("button.dock-item") : null;
  if (!button) return null;
  const index = Array.from(bar.querySelectorAll("button.dock-item")).indexOf(
    button,
  );
  return index < 0 ? null : index;
}

/** Whether a focused item got there by keyboard (`:focus-visible`). */
function focusVisible(element: Element): boolean {
  try {
    return element.matches(":focus-visible");
  } catch {
    return true;
  }
}

function openCalendar() {
  const args: OpenFlyoutArgs = { flyout: "calendar" };
  invoke(OPEN_FLYOUT_COMMAND, { ...args }).catch((error: unknown) => {
    log.error(`dock: opening the calendar failed: ${errorKind(error)}`);
  });
}

/** How long a program's icon animates after a click: the animations of
 * Dock.css last 1.4 s (bounce and shake) and 1.5 s (pulse). */
const OPENING_MS = 1500;

/** Opens the program; resolves when it was asked to, rejects when it could
 * not be. */
/** A press on a pinned program that may become a drag. */
interface Press {
  key: string;
  /** The program's index among the pinned ones. */
  from: number;
  id: PinnedApp["id"];
  pointer: number;
  startX: number;
  /** The pointer went far enough: it is a drag, not a click. */
  dragging: boolean;
}

/** What the drag shows: the dragged item, how far it is from where it rests
 * and where it would land. */
interface DragView {
  key: string;
  offset: number;
  target: number;
  from: number;
}

function openApp(app: PinnedApp): Promise<void> {
  return launchApp(app.id).catch((error: unknown) => {
    // `launchApp` logged anything unexpected; a code from Rust is the kind.
    log.warn(`dock: opening a program failed: ${errorKind(error)}`);
    throw error;
  });
}

function openPanel(panelId: PanelId, anchor: LogicalRect) {
  invoke("toggle_panel", { panelId, anchor }).catch((error: unknown) => {
    log.error(`dock: toggle panel failed: ${errorKind(error)}`);
  });
}

/** Label widths measured for one set of names (`namesKey`). */
interface LabelWidths {
  key: string;
  widths: number[];
}

function Dock() {
  const t = useT();
  const settings = useAppearance("dock");
  const snapshot = useWidgets();
  const appsSnapshot = useApps();
  // Which pinned programs have windows open (spec 0009).
  const openNow = useOpenApps();
  const openApps = openNow.apps;
  // The icons depend on the pack in use and on what the pack folder holds
  // (spec 0007): either changing asks for them again.
  const [iconsEpoch, setIconsEpoch] = useState(0);
  const iconVariant = `${settings?.dock.iconPack ?? ""}#${iconsEpoch}`;
  const motion = !useReducedMotion();
  const viewport = useViewport();
  // The dock's single timer, for every live face.
  const now = useNow(MINUTE_MS);

  const clock = clockText(now);
  // The tray is the dock's only while the taskbar is hidden altogether; the
  // clock and the date, while it is hidden in either way.
  const taskbar = settings?.dock.taskbar ?? DEFAULT_LOOK.taskbar;
  const systems = systemItems(taskbar === "hidden");
  const apps = pinnedApps(appsSnapshot);
  const others = settings?.dock.showOpenApps === false ? [] : openNow.others;
  const entries = dockEntries(t, snapshot, apps, systems, others);
  const groups = groupSizes(
    entries,
    apps.length,
    others.length,
    systems.length,
  );
  const badge = needsAttention(snapshot?.storage);
  // Changes whenever an item or a name changes: the pills are measured
  // again then.
  const names = entries.map((entry) => entry.name);
  const namesKey = JSON.stringify(names);

  const [hoveredAt, setHovered] = useState<number | null>(null);
  const [focusedAt, setFocused] = useState<number | null>(null);
  const [currentKey, setCurrentKey] = useState<string | null>(null);
  const [open, setOpen] = useState<PanelId | null>(null);
  const [labels, setLabels] = useState<LabelWidths | null>(null);
  /** The programs whose icon is animating after a click, by item key. */
  const [opening, setOpening] = useState<ReadonlySet<string>>(new Set());
  const openingTimers = useRef(new Map<string, number>());
  const press = useRef<Press | null>(null);
  /** The click that ends a drag is not a click on the program. */
  const dragged = useRef(false);
  const [drag, setDrag] = useState<DragView | null>(null);

  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const tips = useRef<(HTMLSpanElement | null)[]>([]);
  const lastSent = useRef<string | null>(null);
  const lastPlace = useRef<string | null>(null);

  // An index left over from a longer list counts as none.
  const inRange = (index: number | null) =>
    index !== null && index < entries.length ? index : null;
  const hovered = inRange(hoveredAt);
  const focused = inRange(focusedAt);
  // The single tab stop: the item last focused, or the first one.
  const currentIndex = Math.max(
    entries.findIndex((entry) => entry.key === currentKey),
    0,
  );

  const { size, edge } = settings
    ? { size: dockSize(settings.dock.iconSize), edge: settings.dock.edge }
    : DEFAULT_DOCK_LOOK;
  const hover = hoverConfigOf(settings?.dock);
  // The clock and the date stand in for the taskbar's while it is hidden.
  const status = taskbar !== "visible";
  const shape: DockShape = { size, groups, hover, status };
  // Edge and icon size as set, not as drawn: several sizes draw the same
  // preset.
  const placeKey = settings
    ? `${settings.dock.edge} ${settings.dock.iconSize}`
    : null;

  // The window size depends on the labels: nothing is sent before the
  // current names are measured.
  const measured = labels !== null && labels.key === namesKey;
  const labelWidths = measured ? labels.widths : [];

  // Frozen while a panel is open, off with reduced motion.
  const active = hovered ?? focused;
  const magnified = motion && open === null && drag === null ? active : null;
  const placement = placeDock({ ...shape, edge, magnified, viewport });
  const windowSize = dockWindowSize({ ...shape, motion, labelWidths });
  const layoutKey = JSON.stringify(dockLayout(placement, windowSize));

  // The panel opens where the item rests: the dock stops magnifying then.
  const restItems = placeDock({
    ...shape,
    edge,
    magnified: null,
    viewport,
  }).items;
  const restItemsRef = useRef(restItems);
  const entriesRef = useRef(entries);
  const openRef = useRef(open);
  useLayoutEffect(() => {
    restItemsRef.current = restItems;
    entriesRef.current = entries;
    openRef.current = open;
  });

  const tipIndex =
    drag === null &&
    active !== null &&
    !samePanel(entries[active]?.panelId ?? null, open)
      ? active
      : null;
  const tipTile = tipIndex === null ? undefined : placement.tiles[tipIndex];
  const tipAt = tipTile ? tipAnchor(tipTile, edge) : null;

  // Label pills are laid out (hidden) all the time, so their widths are
  // known before any shows; the window must fit the widest. Measured again
  // whenever an item or a name changes.
  useLayoutEffect(() => {
    const count = (JSON.parse(namesKey) as string[]).length;
    const widths = tips.current
      .slice(0, count)
      .map((tip) => tip?.getBoundingClientRect().width ?? 0);
    setLabels((previous) =>
      previous !== null &&
      previous.key === namesKey &&
      previous.widths.length === widths.length &&
      previous.widths.every((width, index) => width === widths[index])
        ? previous
        : { key: namesKey, widths },
    );
  }, [namesKey]);

  /** Tells Rust where the dock paints (the mouse is captured only there)
   * and the window it needs. Sends when the layout changed, and after any
   * change of edge or icon size even when the layout did not: Rust closed
   * the open panel for the move and reopens it only once a layout comes. */
  useEffect(() => {
    if (!measured) return;
    const moved =
      lastPlace.current !== null &&
      placeKey !== null &&
      placeKey !== lastPlace.current;
    if (placeKey !== null) lastPlace.current = placeKey;
    if (!moved && layoutKey === lastSent.current) return;
    lastSent.current = layoutKey;
    const args: SetDockLayoutArgs = { layout: JSON.parse(layoutKey) };
    invoke(SET_DOCK_LAYOUT_COMMAND, { ...args }).catch((error: unknown) => {
      log.error(`dock: layout update failed: ${errorKind(error)}`);
      // A later change back to this layout sends it again.
      if (lastSent.current === layoutKey) lastSent.current = null;
    });
  }, [layoutKey, measured, placeKey]);

  useEffect(() => {
    const stops = [
      // Once released, the page may get no pointer events at all, so a
      // magnified item would stay stuck.
      listen(DOCK_RELEASED_EVENT, () => {
        setHovered(null);
        // The pointer left the dock for good: the drag is over.
        press.current = null;
        setDrag(null);
      }),
      listen(ICONS_CHANGED_EVENT, () => {
        dropIcons();
        setIconsEpoch((epoch) => epoch + 1);
      }),
      listen<PanelChanged>(PANEL_CHANGED_EVENT, ({ payload }) => {
        openRef.current = payload.panelId;
        setOpen(payload.panelId);
        if (payload.panelId !== null) setHovered(null);
      }),
      listen<DockRequestPanel>(DOCK_REQUEST_PANEL_EVENT, ({ payload }) => {
        const { panelId } = payload;
        const index = entriesRef.current.findIndex((entry) =>
          samePanel(entry.panelId, panelId),
        );
        const anchor = restItemsRef.current[index];
        if (!anchor || samePanel(openRef.current, panelId)) return;
        openPanel(panelId, anchor);
      }),
    ];
    return () => {
      for (const stop of stops) void stop.then((unlisten) => unlisten());
    };
  }, []);

  // No timer outlives the dock.
  useEffect(() => {
    const timers = openingTimers.current;
    return () => {
      for (const timer of timers.values()) window.clearTimeout(timer);
      timers.clear();
    };
  }, []);

  const setOpeningOf = (key: string, on: boolean) =>
    setOpening((previous) => {
      if (previous.has(key) === on) return previous;
      const next = new Set(previous);
      if (on) next.add(key);
      else next.delete(key);
      return next;
    });

  /** Opens the program and animates its icon for a moment; a program that
   * could not be opened stops at once. */
  const launch = (key: string, run: () => Promise<void>) => {
    const timers = openingTimers.current;
    window.clearTimeout(timers.get(key));
    setOpeningOf(key, true);
    timers.set(
      key,
      window.setTimeout(() => {
        timers.delete(key);
        setOpeningOf(key, false);
      }, OPENING_MS),
    );
    run().catch(() => {
      window.clearTimeout(timers.get(key));
      timers.delete(key);
      setOpeningOf(key, false);
    });
  };

  const toggle = (index: number) => {
    const entry = entries[index];
    const anchor = restItems[index];
    if (entry?.app) {
      const app = entry.app;
      if (openApps.has(app.id)) {
        // Open already: to the front (or away, or to its next window); a
        // program whose windows closed meanwhile is opened.
        void activateApp(app.id).then((done) => {
          if (!done) launch(entry.key, () => openApp(app));
        });
      } else {
        launch(entry.key, () => openApp(app));
      }
    } else if (entry?.other) {
      // An open program that is not pinned: to the front, away, or to its
      // next window. It has no menu until the D3 recorte.
      void activateOpen(entry.other.key);
    } else if (entry && anchor) {
      openPanel(entry.panelId, anchor);
    }
  };

  /** The middle button on a program: what the setting says, a new instance
   * unless the user turned it off. */
  const middleClick = (index: number) => {
    if ((settings?.dock.middleClick ?? "newInstance") !== "newInstance") return;
    const entry = entries[index];
    if (entry?.app) {
      const app = entry.app;
      launch(entry.key, () => openApp(app));
    } else if (entry?.other) {
      const key = entry.other.key;
      launch(entry.key, () => launchOpen(key));
    }
  };

  /** The menu of a pinned program, on a right click or the menu key. */
  const openMenu = (index: number, event: MouseEvent<HTMLElement>) => {
    const entry = entries[index];
    const anchor = restItems[index];
    if (!entry?.app || !anchor) return;
    event.preventDefault();
    openPanel(entry.panelId, anchor);
  };

  const onPointer = (event: PointerEvent<HTMLElement>) => {
    if (press.current !== null) {
      dragTo(event);
      return;
    }
    // The magnification is frozen while a panel is open.
    if (open !== null) return;
    setHovered(itemIndex(event.currentTarget, event.target));
  };

  /** The programs at rest: their centres along the bar, in order. */
  const appCenters = () =>
    restItemsRef.current
      .slice(0, apps.length)
      .map((rect) => rect.x + rect.width / 2);

  /** A press on a pinned program: a drag if the pointer goes far enough. */
  const onPress = (index: number, event: PointerEvent<HTMLButtonElement>) => {
    const entry = entries[index];
    if (!entry?.app || event.button !== 0 || open !== null) return;
    press.current = {
      key: entry.key,
      from: index,
      id: entry.app.id,
      pointer: event.pointerId,
      startX: event.clientX,
      dragging: false,
    };
    dragged.current = false;
    // The moves and the release come here even outside the item.
    event.currentTarget.setPointerCapture?.(event.pointerId);
  };

  const dragTo = (event: PointerEvent<HTMLElement>) => {
    const current = press.current;
    if (current === null || event.pointerId !== current.pointer) return;
    if (
      !current.dragging &&
      Math.abs(event.clientX - current.startX) < DRAG_THRESHOLD
    ) {
      return;
    }
    current.dragging = true;
    const rest = restItemsRef.current[current.from];
    const bar = restItemsRef.current.slice(0, apps.length);
    if (!rest || bar.length === 0) return;
    const first = bar[0];
    const last = bar[bar.length - 1];
    if (!first || !last) return;
    setHovered(null);
    setDrag({
      key: current.key,
      from: current.from,
      offset: dragOffset(
        event.clientX,
        current.startX,
        rest.x,
        rest.width,
        first.x,
        last.x + last.width,
      ),
      target: dropIndex(appCenters(), current.from, event.clientX),
    });
  };

  /** Lets go: a drag puts the program where it was let go. */
  const onRelease = (event: PointerEvent<HTMLElement>) => {
    const current = press.current;
    if (current === null || event.pointerId !== current.pointer) return;
    press.current = null;
    setDrag(null);
    if (!current.dragging) return;
    dragged.current = true;
    // The click that follows is the drag's, not the program's.
    window.setTimeout(() => {
      dragged.current = false;
    }, 100);
    const target = dropIndex(appCenters(), current.from, event.clientX);
    if (target !== current.from) {
      placeApp(current.id, target).catch((error: unknown) => {
        log.warn(`dock: placing a program failed: ${errorKind(error)}`);
      });
    }
  };

  const cancelDrag = () => {
    press.current = null;
    setDrag(null);
  };

  const moveFocus = (index: number) => {
    setCurrentKey(entries[index]?.key ?? null);
    buttons.current[index]?.focus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === "Escape" && press.current !== null) {
      cancelDrag();
      return;
    }
    const index = itemIndex(event.currentTarget, event.target);
    if (index === null) return;
    const last = entries.length - 1;
    const next = {
      ArrowLeft: Math.max(index - 1, 0),
      ArrowRight: Math.min(index + 1, last),
      Home: 0,
      End: last,
    }[event.key];
    if (next !== undefined) {
      event.preventDefault();
      moveFocus(next);
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      // Handled here, not left to the button, so a held key toggles once.
      event.preventDefault();
      if (!event.repeat) toggle(index);
    }
  };

  const onFocus = (index: number, event: FocusEvent<HTMLButtonElement>) => {
    setCurrentKey(entries[index]?.key ?? null);
    setFocused(focusVisible(event.currentTarget) ? index : null);
  };

  /** Which side of the item the mark of the drop stands on, if it does. */
  const dropSide = (index: number): string | undefined => {
    if (drag === null) return undefined;
    const mark = dropMark(apps.length, drag.from, drag.target);
    return mark?.index === index ? mark.side : undefined;
  };

  const item = (entry: DockEntry, index: number) => {
    const { scale, lift } = magnificationOf(index, magnified, hover);
    // The badge is aria-hidden: the name says it instead.
    const badged = badge && entry.panelId === "settings";
    return (
      <button
        key={entry.key}
        ref={(element) => {
          buttons.current[index] = element;
        }}
        type="button"
        className="dock-item"
        aria-label={
          badged
            ? format(t.widgets.withNotice, { name: entry.name })
            : entry.name
        }
        // No aria-controls: the panel is in another window, which an IDREF
        // can't reach.
        aria-expanded={samePanel(open, entry.panelId)}
        aria-haspopup={entry.app ? "menu" : undefined}
        tabIndex={index === currentIndex ? 0 : -1}
        data-magnify={magnifyRole(index, magnified, hover.effect) ?? undefined}
        data-opening={opening.has(entry.key) ? "" : undefined}
        data-running={
          entry.other || (entry.app && openApps.has(entry.app.id))
            ? ""
            : undefined
        }
        data-active={
          (entry.other ?? (entry.app && openApps.get(entry.app.id)))?.active
            ? ""
            : undefined
        }
        data-dragging={drag?.key === entry.key ? "" : undefined}
        data-drop={dropSide(index)}
        style={
          {
            "--s": String(scale),
            "--lift": `${lift}px`,
            "--drag-x": drag?.key === entry.key ? `${drag.offset}px` : "0px",
          } as CSSProperties
        }
        onPointerDown={(event) => onPress(index, event)}
        onClick={() => {
          if (dragged.current) {
            dragged.current = false;
            return;
          }
          toggle(index);
        }}
        // The middle button: without this the page starts its scroll mode.
        onMouseDown={(event) => {
          if (event.button === 1) event.preventDefault();
        }}
        onAuxClick={(event) => {
          if (event.button !== 1) return;
          event.preventDefault();
          middleClick(index);
        }}
        onContextMenu={(event) => openMenu(index, event)}
        onFocus={(event) => onFocus(index, event)}
        onBlur={() => setFocused(null)}
      >
        <span className={`dock-tile ${glazeOf(entry)}`}>
          <TileFace entry={entry} now={now} iconVariant={iconVariant} />
          {badged ? <span className="dock-badge" aria-hidden="true" /> : null}
        </span>
        <span className="dock-dot" aria-hidden="true" />
      </button>
    );
  };

  return (
    <div className="dock">
      <nav
        className="dock-bar"
        aria-label={t.dock.label}
        onPointerOver={onPointer}
        onPointerMove={onPointer}
        onPointerLeave={() => setHovered(null)}
        onPointerUp={onRelease}
        onPointerCancel={cancelDrag}
        onKeyDown={onKeyDown}
      >
        {groups.map((size, group) => {
          const start = groups
            .slice(0, group)
            .reduce((sum, before) => sum + before, 0);
          return (
            <Fragment key={start}>
              {group > 0 ? (
                <span className="dock-separator" aria-hidden="true" />
              ) : null}
              {entries
                .slice(start, start + size)
                .map((entry, offset) => item(entry, start + offset))}
            </Fragment>
          );
        })}
        {status ? (
          <>
            <span className="dock-separator" aria-hidden="true" />
            <button
              type="button"
              className="dock-status"
              aria-label={t.dock.status.calendar}
              onClick={openCalendar}
            >
              <span className="dock-status-time">{clock.time}</span>
              <span className="dock-status-date">{clock.date}</span>
            </button>
          </>
        ) : null}
      </nav>
      {entries.map((entry, index) => {
        const at = index === tipIndex ? tipAt : null;
        return (
          <span
            key={entry.key}
            ref={(element) => {
              tips.current[index] = element;
            }}
            className="dock-tip"
            aria-hidden="true"
            data-visible={at ? "" : undefined}
            style={
              at
                ? ({
                    "--tip-x": `${at.x}px`,
                    "--tip-y": `${at.y}px`,
                  } as CSSProperties)
                : undefined
            }
          >
            {entry.name}
          </span>
        );
      })}
    </div>
  );
}

export default Dock;
