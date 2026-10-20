import type { ClockConfig } from "../../core/widgets";
import type { WidgetTileProps } from "../contract";
import "./clock.css";
import { formatDockTime, type Instant } from "./format";

/** The four marks of the dial, at 12, 3, 6 and 9 (24 px viewBox). */
const MARKS = [
  [12, 2.6],
  [21.4, 12],
  [12, 21.4],
  [2.6, 12],
] as const;

const HOUR_HAND = 5;
const MINUTE_HAND = 7.6;

/** Hours (0–23) and minutes at `now` in the clock's zone, read from the
 * same formatting as the digital face, so both styles agree. */
function hoursAndMinutes(now: Instant, config: ClockConfig) {
  const [hours = 0, minutes = 0] = formatDockTime(now, {
    ...config,
    hourCycle: "h23",
  })
    .split(":")
    .map(Number);
  return { hours, minutes };
}

/** The end of a hand of `length` from the center at `degrees` clockwise
 * from 12, rounded for the attributes. */
function handEnd(degrees: number, length: number) {
  const radians = (degrees * Math.PI) / 180;
  return {
    x2: (12 + length * Math.sin(radians)).toFixed(2),
    y2: (12 - length * Math.cos(radians)).toFixed(2),
  };
}

/** The analog face (DESIGN.md "Ícones vivos": Relógio analógico): white
 * hands on the glaze, no numbers and no second hand. */
function Dial({ now, config }: { now: Instant; config: ClockConfig }) {
  const { hours, minutes } = hoursAndMinutes(now, config);
  return (
    <svg className="clock-tile-dial" viewBox="0 0 24 24" aria-hidden="true">
      {MARKS.map(([cx, cy]) => (
        <circle
          key={`${cx},${cy}`}
          className="clock-mark"
          cx={cx}
          cy={cy}
          r={0.75}
        />
      ))}
      <line
        className="clock-hand-hour"
        x1={12}
        y1={12}
        {...handEnd(((hours % 12) + minutes / 60) * 30, HOUR_HAND)}
      />
      <line
        className="clock-hand-minute"
        x1={12}
        y1={12}
        {...handEnd(minutes * 6, MINUTE_HAND)}
      />
      <circle className="clock-axis" cx={12} cy={12} r={1.25} />
    </svg>
  );
}

/** The live face of a clock's dock icon, digital or analog by its
 * `iconStyle`, sized by `--icon`. Decorative: the item's `aria-label` is
 * the label or the kind's name, never the time (spec 0003 §2.3 item 4). */
function ClockTile({ instance, now }: WidgetTileProps<"clock">) {
  const { config } = instance;
  if (config.iconStyle === "analog") return <Dial now={now} config={config} />;
  return (
    <span className="clock-tile-time" aria-hidden="true">
      {formatDockTime(now, config)}
    </span>
  );
}

export default ClockTile;
