/** The Windows panels the dock opens on a click, in place of the taskbar's
 * own (ADR 0009): the calendar of the clock, and the quick settings of
 * battery, network and volume. Must match `Flyout` in
 * `src-tauri/src/windows/flyout.rs`. */
export type Flyout = "calendar" | "quickSettings";

/** Command that opens a `Flyout`. Only the dock may call it. */
export const OPEN_FLYOUT_COMMAND = "open_flyout";

export interface OpenFlyoutArgs {
  flyout: Flyout;
}
