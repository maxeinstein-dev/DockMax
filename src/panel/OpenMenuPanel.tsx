import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { isAppCommandError, type AppErrorCode } from "../core/apps";
import {
  closeOpen,
  launchOpen,
  pinOpen,
  type OpenOther,
} from "../core/openApps";
import { format, useT } from "../i18n";
import PanelFooter from "./PanelFooter";
import PanelHeader from "./parts/PanelHeader";
import { menuItems, moveInMenu } from "./parts/menu";
import "./AppMenuPanel.css";

/** Props of the menu of an open program that is not pinned (spec 0009 D3). */
export interface OpenMenuPanelProps {
  other: OpenOther;
  /** An action was done: the host closes the panel. */
  onDone: () => void;
}

/**
 * The menu of an open program that is not pinned, opened from its icon with a
 * right click: open a new instance, close its windows, pin it to the dock.
 * The program is named by the opaque key Rust gave it; the page never holds a
 * path or a window.
 */
function OpenMenuPanel({ other, onDone }: OpenMenuPanelProps) {
  const t = useT();
  const text = t.apps.menu;
  const [error, setError] = useState<AppErrorCode | null>(null);
  const root = useRef<HTMLDivElement>(null);

  // The first item takes the focus, so the arrow keys work at once.
  useEffect(() => {
    menuItems(root.current)[0]?.focus();
  }, []);

  function done(action: Promise<void>) {
    setError(null);
    action.then(onDone, (reason: unknown) =>
      setError(isAppCommandError(reason) ? reason.code : "notReady"),
    );
  }

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) =>
    moveInMenu(root.current, event);

  return (
    <>
      <PanelHeader title={other.name} />
      <div
        ref={root}
        className="app-menu"
        role="menu"
        aria-label={format(text.label, { name: other.name })}
        onKeyDown={onKeyDown}
      >
        <div className="app-menu-section" role="group">
          <button
            type="button"
            role="menuitem"
            className="app-menu-item"
            onClick={() => done(launchOpen(other.key))}
          >
            {text.newInstance}
          </button>
          <button
            type="button"
            role="menuitem"
            className="app-menu-item"
            onClick={() => done(closeOpen(other.key))}
          >
            {other.count > 1 ? text.closeWindows : text.closeWindow}
          </button>
          <button
            type="button"
            role="menuitem"
            className="app-menu-item"
            onClick={() => done(pinOpen(other.key))}
          >
            {text.pin}
          </button>
        </div>
      </div>
      <PanelFooter alert={error && t.apps.errors[error]}>
        {text.openFooter}
      </PanelFooter>
    </>
  );
}

export default OpenMenuPanel;
