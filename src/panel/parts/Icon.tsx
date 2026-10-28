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
  // The actions of a program's menu (spec 0014 R3).
  launch: [
    "M13.5 5.5h5v5",
    "M18.5 5.5l-7.5 7.5",
    "M17 13.5v4a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 5 17.5v-9A1.5 1.5 0 0 1 6.5 7h4",
  ],
  folder: [
    "M4.5 7.5a1.5 1.5 0 0 1 1.5-1.5h3.5l2 2H18a1.5 1.5 0 0 1 1.5 1.5v7.5A1.5 1.5 0 0 1 18 18.5H6a1.5 1.5 0 0 1-1.5-1.5z",
  ],
  close: ["M7 7l10 10M17 7L7 17"],
  unpin: ["M9 4.5h6M10 4.5v5l-3 3.5h10l-3-3.5v-5", "M12 13v6.5", "M5 5l14 14"],
  // The sections of the Configurações app (spec 0014 R1), on their plates.
  general: [
    "M12 9a3 3 0 1 0 0 6a3 3 0 1 0 0-6",
    "M12 3.5v2.2M12 18.3v2.2M3.5 12h2.2M18.3 12h2.2M6 6l1.6 1.6M16.4 16.4L18 18M6 18l1.6-1.6M16.4 7.6L18 6",
  ],
  workspaces: [
    "M5 5h5.5v5.5H5zM13.5 5H19v5.5h-5.5zM5 13.5h5.5V19H5zM13.5 13.5H19V19h-5.5z",
  ],
  look: [
    "M14.5 4.5l5 5-7.5 7.5-5-5z",
    "M7 12l-2.2 2.2a2.5 2.5 0 0 0 0 3.5l1.5 1.5a2.5 2.5 0 0 0 3.5 0L12 17",
  ],
  apps: [
    "M5 5h4v4H5zM10 5h4v4h-4zM15 5h4v4h-4zM5 10h4v4H5zM10 10h4v4h-4zM15 10h4v4h-4zM5 15h4v4H5zM10 15h4v4h-4zM15 15h4v4h-4z",
  ],
  items: ["M4.5 6.5h9v11h-9z", "M16 6.5h3.5v4.5H16zM16 13h3.5v4.5H16z"],
  accounts: [
    "M12 4.5a3.5 3.5 0 1 0 0 7a3.5 3.5 0 1 0 0-7",
    "M5 19.5a7 7 0 0 1 14 0",
  ],
  about: [
    "M3.5 12a8.5 8.5 0 1 0 17 0a8.5 8.5 0 1 0 -17 0",
    "M12 11v5.5",
    "M12 7.8h.01",
  ],
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
