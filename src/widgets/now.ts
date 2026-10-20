import { useEffect, useRef, useState } from "react";
import { msUntilNextMinute } from "./clock/format";

/* The hosts' timers for the widgets' `now` (contract.ts): one in the dock,
 * at each minute turn, for every tile; one in an open widget panel, at each
 * second. A timer alone trusts the time it was set for, so a clock set by
 * hand or a wake from sleep would leave the faces on a stale minute until
 * the next turn. So the timer never sleeps longer than `RESYNC_MS`, and each
 * wake (and the page becoming visible again) compares the unit shown with
 * the unit of the system clock: a change of the time shows within
 * `RESYNC_MS` at most, and a wake that finds nothing new renders nothing. */

export const MINUTE_MS = 60_000;
export const SECOND_MS = 1_000;

/** Longest the timer sleeps: the bound on how stale a face can get after
 * the system clock changes. */
export const RESYNC_MS = 15_000;

/** Milliseconds from `nowMs` to the next turn of `unitMs`; a whole unit when
 * `nowMs` is on a turn (that unit is already shown). */
export function msUntilNext(unitMs: number, nowMs: number): number {
  if (unitMs === MINUTE_MS) return msUntilNextMinute(nowMs);
  return unitMs - (((nowMs % unitMs) + unitMs) % unitMs);
}

/** When the timer wakes next: at the next turn, or after `RESYNC_MS` if
 * that comes first. */
export function nextWakeMs(unitMs: number, nowMs: number): number {
  return Math.min(msUntilNext(unitMs, nowMs), RESYNC_MS);
}

/** Which unit `ms` falls in (minutes or seconds since 1970). */
function unitOf(unitMs: number, ms: number): number {
  return Math.floor(ms / unitMs);
}

/**
 * The current instant, updated when the minute (`MINUTE_MS`) or the second
 * (`SECOND_MS`) of the system clock changes: at each turn, after a change of
 * the system clock (within `RESYNC_MS`) and as soon as the page becomes
 * visible again (after sleep, for one). A single timer per call.
 */
export function useNow(unitMs: number): Date {
  const [now, setNow] = useState(() => new Date());
  const shown = useRef(now);

  useEffect(() => {
    let handle: ReturnType<typeof setTimeout> | undefined;
    const wake = () => {
      clearTimeout(handle);
      const ms = Date.now();
      if (unitOf(unitMs, ms) !== unitOf(unitMs, shown.current.getTime())) {
        const next = new Date(ms);
        shown.current = next;
        setNow(next);
      }
      handle = setTimeout(wake, nextWakeMs(unitMs, ms));
    };
    const onVisibility = () => {
      if (document.visibilityState === "visible") wake();
    };
    handle = setTimeout(wake, nextWakeMs(unitMs, Date.now()));
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      clearTimeout(handle);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [unitMs]);

  return now;
}
