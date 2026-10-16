import { useT } from "../../i18n";
import { useFocusOnMount } from "./focus";
import Icon from "./Icon";
import "./parts.css";

export interface RemoveButtonProps {
  /** Removes the instance, or asks first (`InlineConfirm` takes this
   * button's place). */
  onClick: () => void;
  /** While a removal runs. */
  disabled?: boolean;
  /** Focuses it when it mounts: coming back from a cancelled
   * confirmation. */
  focusOnMount?: boolean;
}

/** "Remover do dock" (DESIGN.md "Buttons": Remover), at the end of every
 * widget's settings. */
function RemoveButton({
  onClick,
  disabled,
  focusOnMount = false,
}: RemoveButtonProps) {
  const t = useT();
  const ref = useFocusOnMount<HTMLButtonElement>(focusOnMount);
  return (
    <button
      ref={ref}
      type="button"
      className="remove-button"
      disabled={disabled}
      onClick={onClick}
    >
      <Icon name="trash" />
      {t.widgets.common.remove}
    </button>
  );
}

export default RemoveButton;
