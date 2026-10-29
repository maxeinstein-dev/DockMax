import "../system/system.css";

/** The tile of a Uso do Claude outside the dock: a gauge. The dock draws
 * the live one natively (`native_dock/faces.rs`, `claude`). Decorative. */
function ClaudeTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M4.5 16a7.5 7.5 0 1 1 15 0M12 16l3.5 -4.5" />
      </svg>
    </span>
  );
}

export default ClaudeTile;
