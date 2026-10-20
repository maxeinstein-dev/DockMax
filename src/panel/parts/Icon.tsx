/* Line icons of the panel parts (DESIGN.md "Buttons": 16 px, a 1.75 stroke
 * with round ends, drawn for DockMax itself; parts.css sets the stroke).
 * Paths from the approved mock, `docs/design/widgets-mock.html`. */
const PATHS = {
  // Controls: the quiet "Ajustes" button.
  sliders: [
    "M4 7.5h9.5M17.5 7.5H20M4 16.5h3.5M11.5 16.5H20",
    "M13.5 7.5a2 2 0 1 0 4 0a2 2 0 1 0 -4 0",
    "M7.5 16.5a2 2 0 1 0 4 0a2 2 0 1 0 -4 0",
  ],
  back: ["M14.5 6.5l-5.5 5.5 5.5 5.5"],
  next: ["M9.5 6.5l5.5 5.5-5.5 5.5"],
  up: ["M7 14.5l5-5 5 5"],
  down: ["M7 9.5l5 5 5-5"],
  plus: ["M12 6v12M6 12h12"],
  trash: [
    "M5 7.5h14M10 7.5V5.5h4v2M7 7.5l.8 11a1.5 1.5 0 0 0 1.5 1.4h5.4a1.5 1.5 0 0 0 1.5-1.4l.8-11",
  ],
  warn: ["M12 4.5l8.5 15h-17z", "M12 10v4", "M12 16.8h.01"],
} satisfies Record<string, string[]>;

export type IconName = keyof typeof PATHS;

/** A decorative line icon: the control carrying it has the name. */
function Icon({ name }: { name: IconName }) {
  return (
    <svg className="part-icon" viewBox="0 0 24 24" aria-hidden="true">
      {PATHS[name].map((d) => (
        <path key={d} d={d} />
      ))}
    </svg>
  );
}

export default Icon;
