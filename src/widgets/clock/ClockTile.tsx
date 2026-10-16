import type { WidgetTileProps } from "../contract";
import { formatDockTime } from "./format";

/** PROVISIONAL (W1 replaces it): the live face of a clock's dock icon,
 * digital only. Decorative: the item's `aria-label` is the label or the
 * kind's name, never the time (spec 0003 §2.3 item 4). */
function ClockTile({ instance, now }: WidgetTileProps<"clock">) {
  return (
    <span className="tile-time" aria-hidden="true">
      {formatDockTime(now, instance.config)}
    </span>
  );
}

export default ClockTile;
