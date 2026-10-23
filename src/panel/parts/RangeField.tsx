import { useId, useState } from "react";
import "./parts.css";

export interface RangeFieldProps {
  /** The name of the setting; also the accessible name of the slider. */
  label: string;
  /** A line under the label that says what it does. */
  hint?: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  /** The text of a value, shown at the right and read by screen readers:
   * `40%`, `1,10×`. */
  display: (value: number) => string;
  /** Called once the choice is made (pointer released, key released, or
   * focus left), not on every step of the drag: each call is saved. */
  onCommit: (value: number) => void;
  disabled?: boolean;
}

/** A slider row. While it is dragged the value follows the pointer on the
 * screen only; the settings are changed, and saved, once the user lets go. */
function RangeField({
  label,
  hint,
  value,
  min,
  max,
  step = 1,
  display,
  onCommit,
  disabled,
}: RangeFieldProps) {
  const id = useId();
  const hintId = useId();
  const [draft, setDraft] = useState<number | null>(null);
  const shown = draft ?? value;

  const commit = () => {
    if (draft !== null && draft !== value) onCommit(draft);
    setDraft(null);
  };

  return (
    <div className="range-row">
      <div className="range-head">
        <label htmlFor={id} className="range-label">
          {label}
        </label>
        <span className="range-value" aria-hidden="true">
          {display(shown)}
        </span>
      </div>
      {hint ? (
        <span id={hintId} className="range-hint">
          {hint}
        </span>
      ) : null}
      <input
        id={id}
        className="range"
        type="range"
        min={min}
        max={max}
        step={step}
        value={shown}
        disabled={disabled}
        aria-valuetext={display(shown)}
        aria-describedby={hint ? hintId : undefined}
        onChange={(event) => setDraft(Number(event.target.value))}
        onPointerUp={commit}
        onKeyUp={commit}
        onBlur={commit}
      />
    </div>
  );
}

export default RangeField;
