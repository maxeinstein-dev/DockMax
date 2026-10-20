import { act, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { msUntilNextMinute } from "./clock/format";
import {
  MINUTE_MS,
  RESYNC_MS,
  SECOND_MS,
  msUntilNext,
  nextWakeMs,
  useNow,
} from "./now";

// 2026-10-16 12:00:40.250 UTC.
const START = Date.UTC(2026, 9, 16, 12, 0, 40, 250);

describe("msUntilNext", () => {
  it("is msUntilNextMinute for minutes", () => {
    for (const ms of [START, START + 19_750, -1, 0]) {
      expect(msUntilNext(MINUTE_MS, ms)).toBe(msUntilNextMinute(ms));
    }
  });

  it("lands on the next second", () => {
    expect(msUntilNext(SECOND_MS, START)).toBe(750);
    expect(msUntilNext(SECOND_MS, START + 750)).toBe(1000);
    expect(msUntilNext(SECOND_MS, -1)).toBe(1);
  });
});

describe("nextWakeMs", () => {
  it("wakes at the turn when it is near", () => {
    expect(nextWakeMs(MINUTE_MS, START + 10_000)).toBe(9_750);
    expect(nextWakeMs(SECOND_MS, START)).toBe(750);
  });

  it("never sleeps longer than RESYNC_MS", () => {
    expect(nextWakeMs(MINUTE_MS, START - 30_000)).toBe(RESYNC_MS);
  });
});

/** Renders a probe that records each `now` it gets. */
function renderProbe(unitMs: number) {
  const seen: number[] = [];
  function Probe() {
    const now = useNow(unitMs);
    seen.push(now.getTime());
    return null;
  }
  const view = render(<Probe />);
  return { ...view, seen, last: () => seen[seen.length - 1] };
}

describe("useNow", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(START);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("updates at the minute turn, not before", async () => {
    const { seen, last } = renderProbe(MINUTE_MS);
    const renders = seen.length;
    expect(last()).toBe(START);

    await act(() => vi.advanceTimersByTimeAsync(19_749));
    expect(seen).toHaveLength(renders);

    await act(() => vi.advanceTimersByTimeAsync(1));
    expect(last()).toBe(Date.UTC(2026, 9, 16, 12, 1, 0));
  });

  it("follows a change of the system clock within RESYNC_MS", async () => {
    const { last } = renderProbe(MINUTE_MS);
    await act(() => vi.advanceTimersByTimeAsync(19_750));

    // The clock is set 2 h 30 min ahead, mid-minute.
    const jumped = Date.UTC(2026, 9, 16, 14, 31, 0, 0) + 100;
    vi.setSystemTime(jumped);
    await act(() => vi.advanceTimersByTimeAsync(RESYNC_MS));

    expect(last()).toBeGreaterThanOrEqual(jumped);
    expect(Math.floor(last()! / MINUTE_MS)).toBe(
      Math.floor(jumped / MINUTE_MS),
    );
  });

  it("follows the system clock set back within RESYNC_MS", async () => {
    const { last } = renderProbe(MINUTE_MS);
    await act(() => vi.advanceTimersByTimeAsync(19_750));
    expect(last()).toBe(Date.UTC(2026, 9, 16, 12, 1, 0));

    // The clock is set 3 h back, mid-minute: a face that only moved
    // forward would stay on 12:01.
    const earlier = Date.UTC(2026, 9, 16, 9, 1, 0, 0) + 100;
    vi.setSystemTime(earlier);
    await act(() => vi.advanceTimersByTimeAsync(RESYNC_MS));

    expect(last()).toBeGreaterThanOrEqual(earlier);
    expect(Math.floor(last()! / MINUTE_MS)).toBe(
      Math.floor(earlier / MINUTE_MS),
    );
  });

  it("checks again as soon as the page becomes visible", async () => {
    const { last } = renderProbe(MINUTE_MS);
    // Back from sleep: the clock moved on, the timer did not fire.
    const later = START + 3 * MINUTE_MS;
    vi.setSystemTime(later);

    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });

    expect(last()).toBe(later);
  });

  it("renders nothing on a wake that finds the same minute", async () => {
    vi.setSystemTime(START - 30_000);
    const { seen } = renderProbe(MINUTE_MS);
    const renders = seen.length;

    await act(() => vi.advanceTimersByTimeAsync(RESYNC_MS));

    expect(seen).toHaveLength(renders);
  });

  it("ticks every second for a panel", async () => {
    const { last } = renderProbe(SECOND_MS);
    await act(() => vi.advanceTimersByTimeAsync(750));
    expect(last()).toBe(START + 750);
    await act(() => vi.advanceTimersByTimeAsync(1000));
    expect(last()).toBe(START + 1750);
  });

  it("stops its timer and listener when unmounted", () => {
    const remove = vi.spyOn(document, "removeEventListener");
    const { unmount } = renderProbe(MINUTE_MS);

    unmount();

    expect(vi.getTimerCount()).toBe(0);
    expect(remove).toHaveBeenCalledWith("visibilitychange", expect.anything());
  });
});
