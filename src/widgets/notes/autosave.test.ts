import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  AUTOSAVE_DELAY_MS,
  createAutosave,
  SAVE_TIMEOUT_MS,
  type Schedule,
} from "./autosave";

/** A save the test settles by hand, to hold one "in flight". */
function manualSave() {
  const calls: {
    text: string;
    resolve: () => void;
    reject: (error: unknown) => void;
  }[] = [];
  const save = vi.fn(
    (text: string) =>
      new Promise<void>((resolve, reject) => {
        calls.push({ text, resolve, reject });
      }),
  );
  return { save, calls };
}

/** Whether `promise` has settled, without waiting for it. */
function tracked(promise: Promise<unknown>) {
  const state = { settled: false };
  void promise.finally(() => {
    state.settled = true;
  });
  return state;
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("waiting for a pause", () => {
  it("saves 500 ms after the last change, by default", async () => {
    const save = vi.fn(async () => {});
    const autosave = createAutosave({ save });
    expect(AUTOSAVE_DELAY_MS).toBe(500);

    autosave.change("a");
    expect(autosave.getStatus()).toEqual({ state: "pending", error: null });
    await vi.advanceTimersByTimeAsync(499);
    expect(save).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(save).toHaveBeenCalledExactlyOnceWith("a");
    expect(autosave.getStatus()).toEqual({ state: "idle", error: null });
  });

  it("restarts the wait on every keystroke and saves only the last text", async () => {
    const save = vi.fn(async () => {});
    const autosave = createAutosave({ save });

    for (const text of ["o", "oi", "oi!"]) {
      autosave.change(text);
      await vi.advanceTimersByTimeAsync(400);
    }
    expect(save).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(100);
    expect(save).toHaveBeenCalledExactlyOnceWith("oi!");
  });

  it("uses the injected delay and timers", async () => {
    const pending: (() => void)[] = [];
    const schedule: Schedule = {
      setTimeout: vi.fn((callback: () => void) => pending.push(callback)),
      clearTimeout: vi.fn(),
    };
    const save = vi.fn(async () => {});
    const autosave = createAutosave({ save, delayMs: 1200, schedule });

    autosave.change("a");
    autosave.change("ab");
    expect(schedule.setTimeout).toHaveBeenLastCalledWith(
      expect.any(Function),
      1200,
    );
    expect(schedule.clearTimeout).toHaveBeenCalledWith(1);
    pending[pending.length - 1]!();
    await vi.runAllTimersAsync();
    expect(save).toHaveBeenCalledExactlyOnceWith("ab");
  });
});

describe("saving nothing new", () => {
  it("doesn't save the text already stored", async () => {
    const save = vi.fn(async () => {});
    const autosave = createAutosave({ save, savedText: "nota" });

    autosave.change("nota!");
    autosave.change("nota");
    expect(autosave.getStatus().state).toBe("idle");
    await vi.advanceTimersByTimeAsync(1000);
    await autosave.flush();
    expect(save).not.toHaveBeenCalled();
  });

  it("doesn't save the same text twice", async () => {
    const save = vi.fn(async () => {});
    const autosave = createAutosave({ save });

    autosave.change("a");
    await vi.advanceTimersByTimeAsync(500);
    autosave.change("a");
    await vi.advanceTimersByTimeAsync(500);
    await autosave.flush();
    expect(save).toHaveBeenCalledOnce();
  });
});

describe("flush", () => {
  it("saves the pending text at once and drops the timer", async () => {
    const save = vi.fn(async () => {});
    const autosave = createAutosave({ save });

    autosave.change("a");
    await autosave.flush();
    expect(save).toHaveBeenCalledExactlyOnceWith("a");
    expect(autosave.getStatus().state).toBe("idle");
    await vi.advanceTimersByTimeAsync(1000);
    expect(save).toHaveBeenCalledOnce();
  });

  it("does nothing when everything is saved", async () => {
    const save = vi.fn(async () => {});
    const autosave = createAutosave({ save });
    await autosave.flush();
    expect(save).not.toHaveBeenCalled();
  });

  it("waits for the running save, then saves what came after", async () => {
    const { save, calls } = manualSave();
    const autosave = createAutosave({ save });

    autosave.change("a");
    await vi.advanceTimersByTimeAsync(500);
    autosave.change("ab");
    const flushed = autosave.flush();
    expect(save).toHaveBeenCalledOnce(); // never two at once
    calls[0]!.resolve();
    await vi.advanceTimersByTimeAsync(0);
    expect(calls).toHaveLength(2);
    expect(calls[1]!.text).toBe("ab");
    calls[1]!.resolve();
    await flushed;
    expect(autosave.getStatus()).toEqual({ state: "idle", error: null });
  });
});

describe("a change while saving", () => {
  it("saves again after the running save, never alongside it", async () => {
    const { save, calls } = manualSave();
    const autosave = createAutosave({ save });

    autosave.change("a");
    await vi.advanceTimersByTimeAsync(500);
    expect(autosave.getStatus().state).toBe("saving");

    autosave.change("ab");
    // The pause ends while "a" is still being saved.
    await vi.advanceTimersByTimeAsync(500);
    expect(save).toHaveBeenCalledOnce();
    expect(autosave.getStatus().state).toBe("saving");

    calls[0]!.resolve();
    await vi.advanceTimersByTimeAsync(0);
    expect(save).toHaveBeenCalledTimes(2);
    expect(calls[1]!.text).toBe("ab");
    calls[1]!.resolve();
    await vi.advanceTimersByTimeAsync(0);
    expect(autosave.getStatus().state).toBe("idle");
  });

  it("lets a pause still running finish before saving again", async () => {
    const { save, calls } = manualSave();
    const autosave = createAutosave({ save });

    autosave.change("a");
    await vi.advanceTimersByTimeAsync(500);
    autosave.change("ab");
    calls[0]!.resolve();
    await vi.advanceTimersByTimeAsync(0);
    expect(autosave.getStatus().state).toBe("pending");
    expect(save).toHaveBeenCalledOnce();

    await vi.advanceTimersByTimeAsync(500);
    expect(calls[1]!.text).toBe("ab");
  });

  it("doesn't save again when the text went back to the one being saved", async () => {
    const { save, calls } = manualSave();
    const autosave = createAutosave({ save });

    autosave.change("a");
    await vi.advanceTimersByTimeAsync(500);
    autosave.change("ab");
    autosave.change("a");
    await vi.advanceTimersByTimeAsync(500);
    calls[0]!.resolve();
    await vi.advanceTimersByTimeAsync(1000);
    expect(save).toHaveBeenCalledOnce();
    expect(autosave.getStatus().state).toBe("idle");
  });
});

describe("errors", () => {
  it("reports the code, keeps the text unsaved and doesn't loop", async () => {
    const save = vi.fn(async (text: string) => {
      if (text.length > 3) throw { code: "noteTooLarge" };
    });
    const autosave = createAutosave({ save });

    autosave.change("long");
    await vi.advanceTimersByTimeAsync(500);
    const status = autosave.getStatus();
    expect(status).toEqual({ state: "error", error: "noteTooLarge" });
    // Only the code reaches the UI, never the note's text.
    expect(JSON.stringify(status)).not.toContain("long");
    await vi.advanceTimersByTimeAsync(5000);
    expect(save).toHaveBeenCalledOnce();
  });

  it("keeps the message while typing, and clears it on a good save", async () => {
    const save = vi.fn(async (text: string) => {
      if (text.length > 3) throw { code: "noteTooLarge" };
    });
    const autosave = createAutosave({ save });

    autosave.change("long");
    await vi.advanceTimersByTimeAsync(500);
    autosave.change("lon");
    expect(autosave.getStatus()).toEqual({
      state: "pending",
      error: "noteTooLarge",
    });
    await vi.advanceTimersByTimeAsync(500);
    expect(save).toHaveBeenLastCalledWith("lon");
    expect(autosave.getStatus()).toEqual({ state: "idle", error: null });
  });

  it("retries a failed text on flush, once", async () => {
    const save = vi
      .fn<(text: string) => Promise<void>>()
      .mockRejectedValueOnce({ code: "writeFailed" })
      .mockResolvedValue(undefined);
    const autosave = createAutosave({ save });

    autosave.change("a");
    await vi.advanceTimersByTimeAsync(500);
    expect(autosave.getStatus()).toEqual({
      state: "error",
      error: "writeFailed",
    });
    await autosave.flush();
    expect(save).toHaveBeenCalledTimes(2);
    expect(autosave.getStatus()).toEqual({ state: "idle", error: null });
  });

  it("flush resolves even when the save fails", async () => {
    const save = vi.fn(async () => {
      throw { code: "unavailable" };
    });
    const autosave = createAutosave({ save });

    autosave.change("a");
    await expect(autosave.flush()).resolves.toBeUndefined();
    expect(save).toHaveBeenCalledOnce();
    expect(autosave.getStatus()).toEqual({
      state: "error",
      error: "unavailable",
    });
  });

  it("reads a code that isn't a widget error code as writeFailed", async () => {
    const save = vi.fn(async () => {
      throw { code: "boom" };
    });
    const autosave = createAutosave({ save });

    autosave.change("a");
    await autosave.flush();
    expect(autosave.getStatus()).toEqual({
      state: "error",
      error: "writeFailed",
    });
  });

  it("reads anything that isn't a command error as writeFailed", async () => {
    const save = vi.fn(() => {
      throw new Error("the note's text could be in here");
    });
    const autosave = createAutosave({ save });

    autosave.change("a");
    await autosave.flush();
    expect(autosave.getStatus()).toEqual({
      state: "error",
      error: "writeFailed",
    });
  });
});

describe("a save that never ends", () => {
  it("gives up after 10 s by default: error, and the next change retries", async () => {
    const { save, calls } = manualSave();
    const autosave = createAutosave({ save });
    expect(SAVE_TIMEOUT_MS).toBe(10_000);

    autosave.change("a");
    await vi.advanceTimersByTimeAsync(500);
    await vi.advanceTimersByTimeAsync(SAVE_TIMEOUT_MS - 1);
    expect(autosave.getStatus().state).toBe("saving");
    await vi.advanceTimersByTimeAsync(1);
    expect(autosave.getStatus()).toEqual({
      state: "error",
      error: "writeFailed",
    });
    // Not retried on its own.
    await vi.advanceTimersByTimeAsync(60_000);
    expect(save).toHaveBeenCalledOnce();

    autosave.change("ab");
    await vi.advanceTimersByTimeAsync(500);
    expect(calls[1]!.text).toBe("ab");
    calls[1]!.resolve();
    await vi.advanceTimersByTimeAsync(0);
    expect(autosave.getStatus()).toEqual({ state: "idle", error: null });
  });

  it("uses the injected deadline", async () => {
    const { save } = manualSave();
    const autosave = createAutosave({ save, saveTimeoutMs: 2000 });

    autosave.change("a");
    await vi.advanceTimersByTimeAsync(500 + 1999);
    expect(autosave.getStatus().state).toBe("saving");
    await vi.advanceTimersByTimeAsync(1);
    expect(autosave.getStatus().state).toBe("error");
  });

  it("lets flush resolve at the deadline, and the next flush retries", async () => {
    const { save, calls } = manualSave();
    const autosave = createAutosave({ save });

    autosave.change("a");
    const first = tracked(autosave.flush());
    await vi.advanceTimersByTimeAsync(SAVE_TIMEOUT_MS - 1);
    expect(first.settled).toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    expect(first.settled).toBe(true);
    expect(autosave.getStatus()).toEqual({
      state: "error",
      error: "writeFailed",
    });

    const second = autosave.flush();
    expect(calls).toHaveLength(2);
    expect(calls[1]!.text).toBe("a");
    calls[1]!.resolve();
    await second;
    expect(autosave.getStatus()).toEqual({ state: "idle", error: null });
  });

  it("ignores a save that ends after its deadline", async () => {
    const { save, calls } = manualSave();
    const autosave = createAutosave({ save });
    const seen: string[] = [];
    autosave.subscribe(() => seen.push(autosave.getStatus().state));

    autosave.change("a");
    await vi.advanceTimersByTimeAsync(500 + SAVE_TIMEOUT_MS);
    autosave.change("ab");
    await vi.advanceTimersByTimeAsync(500);
    expect(autosave.getStatus().state).toBe("saving");
    const before = seen.length;

    // The late "a" neither ends the "ab" save nor starts another one.
    calls[0]!.resolve();
    await vi.advanceTimersByTimeAsync(0);
    expect(seen).toHaveLength(before);
    expect(autosave.getStatus().state).toBe("saving");
    expect(save).toHaveBeenCalledTimes(2);

    calls[1]!.resolve();
    await vi.advanceTimersByTimeAsync(0);
    expect(autosave.getStatus()).toEqual({ state: "idle", error: null });

    // Nor does a late failure: it neither changes the status nor saves.
    autosave.change("abc");
    await vi.advanceTimersByTimeAsync(500 + SAVE_TIMEOUT_MS);
    expect(autosave.getStatus()).toEqual({
      state: "error",
      error: "writeFailed",
    });
    autosave.change("ab");
    expect(autosave.getStatus()).toEqual({
      state: "pending",
      error: "writeFailed",
    });
    const beforeLateFailure = seen.length;
    calls[2]!.reject({ code: "unavailable" });
    await vi.advanceTimersByTimeAsync(0);
    expect(seen).toHaveLength(beforeLateFailure);
    expect(save).toHaveBeenCalledTimes(3);

    // The pause saves "ab": the store may hold the late "abc".
    await vi.advanceTimersByTimeAsync(500);
    expect(save).toHaveBeenCalledTimes(4);
    expect(calls[3]!.text).toBe("ab");
    calls[3]!.resolve();
    await vi.advanceTimersByTimeAsync(0);
    expect(autosave.getStatus()).toEqual({ state: "idle", error: null });
  });

  it("saves again after a deadline, even the text stored before it", async () => {
    // The store writes at once (the Rust commands are synchronous); only the
    // reply for "abc" comes after the deadline.
    let stored = "ab";
    const replies: (() => void)[] = [];
    const save = vi.fn(
      (text: string) =>
        new Promise<void>((resolve) => {
          stored = text;
          replies.push(resolve);
        }),
    );
    const autosave = createAutosave({ save, savedText: "ab" });

    autosave.change("abc");
    await vi.advanceTimersByTimeAsync(500 + SAVE_TIMEOUT_MS);
    expect(stored).toBe("abc");
    expect(autosave.getStatus()).toEqual({
      state: "error",
      error: "writeFailed",
    });

    autosave.change("ab");
    const flushed = autosave.flush();
    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith("ab");
    replies[1]!();
    await flushed;
    replies[0]!();
    await vi.advanceTimersByTimeAsync(0);

    expect(stored).toBe("ab");
    expect(autosave.getStatus()).toEqual({ state: "idle", error: null });
  });

  it("concurrent flushes: one save at a time, and the last text is saved", async () => {
    const { save, calls } = manualSave();
    const autosave = createAutosave({ save });
    let running = 0;
    let most = 0;
    save.mockImplementation(
      (text: string) =>
        new Promise<void>((resolve, reject) => {
          running += 1;
          most = Math.max(most, running);
          const end = () => {
            running -= 1;
          };
          calls.push({
            text,
            resolve: () => {
              end();
              resolve();
            },
            reject: (error) => {
              end();
              reject(error);
            },
          });
        }),
    );

    autosave.change("a");
    const first = tracked(autosave.flush());
    autosave.change("ab");
    const second = tracked(autosave.flush());
    autosave.change("abc");
    const third = tracked(autosave.flush());
    expect(save).toHaveBeenCalledOnce();

    calls[0]!.resolve();
    await vi.advanceTimersByTimeAsync(0);
    expect(calls).toHaveLength(2);
    expect(calls[1]!.text).toBe("abc");
    expect([first, second, third].some((f) => f.settled)).toBe(false);

    calls[1]!.resolve();
    await vi.advanceTimersByTimeAsync(0);
    expect([first, second, third].every((f) => f.settled)).toBe(true);
    expect(most).toBe(1);
    expect(save).toHaveBeenLastCalledWith("abc");
    expect(autosave.getStatus()).toEqual({ state: "idle", error: null });
  });

  it("concurrent flushes over a hung save: both resolve, then one retry", async () => {
    const { save, calls } = manualSave();
    const autosave = createAutosave({ save });

    autosave.change("a");
    const first = tracked(autosave.flush());
    autosave.change("ab");
    const second = tracked(autosave.flush());
    await vi.advanceTimersByTimeAsync(SAVE_TIMEOUT_MS);
    // "ab" arrived during the hung save: it is saved at once, alone.
    expect(calls).toHaveLength(2);
    expect(calls[1]!.text).toBe("ab");
    expect(first.settled || second.settled).toBe(false);

    calls[1]!.resolve();
    await vi.advanceTimersByTimeAsync(0);
    expect(first.settled && second.settled).toBe(true);
    expect(autosave.getStatus()).toEqual({ state: "idle", error: null });
  });
});

describe("status and disposal", () => {
  it("notifies each change of status, with a stable object in between", async () => {
    const save = vi.fn(async () => {});
    const autosave = createAutosave({ save });
    const seen: string[] = [];
    const unsubscribe = autosave.subscribe(() =>
      seen.push(autosave.getStatus().state),
    );

    autosave.change("a");
    const pending = autosave.getStatus();
    autosave.change("ab");
    expect(autosave.getStatus()).toBe(pending);
    await vi.advanceTimersByTimeAsync(500);
    expect(seen).toEqual(["pending", "saving", "idle"]);

    unsubscribe();
    autosave.change("abc");
    expect(seen).toHaveLength(3);
  });

  it("stops saving after dispose", async () => {
    const save = vi.fn(async () => {});
    const autosave = createAutosave({ save });

    autosave.change("a");
    autosave.dispose();
    await vi.advanceTimersByTimeAsync(1000);
    autosave.change("ab");
    await autosave.flush();
    expect(save).not.toHaveBeenCalled();
  });

  it("dispose during a save: flush resolves, the late result is ignored", async () => {
    const { save, calls } = manualSave();
    const autosave = createAutosave({ save });
    const listener = vi.fn();
    autosave.subscribe(listener);

    autosave.change("a");
    const flushed = tracked(autosave.flush());
    autosave.change("ab");
    const status = autosave.getStatus();
    listener.mockClear();

    autosave.dispose();
    await vi.advanceTimersByTimeAsync(0);
    expect(flushed.settled).toBe(true);
    expect(vi.getTimerCount()).toBe(0);

    calls[0]!.resolve();
    await vi.advanceTimersByTimeAsync(SAVE_TIMEOUT_MS);
    expect(save).toHaveBeenCalledOnce();
    expect(listener).not.toHaveBeenCalled();
    expect(autosave.getStatus()).toBe(status);
  });
});
