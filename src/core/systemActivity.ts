import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorKind, log } from "./log";

/* What the system widgets' panels show (spec 0017 F1). Must match
 * `Reading` in `src-tauri/src/domain/system_activity.rs`; the dock reads
 * the same sampler on its own timer. */

/** The memory, in bytes. */
export interface Memory {
  total: number;
  available: number;
}

/** A reading of the processor, the memory and the network. */
export interface SystemReading {
  /** The processor's use, 0–100; null until two readings were made. */
  cpu: number | null;
  memory: Memory | null;
  /** Bytes per second. */
  down: number;
  up: number;
  /** The last minute, oldest first, one value every 2 s. */
  cpuHistory: number[];
  downHistory: number[];
  upHistory: number[];
}

export const READ_SYSTEM_ACTIVITY_COMMAND = "read_system_activity";

/** How often an open panel reads again, as the dock does. */
export const SYSTEM_ACTIVITY_EVERY_MS = 2_000;

export function readSystemActivity(): Promise<SystemReading> {
  return invoke<SystemReading>(READ_SYSTEM_ACTIVITY_COMMAND);
}

/** The reading, again every 2 s while the component is mounted; null
 * until the first answer, and while the reading fails. */
export function useSystemActivity(): SystemReading | null {
  const [reading, setReading] = useState<SystemReading | null>(null);

  useEffect(() => {
    let active = true;
    let warned = false;
    const read = () => {
      readSystemActivity().then(
        (next) => {
          if (active) setReading(next);
        },
        (error: unknown) => {
          // Once per panel: a failure every 2 s would flood the log.
          if (!warned) log.warn(`system: read failed: ${errorKind(error)}`);
          warned = true;
          if (active) setReading(null);
        },
      );
    };
    read();
    const timer = window.setInterval(read, SYSTEM_ACTIVITY_EVERY_MS);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, []);

  return reading;
}

/** A number with one decimal below ten and none above, with a comma. */
function oneDecimal(value: number): string {
  return value < 10
    ? value.toFixed(1).replace(".", ",")
    : String(Math.round(value));
}

/** A rate, the Brazilian way, as the dock writes it: `340 KB/s`,
 * `1,2 MB/s` (`rate_text` in Rust). */
export function rateText(bytesPerSecond: number): string {
  const kb = Math.max(bytesPerSecond, 0) / 1024;
  if (kb < 1000) return `${Math.round(kb)} KB/s`;
  const mb = kb / 1024;
  if (mb < 1000) return `${oneDecimal(mb)} MB/s`;
  return `${oneDecimal(mb / 1024)} GB/s`;
}

/** A size of memory: `7,8 GB`, `512 MB`. */
export function sizeText(bytes: number): string {
  const gb = bytes / 1024 ** 3;
  return gb >= 1
    ? `${oneDecimal(gb)} GB`
    : `${Math.round(bytes / 1024 ** 2)} MB`;
}

/** The share of the memory in use, 0–100; null with no total. */
export function memoryPercent(memory: Memory | null): number | null {
  if (!memory || memory.total <= 0) return null;
  return ((memory.total - memory.available) / memory.total) * 100;
}
