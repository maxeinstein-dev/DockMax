import type { SystemPanelId } from "../core/panels";

/* Line glyphs drawn for DockMax itself (DESIGN.md "Dock item": one set, a
 * 24 px viewBox, a 1.75 stroke with round ends; Dock.css sets the stroke).
 * No third-party icon. */
const PATHS: Record<SystemPanelId, string[]> = {
  // Two sliders.
  settings: [
    "M4 7.5h9.5M17.5 7.5H20M4 16.5h3.5M11.5 16.5H20",
    "M13.5 7.5a2 2 0 1 0 4 0a2 2 0 1 0 -4 0",
    "M7.5 16.5a2 2 0 1 0 4 0a2 2 0 1 0 -4 0",
  ],
  // An arrow up in a rounded square: more icons.
  tray: [
    "M6 9.5l6 6 6-6",
    "M4.5 5h15a1.5 1.5 0 0 1 1.5 1.5v11a1.5 1.5 0 0 1-1.5 1.5h-15a1.5 1.5 0 0 1-1.5-1.5v-11A1.5 1.5 0 0 1 4.5 5z",
  ],
  // "i" in a circle.
  about: [
    "M3.5 12a8.5 8.5 0 1 0 17 0a8.5 8.5 0 1 0 -17 0",
    "M12 11v5.5",
    "M12 7.6h.01",
  ],
};

function Glyph({ id }: { id: SystemPanelId }) {
  return (
    <svg className="dock-glyph" viewBox="0 0 24 24" aria-hidden="true">
      {PATHS[id].map((d) => (
        <path key={d} d={d} />
      ))}
    </svg>
  );
}

export default Glyph;
