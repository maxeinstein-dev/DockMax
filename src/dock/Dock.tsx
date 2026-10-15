import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent,
  type PointerEvent,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { DOCK_RELEASED_EVENT, type DockHitRegion } from "../core/dock";
import { errorKind, log } from "../core/log";
import type { LogicalRect, PanelId } from "../core/panels";
import { useT } from "../i18n";
import {
  MAGNIFICATION_VARIABLES,
  holdRects,
  magnifyRole,
} from "./magnification";
import "./Dock.css";

const ITEMS: PanelId[] = ["settings", "about"];

function clientRect(element: Element): LogicalRect {
  const { x, y, width, height } = element.getBoundingClientRect();
  return { x, y, width, height };
}

function togglePanel(panelId: PanelId, event: MouseEvent<HTMLButtonElement>) {
  const anchor = clientRect(event.currentTarget);
  invoke("toggle_panel", { panelId, anchor }).catch((error: unknown) => {
    log.error(`dock: toggle panel failed: ${errorKind(error)}`);
  });
}

/** Dock.css turns magnification off for reduced motion; so does the hold
 * region. */
function reducedMotion(): boolean {
  return (
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

/** Where the dock paints. The items are measured by their buttons, which
 * are never transformed (the magnification scales the face inside), so the
 * rects are the layout ones even mid-transition. */
function measureRegion(
  bar: HTMLElement,
  hovered: number | null,
): DockHitRegion {
  const items = Array.from(bar.querySelectorAll("button"), clientRect);
  return {
    bar: clientRect(bar),
    hold: reducedMotion() ? [] : holdRects(items, hovered),
  };
}

/** Index of the item under the pointer, or null between items. */
function itemIndex(bar: HTMLElement, target: EventTarget): number | null {
  const button = target instanceof Element ? target.closest("button") : null;
  if (!button) return null;
  const index = Array.from(bar.querySelectorAll("button")).indexOf(button);
  return index < 0 ? null : index;
}

function Dock() {
  const t = useT();
  const barRef = useRef<HTMLElement>(null);
  const [hovered, setHovered] = useState<number | null>(null);
  const hoveredRef = useRef<number | null>(null);
  const lastSent = useRef<string | null>(null);

  /** Tells Rust where the dock paints: the mouse is captured only there.
   * Sends only when the region changed. */
  const report = useCallback(() => {
    const bar = barRef.current;
    if (!bar) return;
    const region = measureRegion(bar, hoveredRef.current);
    const key = JSON.stringify(region);
    if (key === lastSent.current) return;
    lastSent.current = key;
    invoke("set_dock_hit_region", { region }).catch((error: unknown) => {
      log.error(`dock: hit region update failed: ${errorKind(error)}`);
      // Lets the next report retry the same region.
      if (lastSent.current === key) lastSent.current = null;
    });
  }, []);

  useEffect(() => {
    hoveredRef.current = hovered;
    report();
  }, [hovered, report]);

  useEffect(() => {
    const bar = barRef.current;
    if (!bar || typeof ResizeObserver === "undefined") return;
    // The bar moves when the window resizes and resizes with its items.
    const observer = new ResizeObserver(report);
    observer.observe(bar);
    observer.observe(document.documentElement);
    return () => observer.disconnect();
  }, [report]);

  useEffect(() => {
    // Once released, the page may get no pointer events at all, so a
    // magnified item would stay stuck.
    const unlisten = listen(DOCK_RELEASED_EVENT, () => setHovered(null));
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  const onPointer = (event: PointerEvent<HTMLElement>) => {
    setHovered(itemIndex(event.currentTarget, event.target));
  };

  return (
    <div className="dock">
      <nav
        ref={barRef}
        className="dock-bar"
        aria-label={t.dock.label}
        style={MAGNIFICATION_VARIABLES as CSSProperties}
        onPointerOver={onPointer}
        onPointerMove={onPointer}
        onPointerLeave={() => setHovered(null)}
      >
        {ITEMS.map((panelId, index) => (
          <button
            key={panelId}
            type="button"
            data-magnify={magnifyRole(index, hovered) ?? undefined}
            onClick={(e) => togglePanel(panelId, e)}
          >
            <span className="dock-item">{t.dock.items[panelId]}</span>
          </button>
        ))}
      </nav>
    </div>
  );
}

export default Dock;
