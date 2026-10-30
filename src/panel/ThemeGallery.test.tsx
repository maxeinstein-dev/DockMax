import { act, fireEvent, render, screen, within } from "@testing-library/react";
import type { ComponentProps } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ptBR } from "../i18n/pt-BR";
import type { GalleryDock } from "./StyleGallery";
import ThemeGallery from "./ThemeGallery";

const text = ptBR.settings.look;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const flush = () => act(() => Promise.resolve());

const DOCK: GalleryDock = {
  style: "native",
  tweaks: {},
  indicator: "dot",
  indicatorColor: null,
  iconPack: null,
};

function gallery(props: Partial<ComponentProps<typeof ThemeGallery>> = {}) {
  return <ThemeGallery dock={DOCK} appIds={[]} onApply={vi.fn()} {...props} />;
}

const group = () => screen.getByRole("radiogroup", { name: text.themes });
const card = (name: string) => within(group()).getByRole("radio", { name });

describe("ThemeGallery", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockImplementation(
      (command: string, args?: { pack: string | null; appIds: string[] }) =>
        Promise.resolve(
          command === "icon_pack_preview" && args
            ? args.appIds.map(
                (id) => `data:image/png;base64,${args.pack}-${id}`,
              )
            : undefined,
        ),
    );
  });

  it("offers the themes in order, the one in use chosen", () => {
    render(gallery());

    expect(
      within(group())
        .getAllByRole("radio")
        .map((button) => button.textContent),
    ).toEqual([
      "Padrão",
      "Neon",
      "Esmalte",
      "Vidro",
      "Meia-noite",
      "Retrô",
      "Clássico 98",
      "Fósforo",
      "Tático",
      "Medieval 8-bit",
      "Caçador",
    ]);
    expect(card("Padrão")).toHaveAttribute("aria-checked", "true");
    expect(screen.queryByText(text.themeCustom)).toBeNull();
  });

  it("applies the style, the icon pack and the mark of a theme at once", () => {
    const onApply = vi.fn();
    render(gallery({ onApply }));

    fireEvent.click(card("Vidro"));

    expect(onApply).toHaveBeenCalledWith({
      style: "liquidGlass",
      iconPack: "vidro",
      indicatorColor: null,
    });
  });

  it("shows the theme the dock is in", () => {
    render(gallery({ dock: { ...DOCK, style: "neon", iconPack: "neon" } }));

    expect(card("Neon")).toHaveAttribute("aria-checked", "true");
    expect(card("Padrão")).toHaveAttribute("aria-checked", "false");
  });

  it("chooses none, and says so, when a choice is the user's own", () => {
    render(gallery({ dock: { ...DOCK, style: "neon", iconPack: "argila" } }));

    for (const button of within(group()).getAllByRole("radio")) {
      expect(button).toHaveAttribute("aria-checked", "false");
    }
    expect(screen.getByText(text.themeCustom)).toBeInTheDocument();
  });

  it("draws each card's dock with the first pinned apps in the theme's icons", async () => {
    render(gallery({ appIds: ["a", "b"] }));
    await flush();
    await flush();

    expect(invoke).toHaveBeenCalledWith("icon_pack_preview", {
      pack: "esmalte",
      appIds: ["a", "b"],
      style: "native",
    });
    expect(
      [...card("Esmalte").querySelectorAll("img")].map((img) =>
        img.getAttribute("src"),
      ),
    ).toEqual([
      "data:image/png;base64,esmalte-a",
      "data:image/png;base64,esmalte-b",
    ]);
  });

  it("can be turned off", () => {
    render(gallery({ disabled: true }));

    for (const button of within(group()).getAllByRole("radio")) {
      expect(button).toBeDisabled();
    }
  });
  describe("trying a theme", () => {
    const MINE: GalleryDock = {
      ...DOCK,
      style: "floating",
      iconPack: null,
      indicatorColor: "#FFFFFF",
    };
    const BEFORE = {
      style: "floating",
      iconPack: null,
      indicatorColor: "#FFFFFF",
    };
    const keep = () => screen.queryByRole("group", { name: text.themeKeep });
    /** One second at a time: each tick renders before the next is set. */
    const seconds = (count: number) => {
      for (let second = 0; second < count; second++) {
        act(() => {
          vi.advanceTimersByTime(1000);
        });
      }
    };

    afterEach(() => {
      vi.useRealTimers();
    });

    it("asks to keep it, and keeping it changes nothing more", () => {
      const onApply = vi.fn();
      render(gallery({ dock: MINE, onApply }));

      fireEvent.click(card("Vidro"));
      expect(keep()).toBeInTheDocument();
      expect(screen.getByText("Volta ao anterior em 15 s")).toBeInTheDocument();

      fireEvent.click(
        screen.getByRole("button", { name: text.themeKeepButton }),
      );
      expect(keep()).toBeNull();
      expect(onApply).toHaveBeenCalledTimes(1);
    });

    it("goes back to the user's own choices", () => {
      const onApply = vi.fn();
      render(gallery({ dock: MINE, onApply }));

      fireEvent.click(card("Vidro"));
      fireEvent.click(
        screen.getByRole("button", { name: text.themeRevertButton }),
      );

      expect(onApply).toHaveBeenLastCalledWith(BEFORE);
      expect(keep()).toBeNull();
    });

    it("goes back by itself when the time runs out, counting down", () => {
      vi.useFakeTimers();
      const onApply = vi.fn();
      render(gallery({ dock: MINE, onApply }));

      fireEvent.click(card("Neon"));
      seconds(5);
      expect(screen.getByText("Volta ao anterior em 10 s")).toBeInTheDocument();
      expect(onApply).toHaveBeenCalledTimes(1);
      seconds(10);

      expect(onApply).toHaveBeenLastCalledWith(BEFORE);
      expect(onApply).toHaveBeenCalledTimes(2);
      expect(keep()).toBeNull();
    });

    it("tells a screen reader the question as it appears, 5 s before going back, and that it went back", () => {
      vi.useFakeTimers();
      render(gallery({ dock: MINE }));
      // Mounted with nothing to say: a region mounted with its text is
      // not read, so the same one says each of them.
      const live = screen.getByRole("status");
      expect(live).toBeEmptyDOMElement();

      fireEvent.click(card("Neon"));
      expect(screen.getByRole("status")).toBe(live);
      expect(live).toHaveTextContent(
        "Manter este tema? Se você não escolher Manter, ele volta ao anterior em 15 segundos.",
      );
      // The countdown on screen is not read second by second.
      expect(screen.getByText("Volta ao anterior em 15 s")).toHaveAttribute(
        "aria-hidden",
        "true",
      );

      seconds(9);
      expect(live).toHaveTextContent(/^Manter este tema\?/);
      seconds(1);
      expect(live).toHaveTextContent("O tema volta ao anterior em 5 segundos.");
      seconds(4);
      expect(keep()).toBeInTheDocument();
      seconds(1);
      expect(keep()).toBeNull();
      expect(screen.getByRole("status")).toBe(live);
      expect(live).toHaveTextContent("O tema voltou ao anterior.");

      // Another theme tried asks again.
      fireEvent.click(card("Vidro"));
      expect(live).toHaveTextContent(/^Manter este tema\?/);
    });

    it("has nothing more to say once the theme is kept", () => {
      render(gallery({ dock: MINE }));

      fireEvent.click(card("Neon"));
      fireEvent.click(
        screen.getByRole("button", { name: text.themeKeepButton }),
      );

      expect(screen.getByRole("status")).toBeEmptyDOMElement();
    });

    it("leaves the focus on the card, Manter the next Tab stop", () => {
      render(gallery({ dock: MINE }));

      card("Neon").focus();
      fireEvent.click(card("Neon"));

      expect(card("Neon")).toHaveFocus();
      const radios = within(group()).getAllByRole("radio");
      const keepButton = screen.getByRole("button", {
        name: text.themeKeepButton,
      });
      // Nothing that takes Tab between the cards and Manter.
      const stops = [
        ...document.querySelectorAll<HTMLElement>("button, [tabindex]"),
      ].filter((element) => element.tabIndex >= 0);
      expect(stops.indexOf(keepButton)).toBe(
        stops.indexOf(radios.find((radio) => radio.tabIndex === 0)!) + 1,
      );
    });

    it("goes back to before the first theme tried, and starts the time again", () => {
      vi.useFakeTimers();
      const onApply = vi.fn();
      const { rerender } = render(gallery({ dock: MINE, onApply }));

      fireEvent.click(card("Neon"));
      seconds(8);
      // The dock now shows Neon; the user tries another one.
      rerender(
        gallery({
          dock: {
            ...MINE,
            style: "neon",
            iconPack: "neon",
            indicatorColor: null,
          },
          onApply,
        }),
      );
      fireEvent.click(card("Vidro"));
      expect(screen.getByText("Volta ao anterior em 15 s")).toBeInTheDocument();
      fireEvent.click(
        screen.getByRole("button", { name: text.themeRevertButton }),
      );

      expect(onApply).toHaveBeenLastCalledWith(BEFORE);
    });
  });
});
