import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorKind, log } from "./log";

/* The Calendário's events (spec 0017 F2a), as the dock read them from the
 * connected Google accounts. Must match `src-tauri/src/windows/calendar.rs`
 * and `src-tauri/src/domain/calendar.rs`. Only the panel reads them. */

export type CalendarStatus = "ok" | "noAccounts" | "offline" | "signInAgain";

/** A calendar of the accounts. */
export interface CalendarSource {
  account: string;
  id: string;
  name: string;
  /** `#rrggbb`. */
  color: string | null;
}

/** An event of today or tomorrow. */
export interface CalendarEvent {
  id: string;
  calendar: string;
  calendarName: string;
  color: string | null;
  title: string;
  /** Unix seconds; an all-day event starts at local midnight. */
  start: number;
  end: number;
  allDay: boolean;
  location: string | null;
  description: string | null;
  /** The meeting's address (Meet, Teams or Zoom). */
  join: string | null;
  /** The event in Google Agenda. */
  link: string | null;
  /** Shown only as busy, with no title (a free/busy calendar). */
  busy: boolean;
}

/** A Google Tasks task due today or late, not done (F2a-3). */
export interface CalendarTask {
  account: string;
  list: string;
  listName: string;
  id: string;
  title: string;
  notes: string | null;
  /** `2026-10-28`. */
  due: string;
  /** Due before today. */
  late: boolean;
  link: string | null;
}

export interface CalendarSnapshot {
  status: CalendarStatus;
  events: CalendarEvent[];
  sources: CalendarSource[];
  tasks: CalendarTask[];
  /** Seconds from UTC of the local time. */
  offset: number;
}

export interface CalendarView {
  /** null until the first reading. */
  snapshot: CalendarSnapshot | null;
  /** Unix seconds when it was asked. */
  now: number;
}

/** How often an open panel asks again. */
export const CALENDAR_EVERY_MS = 30_000;

export function calendarEvents(fresh: boolean): Promise<CalendarView> {
  return invoke<CalendarView>("calendar_events", { fresh });
}

/** Opens a meeting or the event's page in Google Agenda. */
export function openCalendarLink(url: string): Promise<void> {
  return invoke<void>("open_calendar_link", { url }).catch((error: unknown) => {
    log.warn(`calendar: a link was not opened: ${errorKind(error)}`);
  });
}

/** Ticks off a task of the last reading; rejects when Google didn't
 * take it. */
export function completeTask(task: CalendarTask): Promise<void> {
  return invoke<void>("complete_calendar_task", {
    account: task.account,
    list: task.list,
    id: task.id,
  });
}

/** The events, asked again every 30 s while mounted; the first ask makes
 * the dock read them again, and a second one follows a moment later. */
export function useCalendar(): CalendarView | null {
  const [view, setView] = useState<CalendarView | null>(null);
  useEffect(() => {
    let active = true;
    const ask = (fresh: boolean) => {
      calendarEvents(fresh).then(
        (next) => {
          if (active) setView(next);
        },
        (error: unknown) => {
          log.warn(`calendar: read failed: ${errorKind(error)}`);
        },
      );
    };
    ask(true);
    const soon = window.setTimeout(() => ask(false), 3_000);
    const every = window.setInterval(() => ask(false), CALENDAR_EVERY_MS);
    return () => {
      active = false;
      window.clearTimeout(soon);
      window.clearInterval(every);
    };
  }, []);
  return view;
}

const DAY = 24 * 3600;

/** `14:30`: the local time of `at`. */
export function clockText(at: number, offset: number): string {
  const minutes = Math.floor(((((at + offset) % DAY) + DAY) % DAY) / 60);
  const hh = String(Math.floor(minutes / 60)).padStart(2, "0");
  const mm = String(minutes % 60).padStart(2, "0");
  return `${hh}:${mm}`;
}

/** The local day of `at`, as a count of days. */
export function dayOf(at: number, offset: number): number {
  return Math.floor((at + offset) / DAY);
}

/** An event's description as plain text: Google sends some as HTML. */
export function plainText(description: string): string {
  return description
    .replace(/<br\s*\/?>/gi, "\n")
    .replace(/<\/p>/gi, "\n")
    .replace(/<[^>]*>/g, "")
    .replace(/&nbsp;/g, " ")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, "&")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}
