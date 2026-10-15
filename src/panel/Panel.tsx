import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  PANEL_READY_COMMAND,
  PANEL_SHOW_EVENT,
  type PanelId,
  type PanelReadyArgs,
  type PanelShowPayload,
} from "../core/panels";
import { useSettings } from "../core/settings";
import PanelBoundary from "../components/PanelBoundary";
import { errorKind, log } from "../core/log";
import { useT } from "../i18n";
import PanelContent from "./PanelContent";
import { PANEL_INSET, measureFrame } from "./geometry";
import "./Panel.css";

function hidePanel() {
  invoke("hide_panel").catch((error: unknown) => {
    log.error(`panel: hide failed: ${errorKind(error)}`);
  });
}

function panelReady(args: PanelReadyArgs) {
  invoke(PANEL_READY_COMMAND, { ...args }).catch((error: unknown) => {
    log.error(`panel: ready failed: ${errorKind(error)}`);
  });
}

/** Records a panel that failed to render: the kind of error only, never its
 * message (it may carry user data). */
function logPanelFailure(error: unknown) {
  log.error(`panel: render failed: ${errorKind(error)}`);
}

/** One `panel-show`: showing the same panel again is a new request. */
interface ShowRequest {
  panelId: PanelId;
  seq: number;
}

function Panel() {
  const t = useT();
  const [request, setRequest] = useState<ShowRequest | null>(null);
  const frameRef = useRef<HTMLDivElement>(null);
  // Read here, not in the Settings panel, so they are loaded long before
  // the panel first shows.
  const settings = useSettings();

  useEffect(() => {
    const unlisten = listen<PanelShowPayload>(PANEL_SHOW_EVENT, (event) => {
      const { panelId } = event.payload;
      setRequest((previous) => ({ panelId, seq: (previous?.seq ?? 0) + 1 }));
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

  // Runs after the content of this request is in the DOM and laid out, and
  // before the browser paints: the size sent is always the requested
  // panel's, never the one shown before it. Rust shows the window only then.
  useLayoutEffect(() => {
    const frame = frameRef.current;
    if (request === null || frame === null) return;
    panelReady({
      panelId: request.panelId,
      size: measureFrame(frame),
      inset: PANEL_INSET,
    });
  }, [request]);

  if (request === null) return null;
  const { panelId, seq } = request;

  return (
    <div
      ref={frameRef}
      className="panel-frame"
      data-edge={settings?.dock.edge ?? "bottom"}
    >
      {/* Keyed by request: every show starts fresh and replays the
          entrance. */}
      <section
        key={seq}
        className="panel"
        data-panel={panelId}
        aria-labelledby="panel-title"
      >
        <h1 id="panel-title">{t.panel.titles[panelId]}</h1>
        <PanelBoundary
          resetKey={panelId}
          fallback={t.panel.failed}
          onError={logPanelFailure}
        >
          <PanelContent panelId={panelId} settings={settings} />
        </PanelBoundary>
      </section>
    </div>
  );
}

export default Panel;
