/** The subviews a panel has: a widget's Ajustes, and Configurações ›
 * Itens do dock or Aparência do dock. */
export type SubView = "items" | "look";

/** Which view of the panel shows: its main view, or one of its subviews. */
export interface PanelView {
  sub: SubView | null;
  /** The main view was reached by Voltar from the subview: the control
   * that opened the subview takes the focus. */
  returned: boolean;
  /** The subview Voltar came from, when `returned`. */
  from: SubView | null;
}

export const MAIN_VIEW: PanelView = { sub: null, returned: false, from: null };
/** A widget's Ajustes, and Configurações › Itens do dock. */
export const SUB_VIEW: PanelView = {
  sub: "items",
  returned: false,
  from: null,
};
export const LOOK_VIEW: PanelView = {
  sub: "look",
  returned: false,
  from: null,
};
/** Voltar from a widget's Ajustes or from Itens do dock. */
export const BACK_VIEW: PanelView = {
  sub: null,
  returned: true,
  from: "items",
};
/** Voltar from Aparência do dock. */
export const BACK_FROM_LOOK_VIEW: PanelView = {
  sub: null,
  returned: true,
  from: "look",
};
