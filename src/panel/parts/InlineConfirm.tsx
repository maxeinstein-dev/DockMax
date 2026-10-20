import { useFocusOnMount } from "./focus";
import "./parts.css";

export interface InlineConfirmProps {
  /** The main sentence, in strong text: "Remover a nota apaga o texto." */
  title: string;
  /** The consequence after it: "Não dá para desfazer." */
  detail: string;
  cancelLabel: string;
  /** The danger button: "Remover nota". */
  confirmLabel: string;
  onCancel: () => void;
  onConfirm: () => void;
  /** While the confirmed action runs: both buttons off. */
  busy?: boolean;
}

/** Inline confirmation (DESIGN.md "Confirmação inline"): takes the place of
 * the control that asked, never a modal. Mounting it focuses "Cancelar",
 * the safe choice. */
function InlineConfirm({
  title,
  detail,
  cancelLabel,
  confirmLabel,
  onCancel,
  onConfirm,
  busy = false,
}: InlineConfirmProps) {
  const cancel = useFocusOnMount<HTMLButtonElement>(true);
  return (
    <div className="inline-confirm" role="alert">
      <p>
        <strong>{title}</strong> {detail}
      </p>
      <div className="inline-confirm-actions">
        <button
          ref={cancel}
          type="button"
          className="secondary-button"
          disabled={busy}
          onClick={onCancel}
        >
          {cancelLabel}
        </button>
        <button
          type="button"
          className="danger-button"
          disabled={busy}
          onClick={onConfirm}
        >
          {confirmLabel}
        </button>
      </div>
    </div>
  );
}

export default InlineConfirm;
