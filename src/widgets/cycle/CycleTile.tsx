import "../system/system.css";

/** The tile of a Ciclo outside the dock: a plain ring, discreet. The dock
 * draws the live one natively (`native_dock/faces.rs`, `cycle`).
 * Decorative. */
function CycleTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M3.5 12a8.5 8.5 0 1 0 17 0a8.5 8.5 0 1 0 -17 0" />
      </svg>
    </span>
  );
}

export default CycleTile;
