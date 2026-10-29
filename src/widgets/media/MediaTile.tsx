import "../system/system.css";

/** The tile of a Tocando agora outside the dock: a note. The dock draws
 * the live one natively, with the cover (`native_dock`, `media_face`).
 * Decorative. */
function MediaTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M9 17.5V5.5l10 -2v12M9 17.5a2.5 2.5 0 1 1 -5 0a2.5 2.5 0 1 1 5 0M19 15.5a2.5 2.5 0 1 1 -5 0a2.5 2.5 0 1 1 5 0" />
      </svg>
    </span>
  );
}

export default MediaTile;
