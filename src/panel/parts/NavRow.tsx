import { useT } from "../../i18n";
import { useFocusOnMount } from "./focus";
import Icon from "./Icon";
import "./parts.css";

export interface NavRowProps {
  /** The subview it leads to, such as "Itens do dock". */
  label: string;
  /** At the right, in the secondary ink: "3 de 12", "Indisponível". */
  value: string;
  /** Shows the warning dot (the storage was recovered or is unavailable),
   * and says so to screen readers. */
  warning?: boolean;
  onClick: () => void;
  /** Focuses the row when it mounts: coming back from its subview. */
  focusOnMount?: boolean;
}

/** Navigation row (DESIGN.md "Linha de navegação"): a full-width button to
 * a subview, with its value and an arrow. */
function NavRow({
  label,
  value,
  warning = false,
  onClick,
  focusOnMount = false,
}: NavRowProps) {
  const t = useT();
  const ref = useFocusOnMount<HTMLButtonElement>(focusOnMount);
  return (
    <button ref={ref} type="button" className="nav-row" onClick={onClick}>
      <span className="nav-row-label">{label}</span>
      <span className="nav-row-value">
        {warning ? (
          <>
            <span className="warning-dot" aria-hidden="true" />
            <span className="visually-hidden">{t.widgets.notice}: </span>
          </>
        ) : null}
        {value}
      </span>
      <Icon name="next" />
    </button>
  );
}

export default NavRow;
