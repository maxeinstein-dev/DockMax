import { useId, type KeyboardEvent } from "react";
import "./parts.css";

export interface TextFieldProps {
  /** The field's name: shown above it, or only its accessible name with
   * `hideLabel` (the workday hours, on the period's line). */
  label: string;
  hideLabel?: boolean;
  value: string;
  /** Every edit, with the text as typed. */
  onChange: (value: string) => void;
  /** The user is done with the field: it lost the focus, or Enter was
   * pressed. Where a field saves (the label), it saves here, not on every
   * keystroke. */
  onCommit?: () => void;
  type?: "text" | "time";
  placeholder?: string;
  /** Marks the field invalid (`aria-invalid`) and points at the text that
   * says why. No `maxLength`: the browser counts UTF-16 units, Rust counts
   * code points (`labelLength`), so the limit is checked by the caller. */
  invalid?: boolean;
  /** Id of the element that describes the field (a hint or an error). */
  describedBy?: string;
  disabled?: boolean;
}

/** Text field of Windows 11 (DESIGN.md "Inputs / Fields"): label above, the
 * field with its bottom line, accent on focus. The note's editor and the
 * zone list use the same look through the `field-input` class. */
function TextField({
  label,
  hideLabel = false,
  value,
  onChange,
  onCommit,
  type = "text",
  placeholder,
  invalid = false,
  describedBy,
  disabled,
}: TextFieldProps) {
  const id = useId();
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") onCommit?.();
  };
  return (
    <div className={type === "time" ? "field time" : "field"}>
      {hideLabel ? null : (
        <label className="field-label" htmlFor={id}>
          {label}
        </label>
      )}
      <input
        id={id}
        className="field-input"
        type={type}
        value={value}
        placeholder={placeholder}
        aria-label={hideLabel ? label : undefined}
        aria-invalid={invalid || undefined}
        aria-describedby={describedBy}
        disabled={disabled}
        spellCheck={false}
        autoComplete="off"
        onChange={(event) => onChange(event.target.value)}
        onBlur={onCommit}
        onKeyDown={onKeyDown}
      />
    </div>
  );
}

export default TextField;
