import "../system/system.css";

/** The tile of a Pomodoro outside the dock: a timer. The dock draws the
 * live one natively (`native_dock/faces.rs`, `pomodoro`). Decorative. */
function PomodoroTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M5 13.5a7 7 0 1 0 14 0a7 7 0 1 0 -14 0M12 13.5V10M10 3.5h4" />
      </svg>
    </span>
  );
}

export default PomodoroTile;
