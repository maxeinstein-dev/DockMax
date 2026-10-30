import { useState } from "react";
import { version } from "../../package.json";
import { errorKind, log } from "../core/log";
import { openTour } from "../core/tour";
import {
  checkForUpdate,
  installUpdate,
  percent,
  useUpdate,
} from "../core/update";
import { format, useT } from "../i18n";
import PanelFooter from "./PanelFooter";

/** Names, not texts: they stay the same in every locale. */
const PRODUCT_NAME = "Dockyard";

/** What a click on Procurar or Atualizar left to say. */
type Outcome =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "upToDate" }
  | { kind: "checkFailed"; error: string }
  | { kind: "installFailed"; error: string };

/** The text of a rejected command: Rust answers a string. */
function reason(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return errorKind(error);
}

/**
 * Sobre's updates (spec 0019 G3): the new version and Atualizar, its
 * download, or Procurar atualizações with its answer.
 */
function UpdateRow() {
  const t = useT();
  const text = t.about.update;
  const [update, setUpdate] = useUpdate();
  const [outcome, setOutcome] = useState<Outcome>({ kind: "idle" });

  const check = () => {
    setOutcome({ kind: "checking" });
    checkForUpdate().then(
      (found) => {
        setUpdate(found);
        setOutcome(
          found.state === "none" ? { kind: "upToDate" } : { kind: "idle" },
        );
      },
      (error: unknown) => {
        log.warn(`update: check failed: ${errorKind(error)}`);
        setOutcome({ kind: "checkFailed", error: reason(error) });
      },
    );
  };

  const install = () => {
    setOutcome({ kind: "idle" });
    installUpdate().catch((error: unknown) => {
      log.warn(`update: install failed: ${errorKind(error)}`);
      setOutcome({ kind: "installFailed", error: reason(error) });
    });
  };

  let line: string | null = null;
  if (outcome.kind === "upToDate") line = text.upToDate;
  if (outcome.kind === "checkFailed") {
    line = format(text.checkFailed, { error: outcome.error });
  }
  if (outcome.kind === "installFailed") {
    line = format(text.installFailed, { error: outcome.error });
  }

  if (update.state === "available") {
    return (
      <div className="about-update">
        <p className="about-update-line">
          {format(text.available, { version: update.version })}
        </p>
        <button
          type="button"
          className="secondary-button about-update-install"
          onClick={install}
        >
          {text.install}
        </button>
        {line ? (
          <p className="about-update-note" role="status">
            {line}
          </p>
        ) : null}
      </div>
    );
  }

  if (update.state === "downloading") {
    const done = percent(update.downloaded, update.total);
    return (
      <div className="about-update">
        <p className="about-update-line" role="status">
          {done === null
            ? format(text.downloadingNoSize, { version: update.version })
            : format(text.downloading, {
                version: update.version,
                percent: done,
              })}
        </p>
        {done === null ? null : (
          <progress
            className="about-update-progress"
            max={100}
            value={done}
            aria-label={text.progress}
          />
        )}
      </div>
    );
  }

  if (update.state === "installing") {
    return (
      <div className="about-update">
        <p className="about-update-line" role="status">
          {format(text.installing, { version: update.version })}
        </p>
      </div>
    );
  }

  const checking = outcome.kind === "checking";
  return (
    <div className="about-update">
      <button
        type="button"
        className="secondary-button"
        onClick={check}
        disabled={checking}
      >
        {checking ? text.checking : text.check}
      </button>
      {line ? (
        <p className="about-update-note" role="status">
          {line}
        </p>
      ) : null}
    </div>
  );
}

/**
 * The About panel (DESIGN.md "Sobre"): the app tile, name and description,
 * then version, license and where the code lives, and the updates.
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
      <UpdateRow />
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
