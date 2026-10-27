import "../system/system.css";

/** The tile of an Atividade de rede outside the dock, where there is no
 * reading: the arrows down and up. The dock draws the live one natively
 * (`native_dock/paint.rs`, `Face::Rates`). Decorative. */
function NetworkTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M8.5 4.5v15M4.5 15.5l4 4 4-4M15.5 19.5v-15M11.5 8.5l4-4 4 4" />
      </svg>
    </span>
  );
}

export default NetworkTile;
