import { errorKind, log } from "../../core/log";
import { mediaControl, useMedia, type MediaAction } from "../../core/media";
import { useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "./media.css";

/**
 * The main view of a Tocando agora panel (spec 0017 F4): the cover, the
 * title, the artist and the program, with the previous, play or pause and
 * the next. Windows decides what plays; the panel only asks it.
 */
function MediaPanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"media">) {
  const t = useT();
  const text = t.widgets.media;
  const view = useMedia();
  const act = (action: MediaAction) => {
    mediaControl(action).catch((error: unknown) => {
      log.warn(`media: an action failed: ${errorKind(error)}`);
    });
  };
  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      {view === undefined ? (
        <p className="media-note">{text.loading}</p>
      ) : view === null ? (
        <p className="media-note">{text.nothing}</p>
      ) : (
        <>
          <div className="media-cover">
            {view.art ? (
              <img src={view.art} alt="" />
            ) : (
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M9 17.5V5.5l10 -2v12M9 17.5a2.5 2.5 0 1 1 -5 0a2.5 2.5 0 1 1 5 0M19 15.5a2.5 2.5 0 1 1 -5 0a2.5 2.5 0 1 1 5 0" />
              </svg>
            )}
          </div>
          <p className="media-title">{view.title || text.untitled}</p>
          {view.artist ? <p className="media-artist">{view.artist}</p> : null}
          <div className="media-actions">
            <button
              type="button"
              className="secondary-button"
              disabled={!view.canPrevious}
              onClick={() => act("previous")}
            >
              {text.previous}
            </button>
            <button
              type="button"
              className="secondary-button media-main"
              disabled={!view.canToggle}
              onClick={() => act("toggle")}
            >
              {view.playing ? text.pause : text.play}
            </button>
            <button
              type="button"
              className="secondary-button"
              disabled={!view.canNext}
              onClick={() => act("next")}
            >
              {text.next}
            </button>
          </div>
        </>
      )}
      <PanelFooter>
        {view ? (
          <span className="media-source">
            {view.icon ? <img src={view.icon} alt="" /> : null}
            {`${text.from} ${view.app}`}
          </span>
        ) : (
          text.footer
        )}
      </PanelFooter>
    </>
  );
}

export default MediaPanel;
