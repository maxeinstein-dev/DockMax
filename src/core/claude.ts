import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { activeLocale } from "../i18n";
import { errorKind, log } from "./log";

/* The Uso do Claude's panel (spec 0017 F5): the plan's limits, read in Rust
 * (`windows/claude_usage.rs`) as the Claude Code's `/usage` reads them.
 * Must match `Reading` there. */

export interface ClaudeLimit {
  /** 0–100. */
  percent: number;
  /** Unix seconds; null when Anthropic didn't say. */
  resetsAt: number | null;
}

export interface ClaudeUsage {
  fiveHour: ClaudeLimit | null;
  sevenDay: ClaudeLimit | null;
}

export type ClaudeReading =
  | { state: "ready"; usage: ClaudeUsage; at: number }
  | { state: "signedOut" }
  | { state: "expired" }
  | { state: "unavailable" };

/** How often an open panel asks for the last reading. */
export const CLAUDE_EVERY_MS = 30_000;

/** The last reading (null before the first); `fresh` reads again now. */
export function claudeUsage(fresh: boolean): Promise<ClaudeReading | null> {
  return invoke<ClaudeReading | null>("claude_usage", { fresh });
}

/** Opens the Claude Code's sign-in in a terminal. */
export function claudeSignIn(): Promise<void> {
  return invoke("claude_sign_in");
}

/** The last reading, asked again while mounted; the first ask reads anew,
 * and so does every one while the Claude Code is signed out or expired
 * (the sign-in may be on its way). */
export function useClaudeUsage(): ClaudeReading | null | undefined {
  const [reading, setReading] = useState<ClaudeReading | null | undefined>(
    undefined,
  );
  useEffect(() => {
    let active = true;
    let waiting = false;
    const ask = (fresh: boolean) => {
      claudeUsage(fresh || waiting).then(
        (next) => {
          waiting = next?.state === "signedOut" || next?.state === "expired";
          if (active) setReading(next);
        },
        (error: unknown) => {
          log.warn(`claude usage: read failed: ${errorKind(error)}`);
        },
      );
    };
    ask(true);
    // The fresh reading lands a moment after: asked again soon, then
    // every half minute.
    const soon = window.setTimeout(() => ask(false), 2_500);
    const every = window.setInterval(() => ask(false), CLAUDE_EVERY_MS);
    return () => {
      active = false;
      window.clearTimeout(soon);
      window.clearInterval(every);
    };
  }, []);
  return reading;
}

/** When a limit starts over, from `now` (Unix seconds): `em 2 h 14 min`
 * within the day, otherwise the weekday and the hour (`sex., 09:00`). */
export function resetText(
  resetsAt: number,
  now: number,
  words: { within: string; hours: string; minutes: string; soon: string },
): string {
  const left = resetsAt - now;
  if (left <= 60) return words.soon;
  if (left < 24 * 3600) {
    const hours = Math.floor(left / 3600);
    const minutes = Math.floor((left % 3600) / 60);
    const parts = [
      hours > 0 ? words.hours.replace("{n}", String(hours)) : null,
      minutes > 0 ? words.minutes.replace("{n}", String(minutes)) : null,
    ].filter(Boolean);
    return words.within.replace("{time}", parts.join(" "));
  }
  return new Intl.DateTimeFormat(activeLocale(), {
    weekday: "short",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(resetsAt * 1000));
}
