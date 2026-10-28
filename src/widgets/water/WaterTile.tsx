import "../system/system.css";

/** The tile of a Beber água outside the dock: a drop. The dock draws the
 * live one natively (`native_dock/faces.rs`, `water`). Decorative. */
function WaterTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M12 3.5c3 4 5.5 7 5.5 10a5.5 5.5 0 0 1 -11 0c0 -3 2.5 -6 5.5 -10z" />
      </svg>
    </span>
  );
}

export default WaterTile;
