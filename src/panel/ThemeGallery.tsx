import type { DockSettings } from "../core/settings";
import {
  THEMES,
  choicesOf,
  themeOf,
  type Theme,
  type ThemeId,
} from "../core/themes";
import { useEffect, useState } from "react";
import { format, useT } from "../i18n";
import ChoiceCards from "./parts/ChoiceCards";
import "./parts/parts.css";
import { MINI_ITEMS, MiniDock, type GalleryDock } from "./StyleGallery";
import { usePackPreview } from "./usePackPreview";

/** The dock in `theme`: its style, its icons on the first pinned programs
 * and its mark. Decorative: the card's name says which theme it is. */
function ThemeThumb({
  theme,
  dock,
  appIds,
}: {
  theme: Theme;
  dock: GalleryDock | null;
  appIds: readonly string[];
}) {
  const icons = usePackPreview(theme.iconPack, appIds, 0, theme.style);
  const shown: GalleryDock | null = dock && { ...dock, ...choicesOf(theme) };
  return (
    <MiniDock
      of={theme.style}
      dock={shown}
      appIds={appIds}
      icons={icons ?? appIds.slice(0, MINI_ITEMS).map(() => null)}
    />
  );
}

type Choices = Pick<DockSettings, "style" | "iconPack" | "indicatorColor">;

/** How long a theme tried waits to be kept before the dock goes back. */
export const KEEP_SECONDS = 15;

/** The seconds left when a screen reader hears again that the theme goes
 * back: the countdown on screen is not read, second by second. */
export const WARN_SECONDS = 5;

/** The dock's three choices a theme makes. */
function choicesNow(dock: GalleryDock): Choices {
  return {
    style: dock.style,
    iconPack: dock.iconPack,
    indicatorColor: dock.indicatorColor,
  };
}

/**
 * The themes of the dock (spec 0020) as a grid of cards like the styles,
 * each with the dock drawn in it. A card applies the theme's style, icon
 * pack and mark at once; the one in use is chosen, and none when the user
 * changed one of those ("Personalizado").
 *
 * Like a new screen resolution in Windows, a theme applied is tried first:
 * "Manter" keeps it, "Voltar ao anterior" (or [`KEEP_SECONDS`] with no
 * answer) puts back the choices the dock had before the first theme tried.
 * Leaving the view keeps what is applied.
 *
 * A screen reader hears the question as it appears, again
 * [`WARN_SECONDS`] before the theme goes back, and that it went back, from
 * a live region always there whose text changes (one mounted with its text
 * is not read). The focus stays on the card: the arrows and Enter keep
 * trying themes, and Manter is the next Tab stop.
 */
function ThemeGallery({
  dock,
  appIds,
  disabled = false,
  onApply,
}: {
  /** The dock's settings shown, or null while they can't be read. */
  dock: GalleryDock | null;
  /** The programs pinned in the workspace in use, in dock order. */
  appIds: readonly string[];
  disabled?: boolean;
  onApply: (choices: Choices) => void;
}) {
  const text = useT().settings.look;
  const current = dock ? themeOf(dock) : null;
  // The choices before the first theme tried, and the seconds left.
  const [trial, setTrial] = useState<{
    before: Choices;
    left: number;
  } | null>(null);
  // The theme went back by itself, said until another one is tried.
  const [reverted, setReverted] = useState(false);
  useEffect(() => {
    if (trial === null) return;
    const tick = window.setTimeout(() => {
      if (trial.left <= 1) {
        onApply(trial.before);
        setTrial(null);
        setReverted(true);
      } else {
        setTrial({ ...trial, left: trial.left - 1 });
      }
    }, 1000);
    return () => window.clearTimeout(tick);
  }, [trial, onApply]);
  const said =
    trial === null
      ? reverted
        ? text.themeReverted
        : ""
      : trial.left <= WARN_SECONDS
        ? format(text.themeRevertSoon, { seconds: WARN_SECONDS })
        : format(text.themeKeepSaid, { seconds: KEEP_SECONDS });

  return (
    <div className="theme-field">
      <ChoiceCards<ThemeId>
        label={text.themes}
        choices={THEMES.map((theme) => ({
          value: theme.id,
          label: text.themeNames[theme.id],
        }))}
        selected={current}
        disabled={disabled}
        onSelect={(id) => {
          const theme = THEMES.find((each) => each.id === id);
          if (!theme || dock === null) return;
          // Trying another theme keeps the way back to the first one's.
          const before = trial?.before ?? choicesNow(dock);
          onApply(choicesOf(theme));
          setTrial({ before, left: KEEP_SECONDS });
          setReverted(false);
        }}
        thumb={(id) => {
          const theme = THEMES.find((each) => each.id === id);
          return theme ? (
            <ThemeThumb theme={theme} dock={dock} appIds={appIds} />
          ) : null;
        }}
        wide
      />
      <p className="visually-hidden" role="status">
        {said}
      </p>
      {trial !== null ? (
        <div className="theme-trial" role="group" aria-label={text.themeKeep}>
          <p className="theme-trial-text">
            {text.themeKeep}{" "}
            <span className="theme-trial-left" aria-hidden="true">
              {format(text.themeRevertIn, { seconds: trial.left })}
            </span>
          </p>
          <div className="theme-trial-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={() => setTrial(null)}
            >
              {text.themeKeepButton}
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={() => {
                onApply(trial.before);
                setTrial(null);
              }}
            >
              {text.themeRevertButton}
            </button>
          </div>
        </div>
      ) : dock !== null && current === null ? (
        <p className="look-hint">{text.themeCustom}</p>
      ) : null}
    </div>
  );
}

export default ThemeGallery;
