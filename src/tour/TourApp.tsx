import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { errorKind, log } from "../core/log";
import { useSettings } from "../core/settings";
import { finishTour } from "../core/tour";
import { format, useT } from "../i18n";
import "./tour.css";

/** The steps, in order: each its glaze (tokens.css) and glyph (24 px box). */
const STEPS = [
  {
    key: "welcome",
    glaze: "t-about",
    paths: [
      "M4 17.5h16",
      "M6.5 14h3v3.5h-3zM10.5 12h3v5.5h-3zM14.5 14h3v3.5h-3z",
    ],
  },
  {
    key: "apps",
    glaze: "t-app",
    paths: ["M5 5h5v5H5zM14 5h5v5h-5zM5 14h5v5H5z", "M16.5 13.5v6M13.5 16.5h6"],
  },
  {
    key: "workspaces",
    glaze: "t-network",
    paths: [
      "M5 12a1.5 1.5 0 1 1 0 0.01M12 12a1.5 1.5 0 1 1 0 0.01M19 12a1.5 1.5 0 1 1 0 0.01",
    ],
  },
  {
    key: "widgets",
    glaze: "t-weather",
    paths: [
      "M12 8a4 4 0 1 1 0 8a4 4 0 1 1 0 -8",
      "M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4 -1.4M17.3 6.7l1.4 -1.4",
    ],
  },
  {
    key: "launcher",
    glaze: "t-countdown",
    paths: [
      "M10.5 4a6.5 6.5 0 1 1 0 13a6.5 6.5 0 1 1 0 -13",
      "M15.5 15.5L20 20",
    ],
  },
  {
    key: "done",
    glaze: "t-todo",
    paths: ["M5 12.5l4.5 4.5L19 7.5"],
  },
] as const;

/**
 * The welcome tour (spec 0019 G2): one step at a time, with Back, Next
 * and Skip; the last one closes it or opens Configurações. Arrow keys move
 * between steps, Esc skips.
 */
function TourApp() {
  const t = useT();
  const text = t.tour;
  const settings = useSettings();
  const [index, setIndex] = useState(0);
  const heading = useRef<HTMLHeadingElement>(null);
  // `index` stays within the steps: Next and Back stop at the ends.
  const step = STEPS[index] ?? STEPS[0];
  const last = index === STEPS.length - 1;
  const copy = text.steps[step.key];

  // Each step is read from its title.
  useEffect(() => {
    heading.current?.focus();
  }, [index]);

  function finish(settingsToo: boolean) {
    finishTour(settingsToo).catch((error: unknown) => {
      log.warn(`tour: finish failed: ${errorKind(error)}`);
    });
  }

  function onKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (event.key === "ArrowRight" && !last) setIndex(index + 1);
    else if (event.key === "ArrowLeft" && index > 0) setIndex(index - 1);
    else if (event.key === "Escape") finish(false);
  }

  return (
    <main className="tour" onKeyDown={onKeyDown}>
      <span className={`about-tile tour-tile ${step.glaze}`} aria-hidden="true">
        <svg viewBox="0 0 24 24">
          {step.paths.map((path) => (
            <path key={path} d={path} />
          ))}
        </svg>
      </span>
      <p className="tour-count">
        {format(text.count, { n: index + 1, total: STEPS.length })}
      </p>
      <h1 className="tour-title" ref={heading} tabIndex={-1}>
        {format(copy.title, {
          hotkey: settings?.launcherHotkey ?? "Alt+Space",
        })}
      </h1>
      <p className="tour-body">
        {format(copy.body, {
          hotkey: settings?.launcherHotkey ?? "Alt+Space",
        })}
      </p>
      <ol className="tour-dots" aria-hidden="true">
        {STEPS.map((each, at) => (
          <li key={each.key} data-current={at === index || undefined} />
        ))}
      </ol>
      <div className="tour-actions">
        {last ? (
          <>
            <button
              type="button"
              className="secondary-button"
              onClick={() => finish(true)}
            >
              {text.openSettings}
            </button>
            <button
              type="button"
              className="secondary-button tour-main"
              onClick={() => finish(false)}
            >
              {text.start}
            </button>
          </>
        ) : (
          <>
            <button
              type="button"
              className="quiet-button"
              onClick={() => finish(false)}
            >
              {text.skip}
            </button>
            <span className="tour-spacer" />
            {index > 0 ? (
              <button
                type="button"
                className="secondary-button"
                onClick={() => setIndex(index - 1)}
              >
                {text.back}
              </button>
            ) : null}
            <button
              type="button"
              className="secondary-button tour-main"
              onClick={() => setIndex(index + 1)}
            >
              {text.next}
            </button>
          </>
        )}
      </div>
    </main>
  );
}

export default TourApp;
