import type { WidgetInstance } from "../core/widgets";
import type { Messages } from "../i18n";

/** What an instance is called: its label, or its kind's name without one.
 * The dock item's `aria-label` and pill, the panel title and the name in
 * Itens do dock (spec 0003 §2.3 item 4: never the time). */
export function widgetName(t: Messages, instance: WidgetInstance): string {
  return instance.label ?? t.widgets.kinds[instance.kind];
}
