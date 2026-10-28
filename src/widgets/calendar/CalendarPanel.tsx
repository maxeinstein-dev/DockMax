import { useLayoutEffect, useRef, useState } from "react";
import {
  clockText,
  dayOf,
  openCalendarLink,
  plainText,
  useCalendar,
  type CalendarEvent,
} from "../../core/calendar";
import { format, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "./calendar.css";

/** A calendar's color dot. The color goes through the CSSOM, never a
 * `style` attribute (strict CSP). Decorative. */
function Dot({ color }: { color: string | null }) {
  const dot = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    if (color && /^#[0-9a-f]{6}$/i.test(color)) {
      dot.current?.style.setProperty("--dot", color);
    }
  }, [color]);
  return <span ref={dot} className="cal-dot" aria-hidden="true" />;
}

/** The event's subview: when, where, the calendar, the description, and
 * Entrar and Abrir no Google Agenda. */
function Details({
  event,
  offset,
  day,
  onBack,
}: {
  event: CalendarEvent;
  offset: number;
  /** Hoje or Amanhã. */
  day: string;
  onBack: () => void;
}) {
  const t = useT();
  const text = t.widgets.calendar;
  const title = event.title || text.noTitle;
  return (
    <>
      <PanelHeader title={title} onBack={onBack} focusAction />
      <dl className="cal-details">
        <dt>{text.when}</dt>
        <dd>
          {day} ·{" "}
          {event.allDay
            ? text.allDay
            : format(text.range, {
                start: clockText(event.start, offset),
                end: clockText(event.end, offset),
              })}
        </dd>
        {event.location ? (
          <>
            <dt>{text.where}</dt>
            <dd>{event.location}</dd>
          </>
        ) : null}
        <dt>{text.calendar}</dt>
        <dd>
          <Dot color={event.color} /> {event.calendarName}
        </dd>
      </dl>
      {event.description ? (
        <p className="cal-description">{plainText(event.description)}</p>
      ) : null}
      <div className="cal-actions">
        {event.link ? (
          <button
            type="button"
            className="quiet-button"
            onClick={() => void openCalendarLink(event.link ?? "")}
          >
            {text.openInGoogle}
          </button>
        ) : null}
        {event.join ? (
          <button
            type="button"
            className="secondary-button"
            onClick={() => void openCalendarLink(event.join ?? "")}
          >
            {text.join}
          </button>
        ) : null}
      </div>
    </>
  );
}

/**
 * The main view of a Calendário panel (spec 0017 F2a): today's and
 * tomorrow's events of the calendars it shows, the all-day ones first, each
 * meeting with Entrar; a click on one opens its details. The events are the
 * dock's reading, asked again while the panel is open.
 */
function CalendarPanel({
  instance,
  now: clock,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"calendar">) {
  const t = useT();
  const text = t.widgets.calendar;
  const view = useCalendar();
  const [open, setOpen] = useState<string | null>(null);

  const snapshot = view?.snapshot ?? null;
  // The panel's clock ticks every second; the events are the dock's.
  const now = Math.floor(clock.getTime() / 1000);
  const offset = snapshot?.offset ?? 0;
  const shown = (snapshot?.events ?? []).filter(
    (event) => !instance.config.hidden.includes(event.calendar),
  );

  const today = dayOf(now, offset);
  const selected = shown.find((event) => event.id === open);
  if (selected) {
    return (
      <Details
        event={selected}
        offset={offset}
        day={dayOf(selected.start, offset) > today ? text.tomorrow : text.today}
        onBack={() => setOpen(null)}
      />
    );
  }

  const days = [
    { name: text.today, day: today },
    { name: text.tomorrow, day: today + 1 },
  ];
  const notice =
    snapshot && snapshot.status !== "ok" ? text.status[snapshot.status] : null;

  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      {notice ? <p className="cal-notice">{notice}</p> : null}
      {snapshot === null ? (
        <p className="cal-empty">{text.loading}</p>
      ) : (
        days.map(({ name, day }) => {
          const events = shown.filter(
            (event) =>
              dayOf(event.start, offset) <= day &&
              (event.allDay
                ? dayOf(event.end - 1, offset) >= day
                : dayOf(event.start, offset) === day) &&
              (event.allDay || event.end > now || day > today),
          );
          return (
            <section key={name} className="cal-day" aria-label={name}>
              <h3 className="cal-day-name">{name}</h3>
              {events.length === 0 ? (
                <p className="cal-empty">{text.free}</p>
              ) : (
                <ul className="cal-list">
                  {events.map((event) => {
                    const title = event.title || text.noTitle;
                    const under = !event.allDay && event.start <= now;
                    return (
                      <li
                        key={event.id + event.calendar}
                        className={event.allDay ? "all-day" : undefined}
                      >
                        <button
                          type="button"
                          className="cal-event"
                          onClick={() => setOpen(event.id)}
                        >
                          <Dot color={event.color} />
                          <span className="cal-time">
                            {event.allDay
                              ? text.allDay
                              : under
                                ? text.now
                                : clockText(event.start, offset)}
                          </span>
                          <span className="cal-title">{title}</span>
                        </button>
                        {event.join && !event.allDay ? (
                          <button
                            type="button"
                            className="secondary-button cal-join"
                            aria-label={format(text.joinNamed, { title })}
                            onClick={() =>
                              void openCalendarLink(event.join ?? "")
                            }
                          >
                            {text.join}
                          </button>
                        ) : null}
                      </li>
                    );
                  })}
                </ul>
              )}
            </section>
          );
        })
      )}
      <PanelFooter>{text.footer}</PanelFooter>
    </>
  );
}

export default CalendarPanel;
