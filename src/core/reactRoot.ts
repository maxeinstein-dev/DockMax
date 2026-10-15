import type { RootOptions } from "react-dom/client";
import { errorKind, log } from "./log";

/* Options for the React root of both windows. React's defaults print render
 * errors, message and component stack included, on the console, in
 * production too. These send them to the app log as the kind of error only
 * (ADR 0007), and nothing to the console. */
export const ROOT_OPTIONS = {
  /** An error caught by an error boundary is reported by the boundary
   * itself (`PanelBoundary` through `onError`, as "panel: render failed").
   * Logging it here too would record every failure twice. */
  onCaughtError: () => {},
  /** No boundary caught it: React unmounts the whole window. */
  onUncaughtError: (error: unknown) => {
    log.error(`react: render failed: ${errorKind(error)}`);
  },
  onRecoverableError: (error: unknown) => {
    log.warn(`react: recovered from a render error: ${errorKind(error)}`);
  },
} satisfies RootOptions;
