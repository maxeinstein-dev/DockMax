import type { ReactNode } from "react";
import { useAppearance, type AppWindow } from "../core/appearance";

type AppearanceProps = {
  /** Which window the page is: only the dock applies the opacity. */
  window: AppWindow;
  children?: ReactNode;
};

/** Applies the theme (and the dock opacity) of the settings to the page,
 * following every change. Renders its children as they are. */
function Appearance({ window, children }: AppearanceProps) {
  useAppearance(window);
  return children;
}

export default Appearance;
