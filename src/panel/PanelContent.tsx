import type { PanelId } from "../core/panels";
import type { Settings } from "../core/settings";
import AboutPanel from "./AboutPanel";
import SettingsPanel from "./SettingsPanel";

type PanelContentProps = {
  panelId: PanelId;
  /** The settings in use, or null while they can't be read. */
  settings: Settings | null;
};

/** The body of a panel, below its title. Runs inside the panel's error
 * boundary: a failure here leaves the title and Esc working. */
function PanelContent({ panelId, settings }: PanelContentProps) {
  switch (panelId) {
    case "settings":
      return <SettingsPanel settings={settings} />;
    case "about":
      return <AboutPanel />;
  }
}

export default PanelContent;
