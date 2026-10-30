import type { DockSettings } from "../core/settings";
import {
  THEMES,
  choicesOf,
  themeOf,
  type Theme,
  type ThemeId,
} from "../core/themes";
import { useT } from "../i18n";
import ChoiceCards from "./parts/ChoiceCards";
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

/**
 * The themes of the dock (spec 0020) as a grid of cards like the styles,
 * each with the dock drawn in it. A card applies the theme's style, icon
 * pack and mark at once; the one in use is chosen, and none when the user
 * changed one of those ("Personalizado").
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
  onApply: (
    choices: Pick<DockSettings, "style" | "iconPack" | "indicatorColor">,
  ) => void;
}) {
  const text = useT().settings.look;
  const current = dock ? themeOf(dock) : null;
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
          if (theme) onApply(choicesOf(theme));
        }}
        thumb={(id) => {
          const theme = THEMES.find((each) => each.id === id);
          return theme ? (
            <ThemeThumb theme={theme} dock={dock} appIds={appIds} />
          ) : null;
        }}
        wide
      />
      {dock !== null && current === null ? (
        <p className="look-hint">{text.themeCustom}</p>
      ) : null}
    </div>
  );
}

export default ThemeGallery;
