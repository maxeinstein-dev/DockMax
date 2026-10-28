import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useApps } from "../core/apps";
import { errorKind, log } from "../core/log";
import { useSettings } from "../core/settings";
import { useWidgets } from "../core/widgets";
import { useWorkspaces } from "../core/workspaces";
import { useT } from "../i18n";
import AboutPanel from "../panel/AboutPanel";
import AppsPanel from "../panel/AppsPanel";
import AccountsPanel from "./AccountsPanel";
import WidgetCatalog from "./WidgetCatalog";
import DockItems from "../panel/DockItems";
import LookPanel from "../panel/LookPanel";
import SettingsPanel from "../panel/SettingsPanel";
import WorkspacePicker from "../panel/WorkspacePicker";
import WorkspacesPanel from "../panel/WorkspacesPanel";
import Icon from "../panel/parts/Icon";
import { NamedBySection } from "../panel/parts/focus";
import { inWorkspace } from "../core/workspaces";
import {
  SECTIONS,
  SECTION_EVENT,
  isSection,
  startSection,
  type Section,
} from "./sections";
import "./SettingsApp.css";
import "./SettingsSections.css";

/** The glaze of each section's plate (spec 0014 R1; DESIGN.md "Tertiary",
 * the static .t-* classes of tokens.css). */
const PLATES: Readonly<Record<Section, string>> = {
  general: "t-settings",
  workspaces: "t-clock",
  look: "t-music",
  apps: "t-files",
  items: "t-notes",
  accounts: "t-calendar",
  about: "t-about",
};

/** A section's plate: its glaze with its line icon. Decorative. */
function Plate({
  section,
  large = false,
}: {
  section: Section;
  large?: boolean;
}) {
  return (
    <span
      className={`section-plate ${PLATES[section]}${large ? " large" : ""}`}
      aria-hidden="true"
    >
      <Icon name={section} />
    </span>
  );
}

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
  const spaces = useWorkspaces();
  const [section, setSection] = useState<Section>(startSection);
  /** The workspace being edited (spec 0013): chosen here, whichever the
   * docks show; at first the one in use. */
  const [chosen, setChosen] = useState<string | null>(null);
  const all = spaces?.workspaces ?? [];
  // The programs of the workspace in use, in dock order: the miniatures of
  // the styles show them, as the dock in use does.
  const inUse = all.find((each) => each.id === spaces?.inUse) ?? all[0];
  const pinnedInUse = inWorkspace(apps?.apps ?? [], inUse, "app").map(
    (app) => app.appId,
  );
  const editing =
    all.find((each) => each.id === chosen) ??
    all.find((each) => each.id === spaces?.inUse) ??
    all[0] ??
    null;

  // The launcher found a section with the window already open.
  useEffect(() => {
    let active = true;
    const unlisten = listen<unknown>(SECTION_EVENT, (event) => {
      if (active && isSection(event.payload)) setSection(event.payload);
    });
    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);

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
        <div className="settings-brand" aria-hidden="true">
          <span className="settings-brand-name">DockMax</span>
          <span className="settings-brand-sub">{text.brand}</span>
        </div>
        {SECTIONS.map((each) => (
          <button
            key={each}
            type="button"
            className="settings-nav-item"
            aria-current={each === section ? "page" : undefined}
            onClick={() => setSection(each)}
          >
            <Plate section={each} />
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
          <header className="section-header">
            <Plate section={section} large />
            <div className="section-header-text">
              <h1 id="panel-title">{text.sections[section]}</h1>
              <p className="section-lead">{text.leads[section]}</p>
            </div>
          </header>
          <NamedBySection.Provider value={true}>
            {section === "general" ? (
              <SettingsPanel
                settings={settings}
                widgets={widgets}
                apps={apps}
                withTrayInDock
                withLauncher
                grouped
              />
            ) : null}
            {section === "workspaces" ? (
              <WorkspacesPanel
                snapshot={spaces}
                selected={editing}
                onSelect={setChosen}
                onEditApps={() => setSection("apps")}
                onEditItems={() => setSection("items")}
              />
            ) : null}
            {section === "look" ? (
              <LookPanel settings={settings} appIds={pinnedInUse} grouped />
            ) : null}
            {section === "apps" ? (
              <>
                <WorkspacePicker
                  workspaces={all}
                  selected={editing}
                  onSelect={setChosen}
                />
                <AppsPanel snapshot={apps} workspace={editing} />
              </>
            ) : null}
            {section === "items" ? (
              <>
                <WorkspacePicker
                  workspaces={all}
                  selected={editing}
                  onSelect={setChosen}
                />
                <DockItems snapshot={widgets} workspace={editing} catalog />
                <WidgetCatalog snapshot={widgets} workspace={editing} />
              </>
            ) : null}
            {section === "accounts" ? <AccountsPanel /> : null}
            {section === "about" ? <AboutPanel /> : null}
          </NamedBySection.Provider>
        </section>
      </main>
    </div>
  );
}

export default SettingsApp;
