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
import ChoiceCards from "./parts/ChoiceCards";
import "./StyleGallery.css";

/** How many items a miniature shows. */
export const MINI_ITEMS = 5;

/** The bar of a miniature is this much of the real dock's (a 48 px tile
 * becomes 18 px): the glows shrink with it. */
const MINI_SCALE = 18 / 48;

/** Half the height of the miniature's bar, in px (StyleGallery.css): its
 * corners at 100 % are a pill. */
const MINI_HALF_HEIGHT = 14;

export type GalleryDock = Pick<
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
 * picture: decorative, the card's name says which style it is. `icons`,
 * when given, are the pictures of the programs (an icon pack's), in place of
 * the ones the dock shows now. */
export function MiniDock({
  of: style,
  dock,
  appIds,
  icons,
}: {
  of: DockStyle;
  dock: GalleryDock | null;
  appIds: readonly string[];
  icons?: readonly (string | null)[];
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
          ? shown.map((appId, index) =>
              icons === undefined ? (
                <MiniApp
                  key={appId}
                  appId={appId}
                  variant={dock?.iconPack ?? ""}
                />
              ) : icons[index] ? (
                <img
                  key={appId}
                  className="mini-app"
                  src={icons[index]}
                  alt=""
                  draggable={false}
                />
              ) : (
                <span key={appId} className="mini-app" />
              ),
            )
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
 * dock drawn in that style (`ChoiceCards`).
 */
function StyleGallery({
  dock,
  appIds,
  label,
  disabled = false,
  onSelect,
}: StyleGalleryProps) {
  const t = useT();
  return (
    <ChoiceCards
      label={label}
      choices={DOCK_STYLES.map((style) => ({
        value: style,
        label: t.settings.look.styles[style],
      }))}
      selected={dock?.style ?? null}
      disabled={disabled}
      onSelect={onSelect}
      thumb={(style) => <MiniDock of={style} dock={dock} appIds={appIds} />}
      wide
    />
  );
}

export default StyleGallery;
