/* The clock and the date at the end of the dock, which stand in for the
 * taskbar's while it is hidden (ADR 0009). */

/** What the clock shows: the time and the date, in the system's language
 * unless `locale` says one. 24 hours, as the taskbar's clock shows by
 * default in Brazil. */
export function clockText(
  now: Date,
  locale?: string,
): { time: string; date: string } {
  return {
    time: new Intl.DateTimeFormat(locale, {
      hour: "2-digit",
      minute: "2-digit",
      hourCycle: "h23",
    }).format(now),
    date: new Intl.DateTimeFormat(locale, {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
    }).format(now),
  };
}
