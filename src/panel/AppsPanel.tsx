import { useEffect, useRef, useState } from "react";
import {
  MAX_APPS,
  isAppCommandError,
  listCatalog,
  moveApp,
  pinApp,
  unpinApp,
  useAppIcon,
  ICON_SIZES,
  type AppErrorCode,
  type AppsSnapshot,
  type CatalogEntry,
  type PinnedApp,
} from "../core/apps";
import type { MoveDirection } from "../core/widgets";
import { format, useT } from "../i18n";
import PanelFooter from "./PanelFooter";
import IconButton from "./parts/IconButton";
import InlineNotice from "./parts/InlineNotice";
import PanelHeader from "./parts/PanelHeader";
import TextField from "./parts/TextField";
import "./AppsPanel.css";

/** Props of Configurações › Apps do dock (spec 0006 §3.5), a subview of the
 * Settings panel. Every accepted change comes back in `snapshot` through
 * `apps-changed`; a refused one shows its message (`t.apps.errors[code]`). */
export interface AppsPanelProps {
  /** The pinned programs in dock order and the storage state (`useApps`);
   * null while loading, and for good when they can't be read. */
  snapshot: AppsSnapshot | null;
  /** Voltar: back to Configurações, where the "Apps do dock" row takes the
   * focus. */
  onBack: () => void;
}

/** How many programs of the catalog show at once: the search narrows the
 * rest. */
const CATALOG_ROWS = 80;

/** A name as the search compares it: no case, no accents. */
function searchable(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

/** A program's icon at the size of the list; a blank square until it is
 * read, and when Windows has none. Not read before the row scrolls into
 * view: the catalog has a few hundred programs. */
function RowIcon({ appId }: { appId: string }) {
  const [seen, setSeen] = useState(typeof IntersectionObserver === "undefined");
  const mark = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    const element = mark.current;
    if (seen || element === null) return;
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) setSeen(true);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [seen]);
  return (
    <span ref={mark} className="apps-icon" aria-hidden="true">
      {seen ? <LoadedIcon appId={appId} /> : null}
    </span>
  );
}

function LoadedIcon({ appId }: { appId: string }) {
  const icon = useAppIcon(appId, ICON_SIZES.list);
  return icon === null ? null : <img src={icon} alt="" draggable={false} />;
}

/**
 * Configurações › Apps do dock: the pinned programs in dock order, each with
 * subir, descer and remover, and the programs installed on the PC to pin
 * from, with a search. Rust is the source of truth: nothing here changes
 * before `apps-changed` says so.
 */
function AppsPanel({ snapshot, onBack }: AppsPanelProps) {
  const t = useT();
  const [error, setError] = useState<AppErrorCode | null>(null);
  /** What the catalog read of one attempt answered; a newer attempt makes
   * it stale, and the list shows as loading again. */
  const [read, setRead] = useState<{
    attempt: number;
    list: CatalogEntry[] | "failed";
  } | null>(null);
  const [query, setQuery] = useState("");
  /** The program being pinned or unpinned: its buttons turn off. */
  const [busy, setBusy] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const catalog = read !== null && read.attempt === attempt ? read.list : null;
  const searchBox = useRef<HTMLDivElement>(null);

  const storage = snapshot?.storage.state ?? null;
  const available = snapshot !== null && storage !== "unavailable";
  const pinned: readonly PinnedApp[] = available ? snapshot.apps : [];
  const full = pinned.length >= MAX_APPS;

  useEffect(() => {
    if (!available) return;
    let active = true;
    listCatalog().then(
      (list) => {
        if (active) setRead({ attempt, list });
      },
      () => {
        if (active) setRead({ attempt, list: "failed" });
      },
    );
    return () => {
      active = false;
    };
  }, [available, attempt]);

  function fail(reason: unknown) {
    setError(isAppCommandError(reason) ? reason.code : "notReady");
  }

  function focusSearch() {
    searchBox.current?.querySelector("input")?.focus();
  }

  function pin(entry: CatalogEntry) {
    setError(null);
    setBusy(entry.appId);
    pinApp(entry.appId).then(
      () => {
        setBusy(null);
        focusSearch();
      },
      (reason: unknown) => {
        setBusy(null);
        fail(reason);
      },
    );
  }

  function unpin(app: PinnedApp) {
    setError(null);
    setBusy(app.id);
    unpinApp(app.id).then(
      () => setBusy(null),
      (reason: unknown) => {
        setBusy(null);
        fail(reason);
      },
    );
  }

  function move(app: PinnedApp, direction: MoveDirection) {
    setError(null);
    moveApp(app.id, direction).catch(fail);
  }

  const pinnedIds = new Set(pinned.map((app) => app.appId.toLowerCase()));
  const needle = searchable(query.trim());
  const matches =
    catalog === null || catalog === "failed"
      ? []
      : catalog.filter(
          (entry) => needle === "" || searchable(entry.name).includes(needle),
        );
  const shown = matches.slice(0, CATALOG_ROWS);

  return (
    <>
      <PanelHeader title={t.apps.title} onBack={onBack} focusAction />
      {storage === "unavailable" || snapshot === null ? (
        <InlineNotice
          title={t.apps.unavailableNotice.title}
          detail={t.apps.unavailableNotice.detail}
        />
      ) : null}
      {available ? (
        <>
          {pinned.length > 0 ? (
            <ul className="apps-list" aria-label={t.apps.listLabel}>
              {pinned.map((app, index) => (
                <li key={app.id} className="apps-row">
                  <RowIcon appId={app.appId} />
                  <div className="apps-name">{app.name}</div>
                  {(["up", "down"] as const).map((direction) => (
                    <IconButton
                      key={direction}
                      icon={direction}
                      label={format(
                        direction === "up" ? t.apps.moveUp : t.apps.moveDown,
                        { name: app.name },
                      )}
                      disabled={
                        direction === "up"
                          ? index === 0
                          : index === pinned.length - 1
                      }
                      onClick={() => move(app, direction)}
                    />
                  ))}
                  <IconButton
                    icon="trash"
                    tone="danger"
                    label={format(t.apps.remove, { name: app.name })}
                    disabled={busy === app.id}
                    onClick={() => unpin(app)}
                  />
                </li>
              ))}
            </ul>
          ) : (
            <p className="apps-empty">{t.apps.empty}</p>
          )}
          <div className="apps-add">
            <div className="apps-add-head">
              <span className="apps-add-label">{t.apps.add}</span>
              <span className="apps-count">
                {full
                  ? format(t.apps.limit, { max: MAX_APPS })
                  : format(t.apps.count, {
                      count: pinned.length,
                      max: MAX_APPS,
                    })}
              </span>
            </div>
            <div ref={searchBox}>
              <TextField
                label={t.apps.searchLabel}
                hideLabel
                value={query}
                onChange={setQuery}
                placeholder={t.apps.searchPlaceholder}
              />
            </div>
            {catalog === null ? (
              <p className="apps-empty" role="status">
                {t.apps.loading}
              </p>
            ) : catalog === "failed" ? (
              <div className="apps-failed">
                <p className="apps-empty" role="alert">
                  {t.apps.catalogFailed}
                </p>
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => setAttempt((count) => count + 1)}
                >
                  {t.apps.retry}
                </button>
              </div>
            ) : shown.length === 0 ? (
              <p className="apps-empty">{t.apps.noMatch}</p>
            ) : (
              <ul className="apps-catalog" aria-label={t.apps.catalogLabel}>
                {shown.map((entry) => {
                  const isPinned = pinnedIds.has(entry.appId.toLowerCase());
                  return (
                    <li key={entry.appId}>
                      <button
                        type="button"
                        className="apps-pick"
                        aria-label={format(
                          isPinned ? t.apps.pinned : t.apps.pin,
                          { name: entry.name },
                        )}
                        disabled={isPinned || full || busy === entry.appId}
                        onClick={() => pin(entry)}
                      >
                        <RowIcon appId={entry.appId} />
                        <span className="apps-name">{entry.name}</span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        </>
      ) : null}
      <PanelFooter alert={error && t.apps.errors[error]}>
        {available ? t.apps.footer : t.apps.footerUnavailable}
      </PanelFooter>
    </>
  );
}

export default AppsPanel;
