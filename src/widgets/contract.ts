import type {
  CreatableKind,
  WidgetConfigs,
  WidgetInstance,
  WidgetKind,
} from "../core/widgets";

/* The contract between the widget lanes and the host (PR 2, W0). Each kind
 * gives three components, listed in `registry.ts`:
 *
 * - `Tile`: the live face of its dock icon, inside the glaze (W5 draws the
 *   tile, its glaze and the label pill);
 * - `Panel`: the main view of its panel (title, "Ajustes", content, footer);
 * - `Settings`: the "Ajustes" view of the same sheet (label, the kind's
 *   fields, "Remover do dock", footer).
 *
 * The host (W5: `Dock.tsx` for tiles, `PanelContent.tsx` for panels) owns
 * the instance list (`useWidgets`), the timers and which view is shown;
 * the components own their drafts and their error messages. Rust is the
 * source of truth: `instance` is always the last accepted one, so a
 * rejected change shows by going back to it. */

/** A kind's label and config, in full, as `update_widget` takes them. */
export interface WidgetUpdate<K extends WidgetKind> {
  /** The label, or null for the kind's name. At most `MAX_LABEL_CHARS`
   * code points (`labelLength`), checked again in Rust. */
  label: string | null;
  config: WidgetConfigs[K];
}

/** Props of a kind's `Tile`. */
export interface WidgetTileProps<K extends CreatableKind> {
  instance: WidgetInstance<K>;
  /** The instant to show, from the dock's single timer: it ticks at each
   * minute turn and again when the system clock changes (W5). The tile
   * never runs a timer of its own. */
  now: Date;
}

/** Props of a kind's `Panel`, the main view. */
export interface WidgetPanelProps<K extends CreatableKind> {
  instance: WidgetInstance<K>;
  /** The instant to show, from the panel's timer: every second while the
   * panel is open (the clock's seconds), aligned to the second (W5). */
  now: Date;
  /** "Ajustes" was clicked: the host shows `Settings` in the same sheet. */
  onAdjust: () => void;
  /** True when the view was reached by Voltar from `Settings`: the
   * "Ajustes" button takes the focus (`PanelHeader` `focusAction`). */
  returnedFromSettings: boolean;
}

/** Props of a kind's `Settings`, the "Ajustes" view. */
export interface WidgetSettingsProps<K extends CreatableKind> {
  instance: WidgetInstance<K>;
  /** Saves the label and the config, both in full (`update_widget`).
   * Resolves once Rust accepted it, when `widgets-changed` brings the new
   * `instance`; rejects with a `WidgetCommandError`, whose message the
   * view shows (`t.widgets.errors[code]`) while its fields go back to
   * `instance`. */
  onUpdate: (update: WidgetUpdate<K>) => Promise<void>;
  /** Removes the instance (`remove_widget`): a note's text goes with it.
   * The view asks first when it has to (a note with text, through
   * `InlineConfirm`). Resolves once removed, and the host closes the
   * panel; rejects with a `WidgetCommandError`. */
  onRemove: () => Promise<void>;
  /** Voltar: the host shows `Panel` again (`returnedFromSettings`). The
   * view mounts with the focus on Voltar (`PanelHeader` `focusAction`). */
  onBack: () => void;
}
