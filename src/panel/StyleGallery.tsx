import { useId, useRef, useState, type KeyboardEvent } from "react";
import { ICON_SIZES, useAppIcon } from "../core/apps";
import {
  DOCK_STYLES,
  type DockSettings,
  type DockStyle,
} from "../core/settings";
import { styleLookOf } from "../dock/look";
import {
  STYLE_PAINTS,
  css,
  fillCss,
  glowCss,
  outlineCss,
  shelfCss,
} from "../dock/paints";
import { useT } from "../i18n";
import { MiniTile } from "./DockItems";
import "./StyleGallery.css";

/** How many items a miniature shows. */
const MINI_ITEMS = 5;

/** The bar of a miniature is this much of the real dock's (a 48 px tile
 * becomes 18 px): the glows shrink with it. */
const MINI_SCALE = 18 / 48;

/** Half the height of the miniature's bar, in px (StyleGallery.css): its
 * corners at 100 % are a pill. */
const MINI_HALF_HEIGHT = 14;

type GalleryDock = Pick<
  DockSettings,
  "style" | "tweaks" | "indicator" | "indicatorColor" | "iconPack"
>;

type StyleGalleryProps = {
  /** The dock's settings shown, or null while they can't be read. */
  dock: GalleryDock | null;
  /** The programs pinned in the workspace in use, in dock order: the
   * miniatures show the first ones. Without any, widget tiles. */
  appIds: readonly string[];
  label: string;
  disabled?: boolean;
  onSelect: (style: DockStyle) => void;
};

/** A program's icon in a miniature, the letter-less plate while it loads. */
function MiniApp({ appId, variant }: { appId: string; variant: string }) {
  const icon = useAppIcon(appId, ICON_SIZES.list, variant);
  return icon === null ? (
    <span className="mini-app" />
  ) : (
    <img className="mini-app" src={icon} alt="" draggable={false} />
  );
}

/** The dock in `style`, with the user's changes to that style, as a
 * picture: decorative, the card's name says which style it is. */
function MiniDock({
  of: style,
  dock,
  appIds,
}: {
  of: DockStyle;
  dock: GalleryDock | null;
  appIds: readonly string[];
}) {
  const paint = STYLE_PAINTS[style];
  const look = styleLookOf(style, dock?.tweaks[style]);
  const outline = look.border ? outlineCss(paint) : null;
  const indicator =
    dock?.indicatorColor ??
    (paint.indicator === null ? null : css(paint.indicator));
  const shown = appIds.slice(0, MINI_ITEMS);

  return (
    <span className="mini-wall" aria-hidden="true">
      <span
        className="mini-bar"
        data-border={look.border && outline === null ? "" : undefined}
        data-outline={outline === null ? undefined : ""}
        data-flat={paint.flat ? "" : undefined}
        data-reflections={look.reflections ? "" : undefined}
        // The style's paint, through the CSSOM (no inline style).
        ref={(bar) => {
          if (!bar) return;
          const set = (name: string, value: string | null) => {
            if (value === null) bar.style.removeProperty(name);
            else bar.style.setProperty(name, value);
          };
          set("--mini-radius", `${(look.corners / 100) * MINI_HALF_HEIGHT}px`);
          set("--mini-fill", fillCss(paint));
          set("--mini-glow", glowCss(paint, MINI_SCALE));
          set("--mini-outline", outline);
          set(
            "--mini-hi",
            paint.hi === "none" ? "transparent" : paint.hi && css(paint.hi),
          );
          set("--mini-indicator", indicator);
          set("--mini-shelf", paint.shelf ? shelfCss() : null);
        }}
      >
        {shown.length > 0
          ? shown.map((appId) => (
              <MiniApp
                key={appId}
                appId={appId}
                variant={dock?.iconPack ?? ""}
              />
            ))
          : (["clock", "note", "time-progress"] as const).map((kind) => (
              <MiniTile key={kind} kind={kind} small />
            ))}
        <span
          className="mini-indicator"
          data-shape={dock?.indicator ?? "dot"}
        />
      </span>
    </span>
  );
}

/**
 * The style of the dock as a grid of cards (spec 0014 R1), each with the
 * dock drawn in that style: a labelled group of toggle buttons, the chosen
 * one `aria-pressed`. One Tab stop (roving tabindex, at the chosen card);
 * the arrows, Home and End move inside it, and choosing applies at once.
 */
function StyleGallery({
  dock,
  appIds,
  label,
  disabled = false,
  onSelect,
}: StyleGalleryProps) {
  const t = useT();
  const labelId = useId();
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const [focused, setFocused] = useState<number | null>(null);
  const selected = dock ? DOCK_STYLES.indexOf(dock.style) : -1;
  const tabStop = focused ?? Math.max(selected, 0);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const last = DOCK_STYLES.length - 1;
    const from = buttons.current.findIndex(
      (button) => button === document.activeElement,
    );
    const current = from >= 0 ? from : tabStop;
    const next = {
      ArrowRight: current === last ? 0 : current + 1,
      ArrowDown: current === last ? 0 : current + 1,
      ArrowLeft: current === 0 ? last : current - 1,
      ArrowUp: current === 0 ? last : current - 1,
      Home: 0,
      End: last,
    }[event.key];
    if (next === undefined) return;
    event.preventDefault();
    setFocused(next);
    buttons.current[next]?.focus();
  };

  return (
    <div className="style-gallery-field">
      <span id={labelId} className="option-label">
        {label}
      </span>
      <div
        className="style-gallery"
        role="group"
        aria-labelledby={labelId}
        onKeyDown={onKeyDown}
        onBlur={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget)) {
            setFocused(null);
          }
        }}
      >
        {DOCK_STYLES.map((style, index) => (
          <button
            key={style}
            ref={(button) => {
              buttons.current[index] = button;
            }}
            type="button"
            className="style-card"
            aria-pressed={index === selected}
            tabIndex={index === tabStop ? 0 : -1}
            disabled={disabled}
            onFocus={() => setFocused(index)}
            onClick={() => {
              setFocused(index);
              if (index !== selected) onSelect(style);
            }}
          >
            <MiniDock of={style} dock={dock} appIds={appIds} />
            <span className="style-card-name">
              {t.settings.look.styles[style]}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}

export default StyleGallery;
