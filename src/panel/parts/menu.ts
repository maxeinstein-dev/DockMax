import type { KeyboardEvent } from "react";

/** The menu items of `root` that can be chosen, in order. */
export function menuItems(root: HTMLElement | null): HTMLButtonElement[] {
  return Array.from(
    root?.querySelectorAll<HTMLButtonElement>(
      'button[role="menuitem"]:not(:disabled)',
    ) ?? [],
  );
}

/** The arrow keys, Home and End move the focus between the items of the menu
 * in `root`, without choosing. */
export function moveInMenu(
  root: HTMLElement | null,
  event: KeyboardEvent<HTMLElement>,
) {
  const buttons = menuItems(root);
  const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
  const target = {
    ArrowDown: buttons[Math.min(at + 1, buttons.length - 1)],
    ArrowUp: buttons[Math.max(at - 1, 0)],
    Home: buttons[0],
    End: buttons[buttons.length - 1],
  }[event.key];
  if (target) {
    event.preventDefault();
    target.focus();
  }
}
