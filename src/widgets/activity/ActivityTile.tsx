import "../system/system.css";

/** The tile of an Atividade outside the dock, where there is no reading: a
 * meter glyph. The dock draws the live one natively
 * (`native_dock/paint.rs`, `Face::Meters`). Decorative. */
function ActivityTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M7 19V11M12 19V5M17 19v-5" />
      </svg>
    </span>
  );
}

export default ActivityTile;
