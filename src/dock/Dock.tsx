import {
  Fragment,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type FocusEvent,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  launchApp,
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
import { errorKind, log } from "../core/log";
import {
  PANEL_CHANGED_EVENT,
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
  /** The panel it opens; null for a pinned program, which opens itself. */
  panelId: PanelId | null;
  /** `panelKey` (`app:<id>` for a program): React key and identity across
   * changes of the list. */
  key: string;
  /** The item's name: its `aria-label` (plus "(com aviso)" while badged)
   * and its label pill. Never the time (spec 0003 §2.3 item 4). */
  name: string;
  /** The widget it shows, for its live face; null for a system item. */
  instance: WidgetInstance<CreatableKind> | null;
  /** The program it opens; null for a widget or a system item. */
  app: PinnedApp | null;
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
): DockEntry[] {
  const widgets =
    snapshot === null || snapshot.storage.state === "unavailable"
      ? []
      : snapshot.widgets.filter(isCreatableInstance);
  return [
    ...apps.map((app) => ({
      panelId: null,
      key: `app:${app.id}`,
      name: app.name,
      instance: null,
      app,
    })),
    ...widgets.map((instance) => {
      const panelId = widgetPanel(instance.id);
      return {
        panelId,
        key: panelKey(panelId),
        name: widgetName(t, instance),
        instance,
        app: null,
      };
    }),
    ...systems.map((panelId) => ({
      panelId,
      key: panelId,
      name: t.dock.items[panelId],
      instance: null,
      app: null,
    })),
  ];
}

/** The size of each group of items: a separator stands between the groups
 * that have items (the programs, the widgets, the system items). */
function groupSizes(
  entries: readonly DockEntry[],
  apps: number,
  systems: number,
): number[] {
  const widgets = entries.length - apps - systems;
  return [apps, widgets, systems].filter((size) => size > 0);
}

/** Whether the Configurações icon carries the attention badge: while the
 * database was recovered (until the notice is dismissed) or is unavailable
 * (spec 0003 §2.11). */
function needsAttention(storage: StorageStatus | undefined): boolean {
  return storage !== undefined && storage.state !== "ok";
}

/** A program's icon on its plate, or the first letter of its name while the
 * icon is not read yet and when Windows has none. */
function AppFace({ app }: { app: PinnedApp }) {
  const icon = useAppIcon(app.appId);
  return icon === null ? (
    <span className="dock-app-letter" aria-hidden="true">
      {Array.from(app.name.trim())[0]?.toUpperCase() ?? "?"}
    </span>
  ) : (
    <img className="dock-app-icon" src={icon} alt="" draggable={false} />
  );
}

/** The face inside an item's glaze: a program's icon, a widget's live face,
 * or a system item's glyph. */
function TileFace({ entry, now }: { entry: DockEntry; now: Date }) {
  if (entry.app !== null) return <AppFace app={entry.app} />;
  if (entry.instance === null) {
    return <Glyph id={entry.panelId as SystemPanelId} />;
  }
  const { Tile } = definitionOf(entry.instance);
  return <Tile instance={entry.instance} now={now} />;
}

function glazeOf(entry: DockEntry): string {
  if (entry.app !== null) return "t-app";
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

function openApp(app: PinnedApp) {
  launchApp(app.id).catch((error: unknown) => {
    // `launchApp` logged anything unexpected; a code from Rust is the kind.
    log.warn(`dock: opening a program failed: ${errorKind(error)}`);
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
  const entries = dockEntries(t, snapshot, apps, systems);
  const groups = groupSizes(entries, apps.length, systems.length);
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
  const magnified = motion && open === null ? active : null;
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

  // A program opens no panel, so its label shows whatever panel is open.
  const activePanel = active === null ? null : entries[active]?.panelId;
  const tipIndex =
    active !== null &&
    !(
      activePanel !== null &&
      activePanel !== undefined &&
      samePanel(activePanel, open)
    )
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
      listen(DOCK_RELEASED_EVENT, () => setHovered(null)),
      listen<PanelChanged>(PANEL_CHANGED_EVENT, ({ payload }) => {
        openRef.current = payload.panelId;
        setOpen(payload.panelId);
        if (payload.panelId !== null) setHovered(null);
      }),
      listen<DockRequestPanel>(DOCK_REQUEST_PANEL_EVENT, ({ payload }) => {
        const { panelId } = payload;
        const index = entriesRef.current.findIndex(
          (entry) =>
            entry.panelId !== null && samePanel(entry.panelId, panelId),
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

  const toggle = (index: number) => {
    const entry = entries[index];
    const anchor = restItems[index];
    if (entry?.app) {
      openApp(entry.app);
    } else if (entry?.panelId && anchor) {
      openPanel(entry.panelId, anchor);
    }
  };

  const onPointer = (event: PointerEvent<HTMLElement>) => {
    // The magnification is frozen while a panel is open.
    if (open !== null) return;
    setHovered(itemIndex(event.currentTarget, event.target));
  };

  const moveFocus = (index: number) => {
    setCurrentKey(entries[index]?.key ?? null);
    buttons.current[index]?.focus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
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
        aria-expanded={
          entry.panelId === null ? undefined : samePanel(open, entry.panelId)
        }
        tabIndex={index === currentIndex ? 0 : -1}
        data-magnify={magnifyRole(index, magnified, hover.effect) ?? undefined}
        style={
          {
            "--s": String(scale),
            "--lift": `${lift}px`,
          } as CSSProperties
        }
        onClick={() => toggle(index)}
        onFocus={(event) => onFocus(index, event)}
        onBlur={() => setFocused(null)}
      >
        <span className={`dock-tile ${glazeOf(entry)}`}>
          <TileFace entry={entry} now={now} />
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
