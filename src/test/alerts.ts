import { screen, waitFor, within } from "@testing-library/react";

/**
 * The alert that says something, in `container` (the page by default), or
 * null. A live region stays mounted while it has nothing to say, since a
 * screen reader doesn't read one mounted with its text (PanelFooter,
 * `.live`), so an empty alert is no alert. More than one saying something
 * is an error, as with `queryByRole`.
 */
export function queryAlert(container?: HTMLElement): HTMLElement | null {
  const said = (container ? within(container) : screen)
    .queryAllByRole("alert")
    .filter((alert) => alert.textContent?.trim());
  if (said.length > 1) {
    throw new Error(`${said.length} alerts say something, not one`);
  }
  return said[0] ?? null;
}

/** The alert that says something; throws when none does. */
export function getAlert(container?: HTMLElement): HTMLElement {
  const alert = queryAlert(container);
  if (alert === null) throw new Error("no alert says anything");
  return alert;
}

/** The live regions on the page now. */
export const liveRegions = (): HTMLElement[] => [
  ...document.querySelectorAll<HTMLElement>('[role="status"], [role="alert"]'),
];

/**
 * Expects what `said` says to come from a live region already in `before`
 * (taken before it was said): many screen readers don't read a region
 * mounted with its text, only a change in one they know.
 */
export function expectSaidIn(
  before: readonly HTMLElement[],
  said: HTMLElement,
): void {
  const region = said.closest<HTMLElement>('[role="status"], [role="alert"]');
  if (region === null || !before.includes(region)) {
    throw new Error(
      `"${said.textContent}" was said by a live region mounted with it`,
    );
  }
}

/** Waits for an alert to say something. */
export const findAlert = (container?: HTMLElement) =>
  waitFor(() => getAlert(container));
