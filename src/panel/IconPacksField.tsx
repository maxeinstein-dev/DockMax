import { useCallback, useEffect, useState } from "react";
import {
  listIconPacks,
  openIconPacksFolder,
  reloadIconPacks,
  type IconPackInfo,
} from "../core/iconPacks";
import { format, useT } from "../i18n";
import SelectField from "./parts/SelectField";

/** The value of the "Original" option: no id is the empty string. */
const ORIGINAL = "";

/** The icon pack of the dock, chosen among the packs of the packs folder
 * (spec 0007): Original, or one that is there; a chosen pack that is gone
 * stays in the list as unavailable. The packs are read when the view opens
 * and again with "Recarregar pacotes". */
function IconPacksField({
  selected,
  disabled,
  onSelect,
}: {
  selected: string | null;
  disabled?: boolean;
  onSelect: (id: string | null) => void;
}) {
  const text = useT().settings.look;
  const [packs, setPacks] = useState<IconPackInfo[] | null>(null);
  const [failed, setFailed] = useState(false);

  const load = useCallback((read: () => Promise<IconPackInfo[]>) => {
    let active = true;
    read().then(
      (found) => {
        if (!active) return;
        setPacks(found);
        setFailed(false);
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

  const options = [
    { value: ORIGINAL, label: text.iconPackOriginal },
    ...usable.map((pack) => ({
      value: pack.id,
      label:
        pack.total > 0
          ? format(text.iconPackCovers, {
              name: pack.name,
              covers: pack.covers,
              total: pack.total,
            })
          : format(text.iconPackCount, {
              name: pack.name,
              count: pack.iconCount,
            }),
    })),
  ];
  if (selected !== null && !usable.some((pack) => pack.id === selected)) {
    options.push({
      value: selected,
      // While the packs are being read the chosen one is not "gone" yet.
      label:
        packs === null
          ? selected
          : format(text.iconPackMissing, { id: selected }),
    });
  }

  return (
    <div className="pack-field">
      <SelectField
        label={text.iconPack}
        value={selected ?? ORIGINAL}
        options={options}
        disabled={disabled}
        onChange={(value) => onSelect(value === ORIGINAL ? null : value)}
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
