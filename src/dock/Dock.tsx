import type { MouseEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { LogicalRect, PanelId } from "../core/panels";
import "./Dock.css";

function togglePanel(panelId: PanelId, event: MouseEvent<HTMLButtonElement>) {
  const { x, y, width, height } = event.currentTarget.getBoundingClientRect();
  const anchor: LogicalRect = { x, y, width, height };
  invoke("toggle_panel", { panelId, anchor }).catch((error: unknown) => {
    console.error("toggle_panel failed", error);
  });
}

function Dock() {
  return (
    <div className="dock">
      <nav className="dock-bar" aria-label="Dock">
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
