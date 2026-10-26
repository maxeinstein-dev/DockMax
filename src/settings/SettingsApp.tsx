import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useApps } from "../core/apps";
import { errorKind, log } from "../core/log";
import { useSettings } from "../core/settings";
import { useWidgets } from "../core/widgets";
import { useT } from "../i18n";
import AboutPanel from "../panel/AboutPanel";
import AppsPanel from "../panel/AppsPanel";
import DockItems from "../panel/DockItems";
import LookPanel from "../panel/LookPanel";
import SettingsPanel from "../panel/SettingsPanel";
import { SECTIONS, type Section } from "./sections";
import "./SettingsApp.css";

function closeWindow() {
  getCurrentWindow()
    .close()
    .catch((error: unknown) => {
      log.warn(`settings: close failed: ${errorKind(error)}`);
    });
}

/**
 * The Configurações app (spec 0011 H1): an ordinary window, made when the
 * user opens the settings and destroyed when it closes. A side bar of
 * sections and, beside it, the same views the panel had, at a width that fits
 * them. Every choice is saved at once, as in the panel; Esc closes the window,
 * as it closes a panel.
 */
function SettingsApp() {
  const t = useT();
  const text = t.settings.app;
  const settings = useSettings();
  const widgets = useWidgets();
  const apps = useApps();
  const [section, setSection] = useState<Section>("general");

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeWindow();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <div className="settings-app">
      <nav className="settings-nav" aria-label={text.nav}>
        {SECTIONS.map((each) => (
          <button
            key={each}
            type="button"
            className="settings-nav-item"
            aria-current={each === section ? "page" : undefined}
            onClick={() => setSection(each)}
          >
            {text.sections[each]}
          </button>
        ))}
      </nav>
      <main className="settings-main">
        <section
          key={section}
          className="panel settings-card"
          data-section={section}
          aria-labelledby="panel-title"
        >
          {section === "general" ? (
            <>
              <h1 id="panel-title">{t.panel.titles.settings}</h1>
              <SettingsPanel
                settings={settings}
                widgets={widgets}
                apps={apps}
                withTrayInDock
                withLauncher
              />
            </>
          ) : null}
          {section === "look" ? <LookPanel settings={settings} /> : null}
          {section === "apps" ? <AppsPanel snapshot={apps} /> : null}
          {section === "items" ? <DockItems snapshot={widgets} /> : null}
          {section === "about" ? (
            <>
              <h1 id="panel-title">{t.panel.titles.about}</h1>
              <AboutPanel />
            </>
          ) : null}
        </section>
      </main>
    </div>
  );
}

export default SettingsApp;
