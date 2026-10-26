import { act, fireEvent, render, screen, within } from "@testing-library/react";
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

function answer(packs: IconPackInfo[]) {
  invoke.mockImplementation((command: string) =>
    command === "list_icon_packs" || command === "reload_icon_packs"
      ? Promise.resolve(packs)
      : Promise.resolve(undefined),
  );
}

const list = () => screen.getByRole("combobox", { name: text.iconPack });
const labels = () =>
  within(list())
    .getAllByRole("option")
    .map((option) => option.textContent);

describe("IconPacksField", () => {
  beforeEach(() => {
    invoke.mockReset();
    log.warn.mockReset();
  });

  it("offers the original icons first and a pack with what it covers", async () => {
    answer([pack({})]);

    render(<IconPacksField selected={null} onSelect={vi.fn()} />);
    await flush();

    expect(labels()).toEqual(["Original", "Cores lisas · veste 3 de 5 apps"]);
    expect(list()).toHaveValue("");
  });

  it("counts the icons of a pack while no app is pinned", async () => {
    answer([pack({ total: 0, covers: 0 })]);

    render(<IconPacksField selected={null} onSelect={vi.fn()} />);
    await flush();

    expect(labels()).toContain("Cores lisas · 12 ícones");
  });

  it("shows the chosen pack as chosen", async () => {
    answer([pack({}), pack({ id: "outro", name: "Outro" })]);

    render(<IconPacksField selected="outro" onSelect={vi.fn()} />);
    await flush();

    expect(list()).toHaveValue("outro");
  });

  it("chooses a pack, and the original with null", async () => {
    answer([pack({})]);
    const onSelect = vi.fn();
    const { rerender } = render(
      <IconPacksField selected={null} onSelect={onSelect} />,
    );
    await flush();

    fireEvent.change(list(), { target: { value: "cores" } });
    expect(onSelect).toHaveBeenLastCalledWith("cores");

    rerender(<IconPacksField selected="cores" onSelect={onSelect} />);
    fireEvent.change(list(), { target: { value: "" } });
    expect(onSelect).toHaveBeenLastCalledWith(null);
  });

  it("keeps a chosen pack that is gone in the list, as unavailable", async () => {
    answer([pack({})]);

    render(<IconPacksField selected="sumiu" onSelect={vi.fn()} />);
    expect(labels()).toContain("sumiu");
    await flush();

    expect(labels()).toContain("sumiu (indisponível)");
    expect(list()).toHaveValue("sumiu");
  });

  it("names the packs that cannot be used and leaves them out of the list", async () => {
    answer([
      pack({}),
      pack({ id: "quebrado", name: "quebrado", error: "manifestInvalid" }),
    ]);

    render(<IconPacksField selected={null} onSelect={vi.fn()} />);
    await flush();

    expect(labels()).not.toContain("quebrado");
    expect(
      screen.getByText("Pacote quebrado: o pack.json não é válido"),
    ).toBeInTheDocument();
  });

  it("reads the folder again with the reload button", async () => {
    answer([pack({})]);
    render(<IconPacksField selected={null} onSelect={vi.fn()} />);
    await flush();
    answer([pack({}), pack({ id: "novo", name: "Novo" })]);

    fireEvent.click(screen.getByRole("button", { name: text.reloadPacks }));
    await flush();

    expect(invoke).toHaveBeenCalledWith("reload_icon_packs");
    expect(labels()).toContain("Novo · veste 3 de 5 apps");
  });

  it("opens the packs folder", async () => {
    answer([]);
    render(<IconPacksField selected={null} onSelect={vi.fn()} />);
    await flush();

    fireEvent.click(screen.getByRole("button", { name: text.openPacksFolder }));

    expect(invoke).toHaveBeenCalledWith("open_icon_packs_folder");
  });

  it("says when the packs cannot be read, and keeps the original", async () => {
    invoke.mockRejectedValue(new Error("boom"));

    render(<IconPacksField selected={null} onSelect={vi.fn()} />);
    await flush();

    expect(screen.getByText(text.iconPackFailed)).toBeInTheDocument();
    expect(labels()).toEqual(["Original"]);
    expect(log.warn).toHaveBeenCalled();
  });

  it("can be turned off", async () => {
    answer([pack({})]);

    render(<IconPacksField selected={null} disabled onSelect={vi.fn()} />);
    await flush();

    expect(list()).toBeDisabled();
  });
});
