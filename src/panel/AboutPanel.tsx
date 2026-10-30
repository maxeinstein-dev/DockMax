import { version } from "../../package.json";
import { errorKind, log } from "../core/log";
import { openTour } from "../core/tour";
import { useT } from "../i18n";
import PanelFooter from "./PanelFooter";

/** Names, not texts: they stay the same in every locale. */
const PRODUCT_NAME = "Dockyard";

/**
 * The About panel (DESIGN.md "Sobre"): the app tile, name and description,
 * then version, license and where the code lives.
 *
 * The repository is selectable text, not a link: the panel window has no
 * way to open the default browser (no opener plugin or permission), and a
 * link would only navigate the panel's own WebView.
 */
function AboutPanel() {
  const t = useT();
  return (
    <>
      <div className="about-head">
        <span className="about-tile t-about" aria-hidden="true">
          <svg viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="8.5" />
            <path d="M12 11v5.5" />
            <path d="M12 7.6h.01" />
          </svg>
        </span>
        <div>
          <h2>{PRODUCT_NAME}</h2>
          <p>{t.about.description}</p>
        </div>
      </div>
      <dl className="about-list">
        <div>
          <dt>{t.about.labels.version}</dt>
          <dd>{version}</dd>
        </div>
        <div>
          <dt>{t.about.labels.license}</dt>
          <dd>{t.about.licenseValue}</dd>
        </div>
        <div>
          <dt>{t.about.labels.code}</dt>
          <dd className="about-code">{t.about.repository}</dd>
        </div>
      </dl>
      <button
        type="button"
        className="secondary-button about-tour"
        onClick={() =>
          void openTour().catch((error: unknown) => {
            log.warn(`tour: open failed: ${errorKind(error)}`);
          })
        }
      >
        {t.about.tour}
      </button>
      <PanelFooter>{t.about.footer}</PanelFooter>
    </>
  );
}

export default AboutPanel;
