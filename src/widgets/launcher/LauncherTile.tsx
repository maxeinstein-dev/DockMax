import "../system/system.css";

/** The tile of a Lançador outside the dock: a magnifier. The dock draws its
 * own natively, with the search glyph of Windows (`native_dock`).
 * Decorative. */
function LauncherTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M10.5 4a6.5 6.5 0 1 1 0 13a6.5 6.5 0 1 1 0 -13M15.2 15.7l4.8 4.8" />
      </svg>
    </span>
  );
}

export default LauncherTile;
