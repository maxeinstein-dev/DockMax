import "../system/system.css";

/** The tile of a Lista de tarefas outside the dock: a list with marks.
 * The dock draws the live one natively (`native_dock/faces.rs`, `todo`).
 * Decorative. */
function TodoTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M4.5 7l1.5 1.5 3-3M4.5 14l1.5 1.5 3-3M12 7.5h7.5M12 14.5h7.5" />
      </svg>
    </span>
  );
}

export default TodoTile;
