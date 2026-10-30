import { act, fireEvent, render, screen, within } from "@testing-library/react";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { IconPackInfo } from "../core/iconPacks";
import { ptBR } from "../i18n/pt-BR";
import IconPacksField from "./IconPacksField";

const text = ptBR.settings.look;

const { invoke, log } = vi.hoisted(() => ({
  invoke: vi.fn(),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log,
}));

const flush = () => act(() => Promise.resolve());

function pack(change: Partial<IconPackInfo>): IconPackInfo {
  return {
    id: "cores",
    name: "Cores lisas",
    author: "Fulano",
    version: "1.0.0",
    description: "",
    accent: null,
    iconCount: 12,
    covers: 3,
    total: 5,
    error: null,
    ...change,
  };
}

/** The packs the folder has; each preview is one URL per program, named
 * after the pack. */
function answer(packs: IconPackInfo[]) {
  invoke.mockImplementation(
    (command: string, args?: { pack: string | null; appIds: string[] }) => {
      if (command === "list_icon_packs" || command === "reload_icon_packs") {
        return Promise.resolve(packs);
      }
      if (command === "icon_pack_preview" && args) {
        return Promise.resolve(
          args.appIds.map(
            (id) => `data:image/png;base64,${args.pack ?? "own"}-${id}`,
          ),
        );
      }
      return Promise.resolve(undefined);
    },
  );
}

function field(props: Partial<ComponentProps<typeof IconPacksField>> = {}) {
  return (
    <IconPacksField
      selected={null}
      dock={null}
      appIds={[]}
      onSelect={vi.fn()}
      {...props}
    />
  );
}

const group = () => screen.getByRole("group", { name: text.iconPack });
const cards = () => within(group()).getAllByRole("button");
const names = () =>
  cards().map(
    (card) => card.querySelector(".choice-card-name")?.textContent ?? "",
  );
const card = (name: string) => within(group()).getByRole("button", { name });

describe("IconPacksField", () => {
  beforeEach(() => {
    invoke.mockReset();
    log.warn.mockReset();
  });

  it("offers the original icons first and a card per pack, with what it covers", async () => {
    answer([pack({})]);

    render(field());
    await flush();

    expect(names()).toEqual(["Original", "Cores lisas"]);
    expect(card("Cores lisas")).toHaveAccessibleDescription(
      "Veste 3 de 5 apps",
    );
    expect(card("Original")).toHaveAttribute("aria-pressed", "true");
  });

  it("counts the icons of a pack while no app is pinned", async () => {
    answer([pack({ total: 0, covers: 0 })]);

    render(field());
    await flush();

    expect(card("Cores lisas")).toHaveAccessibleDescription("12 ícones");
  });

  it("draws each card's dock with the first pinned apps in its icons", async () => {
    answer([pack({})]);
    const appIds = ["a", "b", "c", "d", "e", "f", "g"];

    render(field({ appIds }));
    await flush();
    await flush();

    expect(invoke).toHaveBeenCalledWith("icon_pack_preview", {
      pack: "cores",
      appIds: ["a", "b", "c", "d", "e"],
      style: null,
    });
    expect(invoke).toHaveBeenCalledWith("icon_pack_preview", {
      pack: null,
      appIds: ["a", "b", "c", "d", "e"],
      style: null,
    });
    const pictures = card("Cores lisas").querySelectorAll("img");
    expect([...pictures].map((img) => img.getAttribute("src"))).toEqual(
      ["a", "b", "c", "d", "e"].map(
        (id) => `data:image/png;base64,cores-${id}`,
      ),
    );
  });

  it("asks no preview without pinned apps", async () => {
    answer([pack({})]);

    render(field());
    await flush();

    expect(invoke).not.toHaveBeenCalledWith(
      "icon_pack_preview",
      expect.anything(),
    );
  });

  it("shows the chosen pack as chosen", async () => {
    answer([pack({}), pack({ id: "outro", name: "Outro" })]);

    render(field({ selected: "outro" }));
    await flush();

    expect(card("Outro")).toHaveAttribute("aria-pressed", "true");
    expect(card("Original")).toHaveAttribute("aria-pressed", "false");
  });

  it("chooses a pack, and the original with null", async () => {
    answer([pack({})]);
    const onSelect = vi.fn();
    const { rerender } = render(field({ onSelect }));
    await flush();

    fireEvent.click(card("Cores lisas"));
    expect(onSelect).toHaveBeenLastCalledWith("cores");

    rerender(field({ selected: "cores", onSelect }));
    fireEvent.click(card("Original"));
    expect(onSelect).toHaveBeenLastCalledWith(null);
  });

  it("keeps a chosen pack that is gone, as unavailable", async () => {
    answer([pack({})]);

    render(field({ selected: "sumiu" }));
    expect(card("sumiu")).toHaveAccessibleDescription(text.iconPackLoading);
    await flush();

    expect(card("sumiu")).toHaveAccessibleDescription("Indisponível");
    expect(card("sumiu")).toHaveAttribute("aria-pressed", "true");
  });

  it("names the packs that cannot be used and leaves them out of the cards", async () => {
    answer([
      pack({}),
      pack({ id: "quebrado", name: "quebrado", error: "manifestInvalid" }),
    ]);

    render(field());
    await flush();

    expect(names()).not.toContain("quebrado");
    expect(
      screen.getByText("Pacote quebrado: o pack.json não é válido"),
    ).toBeInTheDocument();
  });

  it("reads the folder again with the reload button, and the previews too", async () => {
    answer([pack({})]);
    render(field({ appIds: ["a"] }));
    await flush();
    await flush();
    answer([pack({}), pack({ id: "novo", name: "Novo" })]);
    invoke.mockClear();

    fireEvent.click(screen.getByRole("button", { name: text.reloadPacks }));
    await flush();
    await flush();

    expect(invoke).toHaveBeenCalledWith("reload_icon_packs");
    expect(names()).toContain("Novo");
    expect(invoke).toHaveBeenCalledWith("icon_pack_preview", {
      pack: "cores",
      appIds: ["a"],
      style: null,
    });
  });

  it("opens the packs folder", async () => {
    answer([]);
    render(field());
    await flush();

    fireEvent.click(screen.getByRole("button", { name: text.openPacksFolder }));

    expect(invoke).toHaveBeenCalledWith("open_icon_packs_folder");
  });

  it("says when the packs cannot be read, and keeps the original", async () => {
    invoke.mockRejectedValue(new Error("boom"));

    render(field());
    await flush();

    expect(screen.getByText(text.iconPackFailed)).toBeInTheDocument();
    expect(names()).toEqual(["Original"]);
    expect(log.warn).toHaveBeenCalled();
  });

  it("can be turned off", async () => {
    answer([pack({})]);

    render(field({ disabled: true }));
    await flush();

    for (const button of cards()) expect(button).toBeDisabled();
  });
});
