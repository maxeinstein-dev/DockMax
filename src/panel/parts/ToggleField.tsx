import { useId } from "react";
import "./parts.css";

export interface ToggleFieldProps {
  /** The name of the setting; also the accessible name of the switch. */
  label: string;
  /** A line under the label that says what it does. */
  hint?: string;
  checked: boolean;
  /** Called with the new value when the switch is toggled. */
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

/** A switch row: the label and its hint at the left, the switch at the
 * right (`role="switch"`, one Tab stop, Space and Enter toggle it). */
function ToggleField({
  label,
  hint,
  checked,
  onChange,
  disabled,
}: ToggleFieldProps) {
  const labelId = useId();
  const hintId = useId();
  return (
    <div className="toggle-row">
      <div className="toggle-text">
        <span id={labelId} className="toggle-label">
          {label}
        </span>
        {hint ? (
          <span id={hintId} className="toggle-hint">
            {hint}
          </span>
        ) : null}
      </div>
      <button
        type="button"
        role="switch"
        className="switch"
        aria-checked={checked}
        aria-labelledby={labelId}
        aria-describedby={hint ? hintId : undefined}
        disabled={disabled}
        onClick={() => onChange(!checked)}
      />
    </div>
  );
}

export default ToggleField;
