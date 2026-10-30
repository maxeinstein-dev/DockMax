/** The sections of the Configurações app, in the order of its side bar; must
 * match `Section` in `src-tauri/src/domain/launcher_pages.rs`. */
export type Section =
  "general" | "workspaces" | "look" | "apps" | "items" | "accounts" | "about";

export const SECTIONS: readonly Section[] = [
  "general",
  "workspaces",
  "look",
  "apps",
  "items",
  "accounts",
  "about",
];

/** Event Rust sends to the open window to show a section (a page the
 * launcher found). */
export const SECTION_EVENT = "settings-section";

export function isSection(value: unknown): value is Section {
  return SECTIONS.some((section) => section === value);
}

declare global {
  interface Window {
    /** The section a window opened by the launcher starts on, set by Rust
     * before the page's scripts run. */
    __DOCKYARD_SECTION__?: string;
  }
}

/** The section the window starts on: the launcher's, else Geral. */
export function startSection(): Section {
  const asked = window.__DOCKYARD_SECTION__;
  return isSection(asked) ? asked : "general";
}
