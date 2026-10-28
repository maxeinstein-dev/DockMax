import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorKind, log } from "./log";
import type { PomodoroConfig } from "./widgets";

/* The Pomodoro's panel (spec 0017 F2c-2). The timer runs in Rust
 * (`windows/pomodoro.rs`), so it keeps counting with the panel closed; the
 * panel asks for it every second while open. Must match `View` there. */

export type PomodoroStage = "focus" | "short" | "long";

export type PomodoroAction = "start" | "pause" | "skip" | "reset";

export interface PomodoroView {
  stage: PomodoroStage;
  /** Milliseconds left of the stage, and its whole length. */
  left: number;
  total: number;
  running: boolean;
  /** Focuses done since the last long pause, of `every`. */
  focuses: number;
  every: number;
}

/** How often an open panel asks again. */
export const POMODORO_EVERY_MS = 1_000;

export function pomodoroStatus(
  widget: string,
  config: PomodoroConfig,
): Promise<PomodoroView> {
  return invoke<PomodoroView>("pomodoro_status", { widget, config });
}

export function pomodoroAct(
  widget: string,
  config: PomodoroConfig,
  action: PomodoroAction,
): Promise<PomodoroView> {
  return invoke<PomodoroView>("pomodoro_act", { widget, config, action });
}

/** `24:05`: milliseconds left as minutes and seconds, rounded up. */
export function clockOf(left: number): string {
  const seconds = Math.max(0, Math.ceil(left / 1000));
  const minutes = Math.floor(seconds / 60);
  return `${String(minutes).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

/** The timer of the widget `widget`, asked every second while mounted, and
 * a way to act on it. */
export function usePomodoro(
  widget: string,
  config: PomodoroConfig,
): [PomodoroView | null, (action: PomodoroAction) => void] {
  const [view, setView] = useState<PomodoroView | null>(null);
  const key = JSON.stringify(config);
  useEffect(() => {
    let active = true;
    const ask = () => {
      pomodoroStatus(widget, config).then(
        (next) => {
          if (active) setView(next);
        },
        (error: unknown) => {
          log.warn(`pomodoro: read failed: ${errorKind(error)}`);
        },
      );
    };
    ask();
    const every = window.setInterval(ask, POMODORO_EVERY_MS);
    return () => {
      active = false;
      window.clearInterval(every);
    };
    // The config is the key: a new object with the same values asks nothing.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [widget, key]);
  const act = (action: PomodoroAction) => {
    pomodoroAct(widget, config, action).then(setView, (error: unknown) => {
      log.warn(`pomodoro: an action failed: ${errorKind(error)}`);
    });
  };
  return [view, act];
}
