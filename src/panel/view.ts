/** Which view of the panel shows: its main view, or its subview (a
 * widget's Ajustes, Configurações › Itens do dock). */
export interface PanelView {
  sub: boolean;
  /** The main view was reached by Voltar from the subview: the control
   * that opened the subview takes the focus. */
  returned: boolean;
}

export const MAIN_VIEW: PanelView = { sub: false, returned: false };
export const SUB_VIEW: PanelView = { sub: true, returned: false };
export const BACK_VIEW: PanelView = { sub: false, returned: true };
