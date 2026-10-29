import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { MediaView } from "../../core/media";
import type { WidgetInstance } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import MediaPanel from "./MediaPanel";

const text = ptBR.widgets.media;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const media: WidgetInstance<"media"> = {
  id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6096",
  label: null,
  kind: "media",
  config: { width: 2 },
};

const PLAYING: MediaView = {
  title: "Garota de Ipanema",
  artist: "Tom Jobim",
  app: "Spotify",
  playing: true,
  canToggle: true,
  canPrevious: false,
  canNext: true,
  art: "data:image/png;base64,AAAA",
  icon: "data:image/png;base64,BBBB",
};

function panel() {
  render(
    <MediaPanel
      instance={media}
      now={new Date()}
      onAdjust={() => undefined}
      returnedFromSettings={false}
    />,
  );
}

beforeEach(() => {
  invoke.mockReset();
});

describe("MediaPanel", () => {
  it("shows the track, the program and the cover, and pauses it", async () => {
    invoke.mockImplementation(() => Promise.resolve(PLAYING));
    panel();
    expect(await screen.findByText("Garota de Ipanema")).toBeInTheDocument();
    expect(screen.getByText("Tom Jobim")).toBeInTheDocument();
    expect(screen.getByText("Tocando em Spotify")).toBeInTheDocument();
    expect(document.querySelector(".media-cover img")).toHaveAttribute(
      "src",
      PLAYING.art,
    );
    expect(screen.getByRole("button", { name: text.previous })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: text.pause }));
    expect(invoke).toHaveBeenCalledWith("media_control", { action: "toggle" });
    fireEvent.click(screen.getByRole("button", { name: text.next }));
    expect(invoke).toHaveBeenCalledWith("media_control", { action: "next" });
  });

  it("offers Tocar while paused", async () => {
    invoke.mockImplementation(() =>
      Promise.resolve({ ...PLAYING, playing: false, art: null }),
    );
    panel();
    expect(
      await screen.findByRole("button", { name: text.play }),
    ).toBeInTheDocument();
  });

  it("says when nothing plays", async () => {
    invoke.mockImplementation(() => Promise.resolve(null));
    panel();
    expect(await screen.findByText(text.nothing)).toBeInTheDocument();
  });
});
