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

/** How many cards the grid lays in a row: those as high as the first
 * (`auto-fill` makes it depend on the window's width). At least one. */
function columnsOf(cards: readonly (HTMLElement | null)[]): number {
  const top = cards[0]?.offsetTop;
  const inRow = cards.findIndex((card) => card?.offsetTop !== top);
  return inRow > 0 ? inRow : Math.max(cards.length, 1);
}

/** The card under `current`, in a grid of `columns` and `last + 1` cards:
 * the last card when the row below is too short to have one, and the top
 * of the column from the last row. In a single row, the next card, as in
 * any radio group. */
function rowBelow(current: number, columns: number, last: number): number {
  if (last < columns) return current === last ? 0 : current + 1;
  if (current + columns <= last) return current + columns;
  const lastRow = Math.floor(last / columns);
  return Math.floor(current / columns) < lastRow ? last : current % columns;
}

/** The card over `current`; from the first row, the bottom of the column
 * (the row before the last when the last is too short to reach it). In a
 * single row, the card before. */
function rowAbove(current: number, columns: number, last: number): number {
  if (last < columns) return current === 0 ? last : current - 1;
  if (current - columns >= 0) return current - columns;
  const bottom = current + Math.floor(last / columns) * columns;
  return bottom <= last ? bottom : bottom - columns;
}

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
 * picture over its name (spec 0014 R1, R2): one choice of many, so a
 * labelled radio group, the chosen card `aria-checked`. One Tab stop
 * (roving tabindex, at the chosen card); the arrows, Home and End move
 * inside it without choosing (a theme tried on every arrow would change
 * the dock under the user), and Enter, Space or a click chooses at once.
 * Left and Right go card by card; Up and Down go a row of the grid, to the
 * card above or below, wrapping around the column.
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
    const columns = columnsOf(buttons.current);
    const next = {
      ArrowRight: current === last ? 0 : current + 1,
      ArrowDown: rowBelow(current, columns, last),
      ArrowLeft: current === 0 ? last : current - 1,
      ArrowUp: rowAbove(current, columns, last),
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
        role="radiogroup"
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
            role="radio"
            className="choice-card"
            // The name names the card; the hint describes it.
            aria-labelledby={`${labelId}-${index}`}
            aria-describedby={
              choice.hint ? `${labelId}-${index}-hint` : undefined
            }
            aria-checked={index === chosen}
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
