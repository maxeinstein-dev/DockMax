import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  PANEL_READY_COMMAND,
  PANEL_SHOW_EVENT,
  isWidgetPanel,
  panelKey,
  type LogicalSize,
  type PanelId,
  type PanelReadyArgs,
  type PanelShowPayload,
} from "../core/panels";
import { useApps } from "../core/apps";
import { useSettings } from "../core/settings";
import {
  useWidgets,
  type CreatableKind,
  type WidgetInstance,
  type WidgetsSnapshot,
} from "../core/widgets";
import PanelBoundary from "../components/PanelBoundary";
import { errorKind, log } from "../core/log";
import { useT } from "../i18n";
import { isCreatableInstance } from "../widgets/registry";
import PanelContent from "./PanelContent";
import { PANEL_INSET, measureFrame } from "./geometry";
import { MAIN_VIEW, type PanelView } from "./view";
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
  /** Rust's number for this show, sent back in `panel_ready`. */
  wait: number;
  seq: number;
}

/** The view chosen for one request (`seq`): any other request starts on
 * its main view. */
interface ChosenView extends PanelView {
  seq: number;
}

/** The instance a widget panel shows, from the list; null when the list
 * doesn't have it (removed, or not read). */
function instanceOf(
  snapshot: WidgetsSnapshot | null,
  panelId: PanelId,
): WidgetInstance<CreatableKind> | null {
  if (!isWidgetPanel(panelId) || snapshot === null) return null;
  const found = snapshot.widgets.find(
    (instance) => instance.id === panelId.widget,
  );
  return found && isCreatableInstance(found) ? found : null;
}

const sameSize = (a: LogicalSize | null, b: LogicalSize) =>
  a !== null && a.width === b.width && a.height === b.height;

function Panel() {
  const t = useT();
  const [request, setRequest] = useState<ShowRequest | null>(null);
  const [chosen, setChosen] = useState<ChosenView | null>(null);
  const frameRef = useRef<HTMLDivElement>(null);
  // Read here, not in the panels, so they are loaded long before the panel
  // first shows.
  const settings = useSettings();
  const widgets = useWidgets();
  const apps = useApps();
  /** The size last sent for the current request. */
  const sentSize = useRef<LogicalSize | null>(null);
  /** The request whose panel was closed for its instance being gone. */
  const closedFor = useRef<number | null>(null);

  useEffect(() => {
    const unlisten = listen<PanelShowPayload>(PANEL_SHOW_EVENT, (event) => {
      const { panelId, wait } = event.payload;
      setRequest((previous) => ({
        panelId,
        wait,
        seq: (previous?.seq ?? 0) + 1,
      }));
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

  const view: PanelView =
    request !== null && chosen?.seq === request.seq ? chosen : MAIN_VIEW;
  const instance = request ? instanceOf(widgets, request.panelId) : null;
  // A widget panel whose instance is gone (removed elsewhere, or the
  // storage became unavailable) closes, once the list is known.
  const gone =
    request !== null &&
    isWidgetPanel(request.panelId) &&
    widgets !== null &&
    instance === null;
  // What the window shows: the size is sent again whenever it changes.
  const viewKey = `${view.sub} ${instance?.kind ?? ""}`;

  /** Closes the panel once for the current request. */
  const closeOnce = (seq: number) => {
    if (closedFor.current === seq) return;
    closedFor.current = seq;
    hidePanel();
  };

  useEffect(() => {
    if (gone && request) closeOnce(request.seq);
  });

  // Runs after the content of this request (or of its new view) is in the
  // DOM and laid out, and before the browser paints: the size sent is
  // always the requested panel's, never the one shown before it. Rust shows
  // the window only then, and places it again for a later answer to the
  // same show.
  useLayoutEffect(() => {
    const frame = frameRef.current;
    if (request === null || frame === null) return;
    const size = measureFrame(frame);
    sentSize.current = size;
    panelReady({
      panelId: request.panelId,
      wait: request.wait,
      size,
      inset: PANEL_INSET,
    });
  }, [request, viewKey]);

  // The content can change size within a view (a confirmation, a notice):
  // the window follows.
  useEffect(() => {
    const frame = frameRef.current;
    if (
      request === null ||
      frame === null ||
      typeof ResizeObserver === "undefined"
    ) {
      return;
    }
    const observer = new ResizeObserver(() => {
      const size = measureFrame(frame);
      if (sameSize(sentSize.current, size)) return;
      sentSize.current = size;
      panelReady({
        panelId: request.panelId,
        wait: request.wait,
        size,
        inset: PANEL_INSET,
      });
    });
    observer.observe(frame);
    return () => observer.disconnect();
  }, [request]);

  if (request === null) return null;
  const { panelId, seq } = request;
  const onView = (next: PanelView) => setChosen({ ...next, seq });
  // Itens do dock and the widget views bring their own title
  // (`PanelHeader`).
  const ownTitle =
    isWidgetPanel(panelId) || (panelId === "settings" && view.sub);

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
        data-panel={isWidgetPanel(panelId) ? "widget" : panelId}
        data-widget={instance?.kind}
        aria-labelledby="panel-title"
      >
        {ownTitle ? null : <h1 id="panel-title">{t.panel.titles[panelId]}</h1>}
        <PanelBoundary
          resetKey={`${panelKey(panelId)} ${view.sub}`}
          fallback={t.panel.failed}
          onError={logPanelFailure}
        >
          <PanelContent
            panelId={panelId}
            settings={settings}
            widgets={widgets}
            apps={apps}
            instance={instance}
            view={view}
            onView={onView}
            onRemoved={() => closeOnce(seq)}
          />
        </PanelBoundary>
      </section>
    </div>
  );
}

export default Panel;
