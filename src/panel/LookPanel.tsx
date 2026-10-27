import { useId, useRef, useState, type KeyboardEvent } from "react";
import {
  INDICATOR_COLORS,
  MAX_CORNERS,
  MAX_HOVER_SCALE,
  MIN_CORNERS,
  MIN_HOVER_SCALE,
  type DockSettings,
  type HoverEffect,
  type IconShape,
  type Indicator,
  type OpenAnimation,
  type Settings,
  type StyleTweaks,
} from "../core/settings";
import { dockLookOf } from "../dock/look";
import { useT } from "../i18n";
import IconPacksField from "./IconPacksField";
import OptionGroup, { type Option } from "./OptionGroup";
import PanelFooter from "./PanelFooter";
import PanelHeader from "./parts/PanelHeader";
import RangeField from "./parts/RangeField";
import ChoiceCards from "./parts/ChoiceCards";
import SettingsGroup from "./parts/SettingsGroup";
import {
  HoverThumb,
  IndicatorThumb,
  OpenThumb,
  ShapeThumb,
} from "./ChoiceThumbs";
import ToggleField from "./parts/ToggleField";
import StyleGallery from "./StyleGallery";
import { useSettingsEditor } from "./useSettingsEditor";
import "./Look.css";

const HOVERS: readonly HoverEffect[] = ["none", "scale", "magnify"];
const INDICATORS: readonly Indicator[] = ["dot", "dash"];
const ICON_SHAPES: readonly IconShape[] = ["plate", "uniform", "original"];
const OPEN_ANIMATIONS: readonly OpenAnimation[] = ["bounce", "pulse", "shake"];

function options<V extends string>(
  values: readonly V[],
  labels: Readonly<Record<V, string>>,
): Option<V>[] {
  return values.map((value) => ({ value, label: labels[value] }));
}

/** The color chosen among the ones offered; a color written by hand in the
 * file matches by its `#RRGGBB`, in any case. */
function chosenColor(color: string | null): string | null {
  if (color === null) return null;
  const upper = color.toUpperCase();
  return INDICATOR_COLORS.find((offered) => offered === upper) ?? null;
}

/** The colors of the indicator as a group of round swatches: the style's
 * own first, then the palette. One Tab stop (the chosen swatch, else the
 * first); the arrows, Home and End move inside it. */
function SwatchGroup({
  label,
  defaultLabel,
  colorName,
  color,
  disabled,
  onSelect,
}: {
  label: string;
  defaultLabel: string;
  colorName: (color: string) => string;
  color: string | null;
  disabled: boolean;
  onSelect: (color: string | null) => void;
}) {
  const labelId = useId();
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const [focused, setFocused] = useState<number | null>(null);
  const choices: (string | null)[] = [null, ...INDICATOR_COLORS];
  const selected = choices.indexOf(chosenColor(color));
  const tabStop = focused ?? Math.max(selected, 0);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const last = choices.length - 1;
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
    <div className="option-row">
      <span id={labelId} className="option-label">
        {label}
      </span>
      <div
        className="swatches"
        role="group"
        aria-labelledby={labelId}
        onKeyDown={onKeyDown}
        onBlur={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget)) {
            setFocused(null);
          }
        }}
      >
        {choices.map((choice, index) => (
          <button
            key={choice ?? "default"}
            ref={(button) => {
              buttons.current[index] = button;
              // The swatch's color, through the CSSOM (no inline style).
              if (button && choice !== null) {
                button.style.setProperty("--swatch", choice);
              }
            }}
            type="button"
            className={choice === null ? "swatch swatch-default" : "swatch"}
            aria-label={choice === null ? defaultLabel : colorName(choice)}
            aria-pressed={index === selected}
            tabIndex={index === tabStop ? 0 : -1}
            disabled={disabled}
            onFocus={() => setFocused(index)}
            onClick={() => {
              setFocused(index);
              if (index !== selected) onSelect(choice);
            }}
          >
            {choice === null ? defaultLabel : null}
          </button>
        ))}
      </div>
    </div>
  );
}

type LookPanelProps = {
  /** The settings in use, or null while they can't be read. */
  settings: Settings | null;
  /** Back to the main view of the panel; none in the Configurações app,
   * where the views are sections. */
  onBack?: () => void;
  /** The programs pinned in the workspace in use, for the miniatures of the
   * styles; none in the panel, where they show widget tiles. */
  appIds?: readonly string[];
  /** The rows in titled cards, as the Configurações app shows them (spec
   * 0014 R1). */
  grouped?: boolean;
};

/**
 * Configurações › Aparência do dock (spec 0005 §3.5): the style of the bar
 * and the changes to it, what an item does under the pointer, and the
 * indicator. Every choice is applied and saved at once, like the rest of the
 * Settings panel.
 */
function LookPanel({
  settings,
  onBack,
  appIds = [],
  grouped = false,
}: LookPanelProps) {
  const t = useT();
  const { shown, change, error } = useSettingsEditor(settings);
  const disabled = shown === null;
  const dock = shown?.dock ?? null;
  const look = dock ? dockLookOf(dock) : null;
  const text = t.settings.look;
  const groups = t.settings.app.groups;

  const update = (patch: Partial<DockSettings>) =>
    change((s) => ({ ...s, dock: { ...s.dock, ...patch } }));
  const tweak = (patch: StyleTweaks) =>
    change((s) => ({
      ...s,
      dock: {
        ...s.dock,
        tweaks: {
          ...s.dock.tweaks,
          [s.dock.style]: { ...s.dock.tweaks[s.dock.style], ...patch },
        },
      },
    }));
  const restore = () =>
    change((s) => {
      const tweaks = { ...s.dock.tweaks };
      delete tweaks[s.dock.style];
      return { ...s, dock: { ...s.dock, tweaks } };
    });
  const tweaked = dock !== null && dock.tweaks[dock.style] !== undefined;

  // The changes to the chosen style: a list, so the panel and the app
  // put the same rows in their own frame.
  const tweaks = [
    <ToggleField
      key="border"
      label={text.border}
      checked={look?.border ?? false}
      disabled={disabled}
      onChange={(border) => tweak({ border })}
    />,
    <RangeField
      key="corners"
      label={text.corners}
      hint={text.cornersHint}
      value={look?.corners ?? MIN_CORNERS}
      min={MIN_CORNERS}
      max={MAX_CORNERS}
      step={5}
      display={(value) => `${value}%`}
      disabled={disabled}
      onCommit={(corners) => tweak({ corners })}
    />,
    <ToggleField
      key="reflections"
      label={text.reflections}
      hint={text.reflectionsHint}
      checked={look?.reflections ?? false}
      disabled={disabled}
      onChange={(reflections) => tweak({ reflections })}
    />,
    <div key="restore" className="toggle-row">
      <div className="toggle-text">
        <span className="toggle-label">{text.restore}</span>
        <span className="toggle-hint">{text.restoreHint}</span>
      </div>
      <button
        type="button"
        className="secondary-button"
        disabled={disabled || !tweaked}
        onClick={restore}
      >
        {text.restore}
      </button>
    </div>,
  ];

  return (
    <>
      {onBack ? (
        <PanelHeader title={text.title} onBack={onBack} focusAction />
      ) : (
        <PanelHeader title={text.title} />
      )}
      <SettingsGroup title={groups.style} grouped={grouped}>
        <StyleGallery
          label={text.style}
          dock={dock}
          appIds={appIds}
          disabled={disabled}
          onSelect={(style) => update({ style })}
        />
      </SettingsGroup>
      {grouped ? (
        <SettingsGroup title={groups.tweaks} grouped>
          {tweaks}
        </SettingsGroup>
      ) : (
        <section className="look-group" aria-label={text.tweaks}>
          <h2 className="look-group-title">{text.tweaks}</h2>
          {tweaks}
        </section>
      )}
      <SettingsGroup title={groups.hover} grouped={grouped}>
        {grouped ? (
          <ChoiceCards
            label={text.hover}
            choices={options(HOVERS, text.hoverOptions)}
            selected={dock?.hover ?? null}
            disabled={disabled}
            onSelect={(hover) => update({ hover })}
            thumb={(hover, chosen) => (
              <HoverThumb effect={hover} chosen={chosen} />
            )}
          />
        ) : (
          <OptionGroup
            label={text.hover}
            options={options(HOVERS, text.hoverOptions)}
            selected={dock?.hover ?? null}
            onSelect={(hover) => update({ hover })}
          />
        )}
        {dock?.hover === "scale" ? (
          <RangeField
            label={text.hoverScale}
            value={dock.hoverScale}
            min={MIN_HOVER_SCALE}
            max={MAX_HOVER_SCALE}
            step={5}
            display={(value) =>
              `${(value / 100).toFixed(2).replace(".", ",")}×`
            }
            onCommit={(hoverScale) => update({ hoverScale })}
          />
        ) : null}
      </SettingsGroup>
      <SettingsGroup title={groups.indicator} grouped={grouped}>
        {grouped ? (
          <ChoiceCards
            label={text.indicator}
            choices={options(INDICATORS, text.indicatorOptions)}
            selected={dock?.indicator ?? null}
            disabled={disabled}
            onSelect={(indicator) => update({ indicator })}
            thumb={(indicator) => <IndicatorThumb shape={indicator} />}
          />
        ) : (
          <OptionGroup
            label={text.indicator}
            options={options(INDICATORS, text.indicatorOptions)}
            selected={dock?.indicator ?? null}
            onSelect={(indicator) => update({ indicator })}
          />
        )}
        {/* The color right after the mark it colors. */}
        <SwatchGroup
          label={text.indicatorColor}
          defaultLabel={text.indicatorDefault}
          colorName={(color) =>
            text.colors[color as keyof typeof text.colors] ?? color
          }
          color={dock?.indicatorColor ?? null}
          disabled={disabled}
          onSelect={(indicatorColor) => update({ indicatorColor })}
        />
      </SettingsGroup>
      <SettingsGroup title={groups.icons} grouped={grouped}>
        {grouped ? (
          <ChoiceCards
            label={text.iconShape}
            choices={options(ICON_SHAPES, text.iconShapeOptions)}
            selected={dock?.iconShape ?? null}
            disabled={disabled}
            onSelect={(iconShape) => update({ iconShape })}
            thumb={(iconShape) => <ShapeThumb shape={iconShape} />}
          />
        ) : (
          <OptionGroup
            label={text.iconShape}
            options={options(ICON_SHAPES, text.iconShapeOptions)}
            selected={dock?.iconShape ?? null}
            onSelect={(iconShape) => update({ iconShape })}
          />
        )}
        <IconPacksField
          selected={dock?.iconPack ?? null}
          disabled={disabled}
          onSelect={(iconPack) => update({ iconPack })}
        />
      </SettingsGroup>
      <SettingsGroup title={groups.openAnimation} grouped={grouped}>
        {grouped ? (
          <ChoiceCards
            label={text.openAnimation}
            choices={options(OPEN_ANIMATIONS, text.openAnimationOptions)}
            selected={dock?.openAnimation ?? null}
            disabled={disabled}
            onSelect={(openAnimation) => update({ openAnimation })}
            thumb={(openAnimation, chosen) => (
              <OpenThumb animation={openAnimation} chosen={chosen} />
            )}
          />
        ) : (
          <OptionGroup
            label={text.openAnimation}
            options={options(OPEN_ANIMATIONS, text.openAnimationOptions)}
            selected={dock?.openAnimation ?? null}
            onSelect={(openAnimation) => update({ openAnimation })}
          />
        )}
      </SettingsGroup>
      <PanelFooter alert={error && t.settings.errors[error]}>
        {t.settings.footer}
      </PanelFooter>
    </>
  );
}

export default LookPanel;
