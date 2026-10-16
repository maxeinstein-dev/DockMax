import type { WidgetTileProps } from "../contract";
import { useT } from "../../i18n";
import { progressOf } from "./progress";

/** PROVISIONAL (W2 replaces it): the live face of a progress dock icon,
 * the first period's percentage without the ring. Decorative: the item's
 * `aria-label` is the label or the kind's name (spec 0003 §2.3 item 4). */
function ProgressTile({ instance, now }: WidgetTileProps<"time-progress">) {
  const t = useT();
  const first = instance.config.periods[0];
  const progress = first ? progressOf(first, now, instance.config) : null;
  return (
    <span className="tile-pct" aria-hidden="true">
      {progress?.state === "running"
        ? `${Math.floor(progress.fraction * 100)}%`
        : t.widgets.progress.noValue}
    </span>
  );
}

export default ProgressTile;
