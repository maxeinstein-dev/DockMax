import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AUTOSAVE_DELAY_MS, createAutosave, type Schedule } from "./autosave";

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
});
