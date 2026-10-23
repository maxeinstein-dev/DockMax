import { invoke } from "@tauri-apps/api/core";
import { clickIcon, useTray, visibleIcons, type TrayIcon } from "../core/tray";
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
 * area while the Windows taskbar is hidden.
 */
function TrayPanel() {
  const t = useT();
  const icons = useTray();
  const shown = icons === null ? [] : visibleIcons(icons);

  // A left click opens a window of the program, which would sit under the
  // panel: it closes. A right click opens the program's own menu and the
  // panel stays (Rust holds it open while the menu lasts).
  const press = (icon: TrayIcon, button: "left" | "right") => {
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
                  onClick={() => press(icon, "left")}
                  onContextMenu={(event) => {
                    event.preventDefault();
                    press(icon, "right");
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
