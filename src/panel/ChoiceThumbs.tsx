import type {
  DockEdge,
  HoverEffect,
  IconShape,
  Indicator,
  OpenAnimation,
  TaskbarMode,
} from "../core/settings";
import type { ReactNode } from "react";
import { MiniTile } from "./DockItems";
import "./ChoiceThumbs.css";

/* The pictures of the visual choices of Configurações (spec 0014 R2), for
 * `ChoiceCards`. Decorative: each card's name says which choice it is. The
 * chosen card shows its effect moving, unless the user asked for less
 * motion (ChoiceThumbs.css). */

const TILES = ["clock", "note", "time-progress"] as const;

/** A row of three tiles on a plain strip, the middle one marked. */
function Row({
  children,
  ...data
}: {
  children?: ReactNode;
  [key: `data-${string}`]: string | undefined;
}) {
  return (
    <span className="thumb thumb-row" aria-hidden="true" {...data}>
      <span className="thumb-strip">
        {TILES.map((kind, index) => (
          <span
            key={kind}
            className="thumb-slot"
            data-middle={index === 1 ? "" : undefined}
          >
            <MiniTile kind={kind} small />
          </span>
        ))}
        {children}
      </span>
    </span>
  );
}

/** A screen with the dock on an edge and the Windows taskbar as it is. */
export function ScreenThumb({
  edge = "bottom",
  taskbar = "visible",
}: {
  edge?: DockEdge;
  taskbar?: TaskbarMode;
}) {
  return (
    <span
      className="thumb thumb-screen"
      aria-hidden="true"
      data-edge={edge}
      data-taskbar={taskbar}
    >
      <span className="thumb-desktop">
        <span className="thumb-dock">
          {TILES.map((kind) => (
            <MiniTile key={kind} kind={kind} small />
          ))}
        </span>
      </span>
      <span className="thumb-taskbar" />
    </span>
  );
}

/** What the item under the pointer does: grows alone, or with its
 * neighbors. */
export function HoverThumb({
  effect,
  chosen,
}: {
  effect: HoverEffect;
  chosen: boolean;
}) {
  return (
    <Row data-hover={effect} data-chosen={chosen ? "" : undefined}>
      <span className="thumb-pointer" />
    </Row>
  );
}

/** The mark under the open programs. */
export function IndicatorThumb({ shape }: { shape: Indicator }) {
  return (
    <Row data-indicator={shape}>
      {TILES.map((kind, index) => (
        <span
          key={kind}
          className="thumb-mark"
          data-shape={shape}
          data-at={index}
        />
      ))}
    </Row>
  );
}

/** How the icons of the programs are evened out. */
export function ShapeThumb({ shape }: { shape: IconShape }) {
  return (
    <span className="thumb thumb-row" aria-hidden="true" data-shape={shape}>
      <span className="thumb-strip">
        {(["round", "square", "wide"] as const).map((form) => (
          <span key={form} className="thumb-icon-slot">
            <span className="thumb-icon" data-form={form} />
          </span>
        ))}
      </span>
    </span>
  );
}

/** How an icon moves while its program opens. */
export function OpenThumb({
  animation,
  chosen,
}: {
  animation: OpenAnimation;
  chosen: boolean;
}) {
  return <Row data-open={animation} data-chosen={chosen ? "" : undefined} />;
}
