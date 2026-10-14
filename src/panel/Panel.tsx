import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  PANEL_SHOW_EVENT,
  type PanelId,
  type PanelShowPayload,
} from "../core/panels";
import "./Panel.css";

const TITLES: Record<PanelId, string> = {
  settings: "Configurações",
  about: "Sobre",
};

function hidePanel() {
  invoke("hide_panel").catch((error: unknown) => {
    console.error("hide_panel failed", error);
  });
}

function Panel() {
  const [panelId, setPanelId] = useState<PanelId | null>(null);

  useEffect(() => {
    const unlisten = listen<PanelShowPayload>(PANEL_SHOW_EVENT, (event) => {
      setPanelId(event.payload.panelId);
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") hidePanel();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  if (panelId === null) return null;

  return (
    <section className="panel" aria-labelledby="panel-title">
      <h1 id="panel-title">{TITLES[panelId]}</h1>
      <p>Em construção.</p>
    </section>
  );
}

export default Panel;
