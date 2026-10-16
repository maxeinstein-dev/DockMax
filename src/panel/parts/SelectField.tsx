import { useId } from "react";
import "./parts.css";

export interface SelectOption {
  value: string;
  label: string;
}

export interface SelectFieldProps {
  /** Shown above the list; also its accessible name. */
  label: string;
  value: string;
  options: readonly SelectOption[];
  /** Called with the chosen option's value, only when it changes. */
  onChange: (value: string) => void;
  disabled?: boolean;
}

/** A list in the text field's look (DESIGN.md "Inputs / Fields": text,
 * list, hour and the Nota share it): the clock's time zone. */
function SelectField({
  label,
  value,
  options,
  onChange,
  disabled,
}: SelectFieldProps) {
  const id = useId();
  return (
    <div className="field">
      <label className="field-label" htmlFor={id}>
        {label}
      </label>
      <select
        id={id}
        className="field-input"
        value={value}
        disabled={disabled}
        onChange={(event) => {
          if (event.target.value !== value) onChange(event.target.value);
        }}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}

export default SelectField;
