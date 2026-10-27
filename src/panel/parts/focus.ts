import { createContext, useEffect, useRef } from "react";

/** Id of the panel's heading: `aria-labelledby` of the card in `Panel.tsx`
 * points at it, so every view's title names the panel. */
export const PANEL_TITLE_ID = "panel-title";

/** A ref that focuses its element once, when it mounts with `focus` true:
 * for the control a view switch lands on (DESIGN.md "Painel": the focus
 * goes back to the control it came from). `preventScroll`, so the panel
 * never jumps. */
export function useFocusOnMount<T extends HTMLElement>(focus: boolean) {
  const ref = useRef<T>(null);
  // Only the value at mount counts: a later change never steals the focus.
  const initial = useRef(focus);
  useEffect(() => {
    if (initial.current) ref.current?.focus({ preventScroll: true });
  }, []);
  return ref;
}

/** Set by the Configurações app, whose section header already names the
 * view (spec 0014 R1): a panel header with no action then draws nothing. */
export const NamedBySection = createContext(false);
