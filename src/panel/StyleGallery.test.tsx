import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_LOOK, type DockSettings } from "../core/settings";
import { forgetIcons } from "../core/apps";
import StyleGallery from "./StyleGallery";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

const DOCK: DockSettings = {
  edge: "bottom",
  iconSize: 48,
  opacity: "translucent",
  ...DEFAULT_LOOK,
};

const gallery = () => screen.getByRole("group", { name: "Tema" });
const card = (name: string) => within(gallery()).getByRole("button", { name });
const bar = (name: string) =>
  card(name).querySelector<HTMLElement>(".mini-bar") as HTMLElement;

describe("StyleGallery", () => {
  beforeEach(() => {
    forgetIcons();
    invoke.mockReset();
    invoke.mockImplementation((_command: string, args: { appId: string }) =>
      Promise.resolve(`data:image/png;base64,${args.appId}`),
    );
  });

  it("draws every style with the user's programs, the first five", async () => {
    render(
      <StyleGallery
        label="Tema"
        dock={{ ...DOCK, style: "neon" }}
        appIds={["a", "b", "c", "d", "e", "f"]}
        onSelect={vi.fn()}
      />,
    );
    await act(async () => {});

    expect(within(gallery()).getAllByRole("button")).toHaveLength(9);
    expect(card("Neon")).toHaveAttribute("aria-pressed", "true");
    expect(card("Papel")).toHaveAttribute("aria-pressed", "false");
    const images = [...card("Papel").querySelectorAll("img")];
    // Decorative: the card's name says which style it is.
    expect(images.map((img) => img.getAttribute("alt"))).toEqual([
      "",
      "",
      "",
      "",
      "",
    ]);
    expect(images.map((img) => img.getAttribute("src"))).toEqual(
      ["a", "b", "c", "d", "e"].map((id) => `data:image/png;base64,${id}`),
    );
  });

  it("shows widget tiles when no program is pinned", () => {
    render(
      <StyleGallery label="Tema" dock={DOCK} appIds={[]} onSelect={vi.fn()} />,
    );

    expect(card("Rocket").querySelectorAll(".mini-tile")).toHaveLength(3);
    expect(invoke).not.toHaveBeenCalled();
  });

  it("paints each style through custom properties, with the user's changes", () => {
    render(
      <StyleGallery
        label="Tema"
        dock={{
          ...DOCK,
          indicator: "dash",
          tweaks: { midnight: { corners: 100, reflections: true } },
        }}
        appIds={[]}
        onSelect={vi.fn()}
      />,
    );

    // Meia-noite: a pill (14 px is half the bar), with reflections.
    expect(bar("Meia-noite").style.getPropertyValue("--mini-radius")).toBe(
      "14px",
    );
    expect(bar("Meia-noite")).toHaveAttribute("data-reflections");
    expect(bar("Meia-noite").style.getPropertyValue("--mini-fill")).toBe(
      "rgb(8 10 16 / 0.93)",
    );
    // Rocket keeps its own 25 %, and its glow.
    expect(bar("Rocket").style.getPropertyValue("--mini-radius")).toBe("3.5px");
    expect(bar("Rocket").style.getPropertyValue("--mini-glow")).not.toBe("");
    // Nativo: the theme's own bar.
    expect(bar("Nativo").style.getPropertyValue("--mini-fill")).toBe("");
    expect(bar("Neon")).toHaveAttribute("data-outline");
    expect(card("Papel").querySelector(".mini-indicator")).toHaveAttribute(
      "data-shape",
      "dash",
    );
    expect(card("Nativo").getAttribute("style")).toBeNull();
  });

  it("colors the mark as the user chose, else as the style does", () => {
    const { rerender } = render(
      <StyleGallery label="Tema" dock={DOCK} appIds={[]} onSelect={vi.fn()} />,
    );
    expect(bar("Neon").style.getPropertyValue("--mini-indicator")).toBe(
      "rgb(0 229 255 / 1)",
    );
    expect(bar("Nativo").style.getPropertyValue("--mini-indicator")).toBe("");

    rerender(
      <StyleGallery
        label="Tema"
        dock={{ ...DOCK, indicatorColor: "#FF453A" }}
        appIds={[]}
        onSelect={vi.fn()}
      />,
    );
    expect(bar("Neon").style.getPropertyValue("--mini-indicator")).toBe(
      "#FF453A",
    );
  });

  it("chooses with a click, and moves with the arrows without choosing", () => {
    const onSelect = vi.fn();
    render(
      <StyleGallery label="Tema" dock={DOCK} appIds={[]} onSelect={onSelect} />,
    );

    expect(card("Nativo")).toHaveAttribute("tabindex", "0");
    expect(card("Neon")).toHaveAttribute("tabindex", "-1");
    card("Nativo").focus();
    fireEvent.keyDown(card("Nativo"), { key: "ArrowLeft" });
    expect(card("Flutuante")).toHaveFocus();
    fireEvent.keyDown(card("Flutuante"), { key: "Home" });
    expect(card("Nativo")).toHaveFocus();
    expect(onSelect).not.toHaveBeenCalled();

    fireEvent.click(card("Canarinho"));
    expect(onSelect).toHaveBeenCalledWith("canary");
    fireEvent.click(card("Nativo"));
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("can't choose while the settings can't be read", () => {
    render(
      <StyleGallery
        label="Tema"
        dock={null}
        appIds={[]}
        disabled
        onSelect={vi.fn()}
      />,
    );

    for (const each of within(gallery()).getAllByRole("button")) {
      expect(each).toBeDisabled();
      expect(each).toHaveAttribute("aria-pressed", "false");
    }
  });
});
