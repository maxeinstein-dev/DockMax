/* The drag that reorders the pinned programs along the bar (spec 0006 B2).
 * Pure: where the pointer is, which place it means, and what to draw. */

/** How far the pointer goes before a press becomes a drag, in pixels: a
 * click that moves a little is still a click. */
export const DRAG_THRESHOLD = 6;

/** Where a pinned program would go, among the pinned programs, if the pointer
 * were let go at `x`: the number of the other programs whose centre is before
 * it. `centers` are the centres of the programs at rest, in the dock's order,
 * and `from` is the one being dragged. */
export function dropIndex(
  centers: readonly number[],
  from: number,
  x: number,
): number {
  let before = 0;
  centers.forEach((center, index) => {
    if (index !== from && center < x) before += 1;
  });
  return before;
}

/** Where the mark of the drop stands: before the program at `index`, after
 * the one at `index`, or nowhere (the program would stay where it is). */
export interface DropMark {
  index: number;
  side: "before" | "after";
}

export function dropMark(
  count: number,
  from: number,
  target: number,
): DropMark | null {
  if (target === from) return null;
  // The programs other than the dragged one, in order: the drop goes before
  // the one at `target` among them, or after the last.
  const others = Array.from({ length: count }, (_, index) => index).filter(
    (index) => index !== from,
  );
  const next = others[target];
  if (next !== undefined) return { index: next, side: "before" };
  const last = others[others.length - 1];
  return last === undefined ? null : { index: last, side: "after" };
}

/** Keeps the dragged tile inside the bar: `x` is the pointer, `start` where
 * the drag began, and the tile can go from `min` to `max` (its own left edge
 * at rest, in pixels, plus what it moves). */
export function dragOffset(
  x: number,
  start: number,
  restLeft: number,
  width: number,
  min: number,
  max: number,
): number {
  const wanted = x - start;
  return Math.min(Math.max(wanted, min - restLeft), max - width - restLeft);
}
