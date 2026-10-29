import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorKind, log } from "./log";

/* The Tocando agora's panel (spec 0017 F4): what Windows plays now, read in
 * Rust (`windows/media.rs`), asked every second while the panel is open.
 * Must match `MediaView` in `commands/media.rs`. */

export interface MediaView {
  title: string;
  artist: string;
  /** The program's name. */
  app: string;
  playing: boolean;
  canToggle: boolean;
  canPrevious: boolean;
  canNext: boolean;
  /** The cover as a `data:` URL. */
  art: string | null;
}

export type MediaAction = "toggle" | "previous" | "next";

/** How often an open panel asks again. */
export const MEDIA_EVERY_MS = 1_000;

export function mediaStatus(): Promise<MediaView | null> {
  return invoke<MediaView | null>("media_status");
}

export function mediaControl(action: MediaAction): Promise<void> {
  return invoke<void>("media_control", { action });
}

/** What plays now (`null` when nothing does; `undefined` until read),
 * asked every second while mounted. */
export function useMedia(): MediaView | null | undefined {
  const [view, setView] = useState<MediaView | null | undefined>(undefined);
  useEffect(() => {
    let active = true;
    const ask = () => {
      mediaStatus().then(
        (next) => {
          if (active) setView(next);
        },
        (error: unknown) => {
          log.warn(`media: read failed: ${errorKind(error)}`);
        },
      );
    };
    ask();
    const every = window.setInterval(ask, MEDIA_EVERY_MS);
    return () => {
      active = false;
      window.clearInterval(every);
    };
  }, []);
  return view;
}
