import { useRef, useState, type CSSProperties, type PointerEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  TRAY_DRAG_THRESHOLD,
  clickIcon,
  dropOnDock,
  panelIcons,
  useTray,
  type TrayIcon,
} from "../core/tray";
import { useT } from "../i18n";
import PanelFooter from "./PanelFooter";
import "./TrayPanel.css";

/** Closes the panel: the window of a program that was clicked takes the
 * foreground, and a panel left open would sit over it. */
function hidePanel() {
  invoke("hide_panel").catch(() => {});
}

/**
 * The notification area in a panel (ADR 0009, Fase E): one button per icon
 * programs put there. A left click does what the taskbar's did, a right
 * click opens the program's menu. The icons come from Rust, which hosts the
 * area while the Windows taskbar is hidden. An icon dragged out and let go on
 * the dock stays in the dock (as the PrimoDock).
 */
function TrayPanel({
  inDock = [],
}: {
  /** The icons the dock shows itself (their ids): not repeated here. */
  inDock?: readonly string[];
}) {
  const t = useT();
  const icons = useTray();
  const shown = icons === null ? [] : panelIcons(icons, inDock);
  /** A press that may become a drag. */
  const press = useRef<{
    key: string;
    x: number;
    y: number;
    dragging: boolean;
  } | null>(null);
  /** The click that ends a drag is not a click on the icon. */
  const dragged = useRef(false);
  const [drag, setDrag] = useState<{
    key: string;
    dx: number;
    dy: number;
  } | null>(null);

  const onPointerDown = (icon: TrayIcon, event: PointerEvent<HTMLElement>) => {
    if (event.button !== 0 || icon.id === null) return;
    press.current = {
      key: icon.key,
      x: event.screenX,
      y: event.screenY,
      dragging: false,
    };
    // The moves and the release come here even outside the panel.
    event.currentTarget.setPointerCapture?.(event.pointerId);
  };

  const onPointerMove = (event: PointerEvent<HTMLElement>) => {
    const current = press.current;
    if (current === null) return;
    const dx = event.screenX - current.x;
    const dy = event.screenY - current.y;
    if (!current.dragging && Math.hypot(dx, dy) < TRAY_DRAG_THRESHOLD) return;
    current.dragging = true;
    setDrag({ key: current.key, dx, dy });
  };

  const onPointerUp = (event: PointerEvent<HTMLElement>) => {
    const current = press.current;
    press.current = null;
    setDrag(null);
    if (current === null || !current.dragging) return;
    dragged.current = true;
    void dropOnDock(
      current.key,
      { x: event.screenX, y: event.screenY },
      window.devicePixelRatio || 1,
    );
  };

  const cancelDrag = () => {
    press.current = null;
    setDrag(null);
  };

  // A left click opens a window of the program, which would sit under the
  // panel: it closes. A right click opens the program's own menu and the
  // panel stays (Rust holds it open while the menu lasts).
  const pressIcon = (icon: TrayIcon, button: "left" | "right") => {
    void clickIcon(icon.key, button).then(() => {
      if (button === "left") hidePanel();
    });
  };

  return (
    <>
      {shown.length > 0 ? (
        <ul className="tray-grid">
          {shown.map((icon) => {
            const name = icon.tip || t.tray.unnamed;
            return (
              <li key={icon.key}>
                <button
                  type="button"
                  className="tray-icon"
                  aria-label={name}
                  title={name}
                  disabled={!icon.clickable}
                  data-dragging={drag?.key === icon.key ? "" : undefined}
                  style={
                    drag?.key === icon.key
                      ? ({
                          "--drag-x": `${drag.dx}px`,
                          "--drag-y": `${drag.dy}px`,
                        } as CSSProperties)
                      : undefined
                  }
                  onPointerDown={(event) => onPointerDown(icon, event)}
                  onPointerMove={onPointerMove}
                  onPointerUp={onPointerUp}
                  onPointerCancel={cancelDrag}
                  onClick={() => {
                    if (dragged.current) {
                      dragged.current = false;
                      return;
                    }
                    pressIcon(icon, "left");
                  }}
                  onContextMenu={(event) => {
                    event.preventDefault();
                    pressIcon(icon, "right");
                  }}
                >
                  {icon.image ? (
                    <img src={icon.image} alt="" draggable={false} />
                  ) : (
                    <span className="tray-icon-blank" aria-hidden="true" />
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      ) : icons === null ? null : (
        <div className="tray-empty">
          <p>{t.tray.empty}</p>
          <p className="tray-empty-hint">{t.tray.emptyHint}</p>
        </div>
      )}
      <PanelFooter>{t.tray.footer}</PanelFooter>
    </>
  );
}

export default TrayPanel;
