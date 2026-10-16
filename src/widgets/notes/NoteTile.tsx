/** A note's dock icon: static, the notes glyph of the mock (DESIGN.md "Dock
 * item": a 24 px viewBox, a 1.75 stroke with round ends, which Dock.css
 * sets on `dock-glyph`). It shows nothing of the note, so it takes none of
 * the `WidgetTileProps`; decorative, since the item carries the name. */
function NoteTile() {
  return (
    <svg className="dock-glyph" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="5" y="3.5" width="14" height="17" rx="2.5" />
      <path d="M8.5 8.5h7M8.5 12h7M8.5 15.5h4" />
    </svg>
  );
}

export default NoteTile;
