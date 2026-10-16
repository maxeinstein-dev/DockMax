/** PROVISIONAL (W3 replaces it): a note's dock icon is static, so its
 * face is the notes glyph (DESIGN.md "Dock item": 24 px viewBox, 1.75
 * stroke), and it takes none of the `WidgetTileProps`. Decorative. */
function NoteTile() {
  return (
    <svg className="dock-glyph" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="5" y="3.5" width="14" height="17" rx="2.5" />
      <path d="M8.5 8.5h7M8.5 12h7M8.5 15.5h4" />
    </svg>
  );
}

export default NoteTile;
