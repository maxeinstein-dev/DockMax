import type { TimeProgressConfig, WidgetInstance } from "../../core/widgets";

/* Test helpers of the time-progress components. */

/** Friday 2026-10-16 at 12:00, local time: half the day, a third of the
 * 09:00–18:00 workday. */
export const FRIDAY_NOON = new Date(2026, 9, 16, 12, 0);

/** Saturday 2026-10-17 at 12:00: the workday is off. */
export const SATURDAY_NOON = new Date(2026, 9, 17, 12, 0);

/** A time-progress instance, with the default config unless given. */
export function progressInstance(
  config: Partial<TimeProgressConfig> = {},
  label: string | null = null,
): WidgetInstance<"time-progress"> {
  return {
    id: "w-progress",
    label,
    kind: "time-progress",
    config: {
      periods: ["day", "week", "month", "year"],
      weekStart: "monday",
      workday: { start: "09:00", end: "18:00" },
      ...config,
    },
  };
}
