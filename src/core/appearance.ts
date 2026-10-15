import { useEffect } from "react";
import {
  SIZE_PRESETS,
  useSettings,
  type DockEdge,
  type Settings,
  type SizePreset,
} from "./settings";

/** The window a page runs in: the dock's opacity, size and edge are its
 * own; the panel is always opaque (DESIGN.md, The Opaque Panel Rule) and
 * has no size or edge of its own. */
export type AppWindow = "dock" | "panel";

/** Size preset of the dock as `src/themes/tokens.css` names it
 * (`data-size`): Pequeno, Médio, Grande. */
export type DockSize = "s" | "m" | "l";

const DOCK_SIZES: Record<SizePreset, DockSize> = {
  small: "s",
  medium: "m",
  large: "l",
};

/** The preset closest to `iconSize`; a tie goes to the smaller one. The
 * settings accept any size in 32–96, but the dock draws only the presets. */
export function dockSize(iconSize: number): DockSize {
  let best: SizePreset = "medium";
  let bestDistance = Infinity;
  for (const [preset, size] of Object.entries(SIZE_PRESETS) as [
    SizePreset,
    number,
  ][]) {
    const distance = Math.abs(size - iconSize);
    if (
      distance < bestDistance ||
      (distance === bestDistance && size < SIZE_PRESETS[best])
    ) {
      best = preset;
      bestDistance = distance;
    }
  }
  return DOCK_SIZES[best];
}

/** Size and edge the dock draws when there are no settings (loading, or
 * unreadable): the CSS defaults. */
export const DEFAULT_DOCK_LOOK: { size: DockSize; edge: DockEdge } = {
  size: "m",
  edge: "bottom",
};

/**
 * Applies the theme and, in the dock, the opacity, size and edge to `root`
 * as the data attributes `src/themes/tokens.css` and `src/dock/Dock.css`
 * read. The "Sistema" theme removes `data-mode`, so the colors follow
 * `prefers-color-scheme`, which the WebView keeps in sync with Windows
 * without any script.
 */
export function applyAppearance(
  root: HTMLElement,
  settings: Settings,
  { window }: { window: AppWindow },
) {
  if (settings.theme === "system") {
    delete root.dataset.mode;
  } else {
    root.dataset.mode = settings.theme;
  }
  if (window === "dock") {
    root.dataset.opacity = settings.dock.opacity;
    root.dataset.size = dockSize(settings.dock.iconSize);
    root.dataset.edge = settings.dock.edge;
  }
}

/** Keeps the page's `<html>` in line with the settings; returns them (null
 * while loading, when the CSS defaults apply: Sistema, translucent, Médio
 * and bottom). */
export function useAppearance(window: AppWindow): Settings | null {
  const settings = useSettings();

  useEffect(() => {
    if (settings) {
      applyAppearance(document.documentElement, settings, { window });
    }
  }, [settings, window]);

  return settings;
}
