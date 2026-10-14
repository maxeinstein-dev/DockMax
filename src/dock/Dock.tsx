import { useEffect, useRef, type MouseEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { LogicalRect, PanelId } from "../core/panels";
import "./Dock.css";

function clientRect(element: Element): LogicalRect {
  const { x, y, width, height } = element.getBoundingClientRect();
  return { x, y, width, height };
}

function togglePanel(panelId: PanelId, event: MouseEvent<HTMLButtonElement>) {
  const anchor = clientRect(event.currentTarget);
  invoke("toggle_panel", { panelId, anchor }).catch((error: unknown) => {
    console.error("toggle_panel failed", error);
  });
}

/** Tells Rust where the visible bar is: only it captures the mouse. */
function reportBar(bar: Element) {
  invoke("set_dock_hit_rect", { rect: clientRect(bar) }).catch(
    (error: unknown) => {
      console.error("set_dock_hit_rect failed", error);
    },
  );
}

function Dock() {
  const barRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const bar = barRef.current;
    if (!bar) return;
    reportBar(bar);
    if (typeof ResizeObserver === "undefined") return;
    // The bar moves when the window resizes and resizes with its items.
    const observer = new ResizeObserver(() => reportBar(bar));
    observer.observe(bar);
    observer.observe(document.documentElement);
    return () => observer.disconnect();
  }, []);

  return (
    <div className="dock">
      <nav ref={barRef} className="dock-bar" aria-label="Dock">
        <button type="button" onClick={(e) => togglePanel("settings", e)}>
          Configurações
        </button>
        <button type="button" onClick={(e) => togglePanel("about", e)}>
          Sobre
        </button>
      </nav>
    </div>
  );
}

export default Dock;
