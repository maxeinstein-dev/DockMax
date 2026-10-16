import { useT } from "../../i18n";
import { PANEL_TITLE_ID, useFocusOnMount } from "./focus";
import Icon from "./Icon";
import IconButton from "./IconButton";
import "./parts.css";

/** The header of a panel view: its title, with at most one action. */
export type PanelHeaderProps = {
  /** The view's title: the instance's label or kind name in the main view,
   * "Ajustes do relógio" and the like in a subview. */
  title: string;
  /** Focuses the action when the header mounts: set it when a view switch
   * lands here (Ajustes → the Voltar button; Voltar → the Ajustes
   * button). */
  focusAction?: boolean;
} & (
  | {
      /** Main view of a widget: the quiet "Ajustes" button at the right. */
      onAdjust: () => void;
      onBack?: never;
    }
  | {
      /** A subview (Ajustes, Itens do dock): the Voltar icon button before
       * the title. */
      onBack: () => void;
      onAdjust?: never;
    }
  | { onAdjust?: never; onBack?: never }
);

/** Panel header (DESIGN.md "Painel": Cabeçalho), a 24 px row. The title is
 * the panel's heading (`PANEL_TITLE_ID`), so it names the card. */
function PanelHeader({
  title,
  onAdjust,
  onBack,
  focusAction = false,
}: PanelHeaderProps) {
  const t = useT();
  const action = useFocusOnMount<HTMLButtonElement>(focusAction);
  return (
    <div className="panel-header">
      {onBack ? (
        <IconButton
          ref={action}
          icon="back"
          label={t.widgets.common.back}
          onClick={onBack}
        />
      ) : null}
      <h1 id={PANEL_TITLE_ID}>{title}</h1>
      {onAdjust ? (
        <button
          ref={action}
          type="button"
          className="quiet-button"
          onClick={onAdjust}
        >
          <Icon name="sliders" />
          {t.widgets.common.adjust}
        </button>
      ) : null}
    </div>
  );
}

export default PanelHeader;
