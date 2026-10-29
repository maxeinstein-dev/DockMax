import { useId, useState } from "react";
import {
  MAX_WIDGETS,
  addWidget,
  isWidgetCommandError,
  type CreatableKind,
  type WidgetErrorCode,
  type WidgetsSnapshot,
} from "../core/widgets";
import { inWorkspace, type Workspace } from "../core/workspaces";
import { format, useT } from "../i18n";
import { MiniTile } from "../panel/DockItems";
import "./WidgetCatalog.css";

type Category = "system" | "time" | "media" | "productivity" | "personal";

/** The catalog's shelves, in order, with their kinds. */
const SHELVES: readonly { category: Category; kinds: CreatableKind[] }[] = [
  { category: "system", kinds: ["activity", "network"] },
  {
    category: "time",
    kinds: ["calendar", "clock", "countdown", "pomodoro", "time-progress"],
  },
  { category: "productivity", kinds: ["todo", "note", "water", "claude"] },
  { category: "media", kinds: ["media"] },
  { category: "personal", kinds: ["cycle"] },
];

/** How many icons wide a new instance of each kind is (its default). */
const WIDTHS: Readonly<Record<CreatableKind, number>> = {
  clock: 1,
  "time-progress": 1,
  note: 1,
  activity: 1,
  network: 2,
  calendar: 2,
  cycle: 1,
  countdown: 1,
  pomodoro: 1,
  todo: 1,
  water: 1,
  media: 2,
  claude: 2,
};

/** Text compared without case or accents: "relogio" finds "Relógio". */
function folded(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

/** A kind's thumbnail: its tile, as wide as a new one is in the dock.
 * Decorative: the card names it. */
function Thumb({ kind }: { kind: CreatableKind }) {
  return (
    <span className="catalog-thumb" aria-hidden="true">
      <MiniTile kind={kind} wide={WIDTHS[kind] > 1} />
    </span>
  );
}

/**
 * Configurações › Widgets, the catalog (spec 0017 F1): every kind on its
 * shelf, with its thumbnail, what it does and Adicionar, which puts a new
 * one at the end of the workspace being edited. A search narrows the
 * shelves by name and description. Rust is the source of truth: "Em uso"
 * (`DockItems`) shows the new one when `widgets-changed` comes.
 */
function WidgetCatalog({
  snapshot,
  workspace,
}: {
  snapshot: WidgetsSnapshot | null;
  workspace: Workspace | null;
}) {
  const t = useT();
  const text = t.widgets.catalog;
  const searchId = useId();
  const [query, setQuery] = useState("");
  const [error, setError] = useState<WidgetErrorCode | null>(null);
  const [said, setSaid] = useState("");
  const [adding, setAdding] = useState<CreatableKind | null>(null);

  const available =
    snapshot !== null && snapshot.storage.state !== "unavailable";
  if (!available) return null;
  const count = inWorkspace(snapshot.widgets, workspace, "widget").length;
  const full = count >= MAX_WIDGETS;

  const wanted = folded(query.trim());
  const matches = (kind: CreatableKind) =>
    wanted === "" ||
    folded(`${t.widgets.kinds[kind]} ${text.descriptions[kind]}`).includes(
      wanted,
    );
  const shelves = SHELVES.map((shelf) => ({
    ...shelf,
    kinds: shelf.kinds.filter(matches),
  })).filter((shelf) => shelf.kinds.length > 0);

  function add(kind: CreatableKind) {
    setError(null);
    setSaid("");
    setAdding(kind);
    addWidget({ kind, workspace: workspace?.id }).then(
      () => {
        setAdding(null);
        setSaid(format(text.added, { name: t.widgets.kinds[kind] }));
      },
      (reason: unknown) => {
        setAdding(null);
        setError(isWidgetCommandError(reason) ? reason.code : "notReady");
      },
    );
  }

  return (
    <section className="catalog" aria-labelledby={`${searchId}-title`}>
      <div className="catalog-head">
        <h2 id={`${searchId}-title`} className="catalog-title">
          {text.title}
        </h2>
        <span className="catalog-count">
          {full
            ? format(t.widgets.items.limit, { max: MAX_WIDGETS })
            : format(t.widgets.items.count, { count, max: MAX_WIDGETS })}
        </span>
      </div>
      <input
        id={searchId}
        type="search"
        className="catalog-search"
        placeholder={text.search}
        aria-label={text.search}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      {shelves.length === 0 ? (
        <p className="catalog-none">{text.none}</p>
      ) : (
        shelves.map((shelf) => (
          <div key={shelf.category} className="catalog-shelf">
            <h3 className="catalog-shelf-title">
              {text.categories[shelf.category]}
            </h3>
            <ul className="catalog-cards">
              {shelf.kinds.map((kind) => {
                const name = t.widgets.kinds[kind];
                return (
                  <li key={kind} className="catalog-card">
                    <Thumb kind={kind} />
                    <div className="catalog-card-text">
                      <span className="catalog-card-name">{name}</span>
                      <span className="catalog-card-detail">
                        {text.descriptions[kind]}
                      </span>
                    </div>
                    <button
                      type="button"
                      className="secondary-button"
                      aria-label={format(text.addNamed, { name })}
                      disabled={full || adding !== null}
                      onClick={() => add(kind)}
                    >
                      {text.add}
                    </button>
                  </li>
                );
              })}
            </ul>
          </div>
        ))
      )}
      <p className="catalog-status" role="status">
        {said}
      </p>
      {error ? (
        <p className="catalog-error" role="alert">
          {t.widgets.errors[error]}
        </p>
      ) : null}
    </section>
  );
}

export default WidgetCatalog;
