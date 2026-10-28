import "../system/system.css";

/** The tile of a Calendário outside the dock, where there is no reading: a
 * calendar glyph. The dock draws the live one natively
 * (`native_dock/paint.rs`, `Face::Agenda`). Decorative. */
function CalendarTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M5 6.5h14v13H5zM5 10.5h14M9 4v4M15 4v4" />
      </svg>
    </span>
  );
}

export default CalendarTile;
