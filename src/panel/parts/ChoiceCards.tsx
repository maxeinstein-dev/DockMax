import {
  useId,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import "./ChoiceCards.css";

export type Choice<V extends string> = {
  value: V;
  label: string;
  /** A line under the name, for a choice the name alone doesn't explain. */
  hint?: string;
};

type ChoiceCardsProps<V extends string> = {
  label: string;
  choices: readonly Choice<V>[];
  /** The value in use; null while unknown (no card chosen). */
  selected: V | null;
  disabled?: boolean;
  onSelect: (value: V) => void;
  /** The picture of a choice, decorative: the card's name says which it is.
   * `chosen` lets the chosen one show its effect moving. */
  thumb: (value: V, chosen: boolean) => ReactNode;
  /** Wider cards, fewer per row (the styles: 148 px; the rest: 120 px). */
  wide?: boolean;
};

/**
 * A visual choice of the Configurações app as a grid of cards, each with a
 * picture over its name (spec 0014 R1, R2): a labelled group of toggle
 * buttons, the chosen one `aria-pressed`. One Tab stop (roving tabindex, at
 * the chosen card); the arrows, Home and End move inside it without
 * choosing, and Enter, Space or a click chooses at once.
 */
function ChoiceCards<V extends string>({
  label,
  choices,
  selected,
  disabled = false,
  onSelect,
  thumb,
  wide = false,
}: ChoiceCardsProps<V>) {
  const labelId = useId();
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const [focused, setFocused] = useState<number | null>(null);
  const chosen = choices.findIndex((choice) => choice.value === selected);
  const tabStop = focused ?? Math.max(chosen, 0);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const last = choices.length - 1;
    const from = buttons.current.findIndex(
      (button) => button === document.activeElement,
    );
    const current = from >= 0 ? from : tabStop;
    const next = {
      ArrowRight: current === last ? 0 : current + 1,
      ArrowDown: current === last ? 0 : current + 1,
      ArrowLeft: current === 0 ? last : current - 1,
      ArrowUp: current === 0 ? last : current - 1,
      Home: 0,
      End: last,
    }[event.key];
    if (next === undefined) return;
    event.preventDefault();
    setFocused(next);
    buttons.current[next]?.focus();
  };

  return (
    <div className="choice-field">
      <span id={labelId} className="option-label">
        {label}
      </span>
      <div
        className={`choice-cards${wide ? " wide" : ""}`}
        role="group"
        aria-labelledby={labelId}
        onKeyDown={onKeyDown}
        onBlur={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget)) {
            setFocused(null);
          }
        }}
      >
        {choices.map((choice, index) => (
          <button
            key={choice.value}
            ref={(button) => {
              buttons.current[index] = button;
            }}
            type="button"
            className="choice-card"
            // The name names the card; the hint describes it.
            aria-labelledby={`${labelId}-${index}`}
            aria-describedby={
              choice.hint ? `${labelId}-${index}-hint` : undefined
            }
            aria-pressed={index === chosen}
            tabIndex={index === tabStop ? 0 : -1}
            disabled={disabled}
            onFocus={() => setFocused(index)}
            onClick={() => {
              setFocused(index);
              if (index !== chosen) onSelect(choice.value);
            }}
          >
            {thumb(choice.value, index === chosen)}
            <span className="choice-card-text">
              <span id={`${labelId}-${index}`} className="choice-card-name">
                {choice.label}
              </span>
              {choice.hint ? (
                <span
                  id={`${labelId}-${index}-hint`}
                  className="choice-card-hint"
                >
                  {choice.hint}
                </span>
              ) : null}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}

export default ChoiceCards;
