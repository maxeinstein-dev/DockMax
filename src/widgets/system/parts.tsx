import { useLayoutEffect, useRef } from "react";
import { WIDGET_WIDTHS, type WidgetWidth } from "../../core/widgets";
import { useT } from "../../i18n";
import OptionGroup from "../../panel/OptionGroup";
import "./system.css";

/** A share as a bar, 0–1. Its share goes through the CSSOM on `--f`, never
 * a `style` attribute (strict CSP). Decorative: the figure next to it says
 * the same. */
export function MeterBar({ fraction }: { fraction: number | null }) {
  const fill = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    const share = Math.min(Math.max(fraction ?? 0, 0), 1);
    fill.current?.style.setProperty("--f", share.toFixed(4));
  }, [fraction]);
  return (
    <span className="sys-bar" aria-hidden="true">
      <span ref={fill} className="sys-fill" />
    </span>
  );
}

/** The last minute as a line, newest on the right, scaled to `peak`; the
 * area under it filled. Decorative: the figures say the values. */
export function Sparkline({
  values,
  peak,
  slots,
}: {
  values: readonly number[];
  peak: number;
  /** How many values a full minute has: a short history starts at the
   * right. */
  slots: number;
}) {
  const width = 100;
  const height = 28;
  const step = width / Math.max(slots - 1, 1);
  const start = width - step * (values.length - 1);
  const points = values.map((value, index) => {
    const y = height - (Math.min(value, peak) / (peak || 1)) * (height - 2);
    return `${(start + step * index).toFixed(2)},${y.toFixed(2)}`;
  });
  return (
    <svg
      className="sys-spark"
      viewBox={`0 0 ${width} ${height}`}
      preserveAspectRatio="none"
      aria-hidden="true"
    >
      {values.length > 1 ? (
        <>
          <polygon
            className="sys-spark-area"
            points={`${start.toFixed(2)},${height} ${points.join(" ")} ${width},${height}`}
          />
          <polyline className="sys-spark-line" points={points.join(" ")} />
        </>
      ) : null}
    </svg>
  );
}

/** The tile's width, 1 or 2 icons. */
export function WidthGroup<W extends WidgetWidth = WidgetWidth>({
  width,
  widths = WIDGET_WIDTHS as unknown as readonly W[],
  onSelect,
}: {
  width: W;
  /** The widths the kind takes; all of them by default. */
  widths?: readonly W[];
  onSelect: (width: W) => void;
}) {
  const t = useT();
  const text = t.widgets.system;
  const names: Record<WidgetWidth, string> = {
    1: text.widths.one,
    2: text.widths.two,
    3: text.widths.three,
  };
  return (
    <OptionGroup
      label={text.width}
      options={widths.map((value) => ({
        value: String(value),
        label: names[value],
      }))}
      selected={String(width)}
      onSelect={(value) => onSelect(Number(value) as W)}
    />
  );
}
