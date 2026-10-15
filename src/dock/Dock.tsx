import {
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
import { DEFAULT_DOCK_LOOK, dockSize, useAppearance } from "../core/appearance";
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
  type PanelChanged,
  type PanelId,
} from "../core/panels";
import { useT } from "../i18n";
import Glyph from "./Glyph";
import {
  dockLayout,
  dockWindowSize,
  placeDock,
  tipAnchor,
  type DockShape,
} from "./layout";
import { magnificationOf, magnifyRole } from "./magnification";
import "./Dock.css";

/** The dock's items, by group; a separator stands between groups. Fase 1
 * has only the two system items, one group, so no separator: DESIGN.md
 * keeps the separator for setting Configurações and Sobre apart from the
 * items of the next phases. */
const GROUPS: readonly (readonly PanelId[])[] = [["settings", "about"]];
const ITEMS: readonly PanelId[] = GROUPS.flat();
const GROUP_SIZES = GROUPS.map((group) => group.length);

/** Static glaze class of each item's tile (tokens.css, The Own Glaze
 * Rule). */
const GLAZE: Record<PanelId, string> = {
  settings: "t-settings",
  about: "t-about",
};

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
  const button = target instanceof Element ? target.closest("button") : null;
  if (!button) return null;
  const index = Array.from(bar.querySelectorAll("button")).indexOf(button);
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

function openPanel(panelId: PanelId, anchor: LogicalRect) {
  invoke("toggle_panel", { panelId, anchor }).catch((error: unknown) => {
    log.error(`dock: toggle panel failed: ${errorKind(error)}`);
  });
}

function Dock() {
  const t = useT();
  const settings = useAppearance("dock");
  const motion = !useReducedMotion();
  const viewport = useViewport();

  const [hovered, setHovered] = useState<number | null>(null);
  const [focused, setFocused] = useState<number | null>(null);
  const [current, setCurrent] = useState(0);
  const [open, setOpen] = useState<PanelId | null>(null);
  const [labelWidths, setLabelWidths] = useState<number[]>([]);

  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const tips = useRef<(HTMLSpanElement | null)[]>([]);
  const lastSent = useRef<string | null>(null);
  const lastPlace = useRef<string | null>(null);

  const { size, edge } = settings
    ? { size: dockSize(settings.dock.iconSize), edge: settings.dock.edge }
    : DEFAULT_DOCK_LOOK;
  const shape: DockShape = { size, groups: GROUP_SIZES };
  // Edge and icon size as set, not as drawn: several sizes draw the same
  // preset.
  const placeKey = settings
    ? `${settings.dock.edge} ${settings.dock.iconSize}`
    : null;

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
  const openRef = useRef(open);
  useLayoutEffect(() => {
    restItemsRef.current = restItems;
    openRef.current = open;
  });

  const tipIndex = active !== null && ITEMS[active] !== open ? active : null;
  const tipAt =
    tipIndex === null
      ? null
      : tipAnchor(placement.tiles[tipIndex] as LogicalRect, edge);

  // Label pills are laid out (hidden) all the time, so their widths are
  // known before any shows; the window must fit the widest.
  useLayoutEffect(() => {
    const widths = tips.current.map(
      (tip) => tip?.getBoundingClientRect().width ?? 0,
    );
    setLabelWidths((previous) =>
      previous.length === widths.length &&
      previous.every((width, index) => width === widths[index])
        ? previous
        : widths,
    );
  }, []);

  // The window size depends on the labels: nothing is sent before they are
  // measured.
  const measured = labelWidths.length === ITEMS.length;

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
        const index = ITEMS.indexOf(panelId);
        const anchor = restItemsRef.current[index];
        if (!anchor || openRef.current === panelId) return;
        openPanel(panelId, anchor);
      }),
    ];
    return () => {
      for (const stop of stops) void stop.then((unlisten) => unlisten());
    };
  }, []);

  const toggle = (index: number) => {
    const panelId = ITEMS[index];
    const anchor = restItems[index];
    if (panelId && anchor) openPanel(panelId, anchor);
  };

  const onPointer = (event: PointerEvent<HTMLElement>) => {
    // The magnification is frozen while a panel is open.
    if (open !== null) return;
    setHovered(itemIndex(event.currentTarget, event.target));
  };

  const moveFocus = (index: number) => {
    setCurrent(index);
    buttons.current[index]?.focus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    const index = itemIndex(event.currentTarget, event.target);
    if (index === null) return;
    const last = ITEMS.length - 1;
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
    setCurrent(index);
    setFocused(focusVisible(event.currentTarget) ? index : null);
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
        {GROUPS.map((group, groupIndex) => [
          groupIndex > 0 ? (
            <span
              key={`separator-${groupIndex}`}
              className="dock-separator"
              aria-hidden="true"
            />
          ) : null,
          ...group.map((panelId) => {
            const index = ITEMS.indexOf(panelId);
            const { scale, lift } = magnificationOf(index, magnified);
            const expanded = open === panelId;
            return (
              <button
                key={panelId}
                ref={(element) => {
                  buttons.current[index] = element;
                }}
                type="button"
                className="dock-item"
                aria-label={t.dock.items[panelId]}
                // No aria-controls: the panel is in another window, which
                // an IDREF can't reach.
                aria-expanded={expanded}
                tabIndex={index === current ? 0 : -1}
                data-magnify={magnifyRole(index, magnified) ?? undefined}
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
                <span className={`dock-tile ${GLAZE[panelId]}`}>
                  <Glyph id={panelId} />
                </span>
                <span className="dock-dot" aria-hidden="true" />
              </button>
            );
          }),
        ])}
      </nav>
      {ITEMS.map((panelId, index) => {
        const at = index === tipIndex ? tipAt : null;
        return (
          <span
            key={panelId}
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
            {t.dock.items[panelId]}
          </span>
        );
      })}
    </div>
  );
}

export default Dock;
