import type { LogicalSize } from "../core/panels";

/** Transparent margin around the visible panel card, in logical pixels: room
 * for its shadow inside the window. At most the 8 px between the card and
 * the dock bar, so the window (which blocks clicks) never covers the bar.
 * Must match `--panel-inset` in Panel.css (pinned by geometry.test.ts). */
export const PANEL_INSET = 8;

/** Size the panel window needs to show `frame` whole: the frame is the card
 * plus `PANEL_INSET` on every side. Rounded up so a fractional line height
 * never clips the card. Measures the frame, which is never transformed, so
 * the card's entrance animation doesn't change the result. */
export function measureFrame(frame: Element): LogicalSize {
  const { width, height } = frame.getBoundingClientRect();
  return { width: Math.ceil(width), height: Math.ceil(height) };
}
