import type { PanelId } from "../core/panels";
import { useT } from "../i18n";

/** The body of a panel, below its title. Runs inside the panel's error
 * boundary: a failure here leaves the title and Esc working. */
function PanelContent({ panelId }: { panelId: PanelId }) {
  const t = useT();
  switch (panelId) {
    // Placeholders until the panels arrive (PR 3).
    case "settings":
    case "about":
      return <p>{t.panel.underConstruction}</p>;
  }
}

export default PanelContent;
