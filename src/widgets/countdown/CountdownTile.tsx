import "../system/system.css";

/** The tile of a Contagem regressiva outside the dock: an hourglass. The
 * dock draws the live one natively (`native_dock/faces.rs`, `countdown`).
 * Decorative. */
function CountdownTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M7 4h10M7 20h10M8 4c0 4 4 5 4 8s-4 4-4 8M16 4c0 4-4 5-4 8s4 4 4 8" />
      </svg>
    </span>
  );
}

export default CountdownTile;
