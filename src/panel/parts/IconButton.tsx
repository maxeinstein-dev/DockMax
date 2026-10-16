import type { Ref } from "react";
import Icon, { type IconName } from "./Icon";
import "./parts.css";

export interface IconButtonProps {
  icon: Extract<IconName, "up" | "down" | "back">;
  /** Accessible name, such as `Subir Relógio` or `Voltar`: the button has
   * no visible text. */
  label: string;
  onClick: () => void;
  /** Off at either end of a list: dimmed and skipped by Tab. */
  disabled?: boolean;
  ref?: Ref<HTMLButtonElement>;
}

/** Icon button (DESIGN.md "Buttons": Ícone, 26 × 26 px): Voltar, subir and
 * descer. */
function IconButton({ icon, label, onClick, disabled, ref }: IconButtonProps) {
  return (
    <button
      ref={ref}
      type="button"
      className="icon-button"
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
    >
      <Icon name={icon} />
    </button>
  );
}

export default IconButton;
