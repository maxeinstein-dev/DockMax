import { useEffect } from "react";
import { useSettings, type Settings } from "./settings";

/** The window a page runs in: the dock's opacity is its own, the panel is
 * always opaque (DESIGN.md, The Opaque Panel Rule). */
export type AppWindow = "dock" | "panel";

/**
 * Applies the theme and, in the dock, the opacity to `root` as the data
 * attributes `src/themes/tokens.css` reads. The "Sistema" theme removes
 * `data-mode`, so the colors follow `prefers-color-scheme`, which the
 * WebView keeps in sync with Windows without any script. Size and edge are
 * the dock layout's job (PR 3).
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
  }
}

/** Keeps the page's `<html>` in line with the settings; returns them (null
 * while loading, when the CSS defaults apply: Sistema and translucent). */
export function useAppearance(window: AppWindow): Settings | null {
  const settings = useSettings();

  useEffect(() => {
    if (settings) {
      applyAppearance(document.documentElement, settings, { window });
    }
  }, [settings, window]);

  return settings;
}
