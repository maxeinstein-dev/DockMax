import { useEffect, useState } from "react";
import { iconPackPreview } from "../core/iconPacks";
import type { DockStyle } from "../core/settings";
import { MINI_ITEMS } from "./StyleGallery";

/** The first pinned programs drawn by `pack` (null: their own icons) on the
 * bar of `style` (the dock's own when null), read
 * again when `generation` changes (the packs were read again). Null until
 * they arrive, and when they can't be read. */
export function usePackPreview(
  pack: string | null,
  appIds: readonly string[],
  generation: number,
  style: DockStyle | null = null,
): (string | null)[] | null {
  const shown = appIds.slice(0, MINI_ITEMS);
  const key = `${generation} ${style ?? ""} ${pack ?? ""} ${shown.join(" ")}`;
  const [preview, setPreview] = useState<{
    key: string;
    icons: (string | null)[];
  } | null>(null);
  useEffect(() => {
    if (shown.length === 0) return;
    let active = true;
    iconPackPreview(pack, shown, style).then(
      (icons) => {
        if (active) setPreview({ key, icons });
      },
      () => {},
    );
    return () => {
      active = false;
    };
    // `key` stands for `pack`, `style`, `shown` and `generation`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return preview !== null && preview.key === key ? preview.icons : null;
}
