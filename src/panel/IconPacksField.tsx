import { useCallback, useEffect, useState } from "react";
import {
  iconPackPreview,
  listIconPacks,
  openIconPacksFolder,
  reloadIconPacks,
  type IconPackInfo,
} from "../core/iconPacks";
import { format, useT } from "../i18n";
import ChoiceCards from "./parts/ChoiceCards";
import { MINI_ITEMS, MiniDock, type GalleryDock } from "./StyleGallery";

/** The value of the "Original" option: no id is the empty string. */
const ORIGINAL = "";

/** The first pinned programs drawn by `pack` (null: their own icons), read
 * again when `generation` changes (the packs were read again). Null until
 * they arrive, and when they can't be read. */
function usePreview(
  pack: string | null,
  appIds: readonly string[],
  generation: number,
): (string | null)[] | null {
  const shown = appIds.slice(0, MINI_ITEMS);
  const key = `${generation} ${pack ?? ""} ${shown.join(" ")}`;
  const [preview, setPreview] = useState<{
    key: string;
    icons: (string | null)[];
  } | null>(null);
  useEffect(() => {
    if (shown.length === 0) return;
    let active = true;
    iconPackPreview(pack, shown).then(
      (icons) => {
        if (active) setPreview({ key, icons });
      },
      () => {},
    );
    return () => {
      active = false;
    };
    // `key` stands for `pack`, `shown` and `generation`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return preview !== null && preview.key === key ? preview.icons : null;
}

/** The dock with the pinned programs in the icons of `pack`: the picture of
 * its card, decorative. */
function PackThumb({
  pack,
  dock,
  appIds,
  generation,
}: {
  pack: string | null;
  dock: GalleryDock | null;
  appIds: readonly string[];
  generation: number;
}) {
  const icons = usePreview(pack, appIds, generation);
  return (
    <MiniDock
      of={dock?.style ?? "native"}
      dock={dock}
      appIds={appIds}
      icons={icons ?? appIds.slice(0, MINI_ITEMS).map(() => null)}
    />
  );
}

/** The icon pack of the dock, chosen among the packs of the packs folder
 * (spec 0007) as a grid of cards like the dock's styles (spec 0014 R1):
 * Original, or one that is there, each with the dock drawn in its icons; a
 * chosen pack that is gone stays as unavailable. The packs are read when the
 * view opens and again with "Recarregar pacotes". */
function IconPacksField({
  selected,
  dock,
  appIds,
  disabled,
  onSelect,
}: {
  selected: string | null;
  /** The dock's settings, for the style of the miniatures. */
  dock: GalleryDock | null;
  /** The programs pinned in the workspace in use, in dock order. */
  appIds: readonly string[];
  disabled?: boolean;
  onSelect: (id: string | null) => void;
}) {
  const text = useT().settings.look;
  const [packs, setPacks] = useState<IconPackInfo[] | null>(null);
  const [failed, setFailed] = useState(false);
  const [generation, setGeneration] = useState(0);

  const load = useCallback((read: () => Promise<IconPackInfo[]>) => {
    let active = true;
    read().then(
      (found) => {
        if (!active) return;
        setPacks(found);
        setFailed(false);
        setGeneration((n) => n + 1);
      },
      () => {
        if (active) setFailed(true);
      },
    );
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => load(listIconPacks), [load]);

  const usable = (packs ?? []).filter((pack) => pack.error === null);
  const broken = (packs ?? []).filter((pack) => pack.error !== null);

  const choices = [
    { value: ORIGINAL, label: text.iconPackOriginal },
    ...usable.map((pack) => ({
      value: pack.id,
      label: pack.name,
      hint:
        pack.total > 0
          ? format(text.iconPackCovers, {
              covers: pack.covers,
              total: pack.total,
            })
          : format(text.iconPackCount, { count: pack.iconCount }),
    })),
  ];
  if (selected !== null && !usable.some((pack) => pack.id === selected)) {
    choices.push({
      value: selected,
      label: selected,
      // While the packs are being read the chosen one is not "gone" yet.
      hint: packs === null ? text.iconPackLoading : text.iconPackMissing,
    });
  }

  return (
    <div className="pack-field">
      <ChoiceCards
        label={text.iconPack}
        choices={choices}
        selected={selected ?? ORIGINAL}
        disabled={disabled}
        onSelect={(value) => onSelect(value === ORIGINAL ? null : value)}
        thumb={(value) => (
          <PackThumb
            pack={value === ORIGINAL ? null : value}
            dock={dock}
            appIds={appIds}
            generation={generation}
          />
        )}
        wide
      />
      <p className="look-hint">
        {failed ? text.iconPackFailed : text.iconPackHint}
      </p>
      {broken.length > 0 ? (
        <ul className="pack-problems">
          {broken.map((pack) => (
            <li key={pack.id}>
              {format(text.packProblem, {
                id: pack.id,
                reason: pack.error ? text.packErrors[pack.error] : "",
              })}
            </li>
          ))}
        </ul>
      ) : null}
      <div className="pack-actions">
        <button
          type="button"
          className="secondary-button"
          onClick={() => {
            void openIconPacksFolder().catch(() => {});
          }}
        >
          {text.openPacksFolder}
        </button>
        <button
          type="button"
          className="secondary-button"
          onClick={() => {
            load(reloadIconPacks);
          }}
        >
          {text.reloadPacks}
        </button>
      </div>
    </div>
  );
}

export default IconPacksField;
