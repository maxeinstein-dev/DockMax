import { useT } from "../../i18n";
import type { WidgetTileProps } from "../contract";
import { progressOf } from "./progress";
import "./time-progress.css";

/** Ring radius in the 24-unit view box (DESIGN.md "Ladrilho vivo"). */
const RADIUS = 9;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

/** The live face of a progress dock icon: the first period's ring, starting
 * at the top, with the percentage rounded down in the center. The Pequeno
 * size hides the number (time-progress.css). Without a running period (the
 * workday off), the ring is empty and the center shows a dash.
 *
 * Decorative: the item's `aria-label` is the label or the kind's name, and
 * the panel has the values (spec 0003 §2.3 item 4). */
function ProgressTile({ instance, now }: WidgetTileProps<"time-progress">) {
  const t = useT();
  const first = instance.config.periods[0];
  const progress = first ? progressOf(first, now, instance.config) : null;
  const fraction = progress?.state === "running" ? progress.fraction : null;
  return (
    <span className="progress-tile" aria-hidden="true">
      <svg className="progress-ring" viewBox="0 0 24 24">
        <circle className="progress-ring-track" cx="12" cy="12" r={RADIUS} />
        {fraction === null ? null : (
          <circle
            className="progress-ring-arc"
            cx="12"
            cy="12"
            r={RADIUS}
            strokeDasharray={`${(fraction * CIRCUMFERENCE).toFixed(2)} ${CIRCUMFERENCE.toFixed(2)}`}
            transform="rotate(-90 12 12)"
          />
        )}
      </svg>
      <span className="progress-tile-pct">
        {fraction === null ? (
          t.widgets.progress.noValue
        ) : (
          <>
            {Math.floor(fraction * 100)}
            <small>%</small>
          </>
        )}
      </span>
    </span>
  );
}

export default ProgressTile;
