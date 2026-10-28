import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorKind, log } from "./log";
import type { CycleConfig } from "./widgets";

/* The Ciclo's panel (spec 0017 F2b): where the cycle is, computed in Rust
 * from the marks in her calendar (`domain/cycle.rs`); the marks never
 * reach the page. Must match `CycleView` in `commands/calendar.rs`. */

export type CyclePhase =
  | { phase: "unknown" }
  | { phase: "period"; day: number }
  | { phase: "sensitive"; in: number }
  | { phase: "between"; in: number };

export interface CycleStatus {
  phase: CyclePhase;
  averages: { cycle: number; period: number; cycles: number };
  /** Today's day of the cycle; 0 without marks. */
  day: number;
  /** Local day numbers (days since 1970-01-01). */
  last: number | null;
  next: number | null;
  marks: number;
}

export interface CycleView {
  /** null until the marks were read. */
  status: CycleStatus | null;
  today: number;
  accounts: boolean;
}

/** How often an open panel asks again. */
export const CYCLE_EVERY_MS = 60_000;

export function cycleStatus(config: CycleConfig): Promise<CycleView> {
  return invoke<CycleView>("cycle_status", {
    symbol: config.symbol,
    calendar: config.calendar,
    sensitive: config.sensitive,
  });
}

/** The cycle of `config`, asked again every minute while mounted. */
export function useCycle(config: CycleConfig): CycleView | null {
  const [view, setView] = useState<CycleView | null>(null);
  const key = `${config.symbol}|${config.calendar ?? ""}|${config.sensitive}`;
  useEffect(() => {
    let active = true;
    const ask = () => {
      cycleStatus(config).then(
        (next) => {
          if (active) setView(next);
        },
        (error: unknown) => {
          log.warn(`cycle: read failed: ${errorKind(error)}`);
        },
      );
    };
    ask();
    const every = window.setInterval(ask, CYCLE_EVERY_MS);
    return () => {
      active = false;
      window.clearInterval(every);
    };
    // The config is the key: a new object with the same values asks nothing.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return view;
}

/** `09/11`: the date of a local day number. */
export function dayText(day: number): string {
  const date = new Date(day * 24 * 3600 * 1000);
  const dd = String(date.getUTCDate()).padStart(2, "0");
  const mm = String(date.getUTCMonth() + 1).padStart(2, "0");
  return `${dd}/${mm}`;
}
