import type { ReactNode } from "react";
import { useT } from "../i18n";

/** The key hint at the right of a panel footer: `Esc fecha`. */
export function CloseHint() {
  const t = useT();
  return (
    <span className="close-hint">
      <kbd>{t.panel.closeHint.key}</kbd> {t.panel.closeHint.action}
    </span>
  );
}

type PanelFooterProps = {
  /** The caption at the left. */
  children: ReactNode;
  /** For a footer that reports failures: the message to show, or null for
   * none. While there is one it takes the whole footer, in an alert. Such a
   * footer always keeps room for a two-line message, so a failure never
   * makes the card taller than the window sized for it (`panel_ready` is
   * sent once per show). */
  alert?: string | null;
};

/** Panel footer (DESIGN.md "Rodapé"): a caption at the left and the close
 * hint at the right, under a hairline. */
function PanelFooter({ children, alert }: PanelFooterProps) {
  const reportsFailures = alert !== undefined;
  return (
    <footer className={reportsFailures ? "panel-foot status" : "panel-foot"}>
      {/* Mounted before the failure, so a screen reader reads it. */}
      {reportsFailures ? (
        <p className="live" role="alert">
          {alert}
        </p>
      ) : null}
      {alert ? null : (
        <>
          <span>{children}</span>
          <CloseHint />
        </>
      )}
    </footer>
  );
}

export default PanelFooter;
