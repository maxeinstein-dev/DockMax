import { invoke } from "@tauri-apps/api/core";

/* The welcome tour (spec 0019 G2): a window of its own, opened by Rust on
 * the first run and by "Ver o tour" in Sobre. */

/** Opens the tour, or brings it to the front. */
export function openTour(): Promise<void> {
  return invoke("open_tour");
}

/** Closes the tour; with `settings`, Configurações opens. Only the tour's
 * window may call this. */
export function finishTour(settings: boolean): Promise<void> {
  return invoke("finish_tour", { settings });
}
