import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  PANEL_SHOW_EVENT,
  type PanelId,
  type PanelShowPayload,
} from "../core/panels";
import PanelBoundary from "../components/PanelBoundary";
import { errorKind, log } from "../core/log";
import { useT } from "../i18n";
import PanelContent from "./PanelContent";
import "./Panel.css";

function hidePanel() {
  invoke("hide_panel").catch((error: unknown) => {
    console.error("hide_panel failed", error);
  });
}

/** Records a panel that failed to render: the kind of error only, never its
 * message (it may carry user data). */
function logPanelFailure(error: unknown) {
  log.error(`panel: render failed: ${errorKind(error)}`);
}

function Panel() {
  const t = useT();
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
      <h1 id="panel-title">{t.panel.titles[panelId]}</h1>
      <PanelBoundary
        resetKey={panelId}
        fallback={t.panel.failed}
        onError={logPanelFailure}
      >
        <PanelContent panelId={panelId} />
      </PanelBoundary>
    </section>
  );
}

export default Panel;
