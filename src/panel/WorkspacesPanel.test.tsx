import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  inWorkspace,
  isValidWorkspaceName,
  type Workspace,
  type WorkspacesSnapshot,
} from "../core/workspaces";
import { ptBR } from "../i18n/pt-BR";
import WorkspacePicker from "./WorkspacePicker";
import WorkspacesPanel from "./WorkspacesPanel";

const text = ptBR.workspaces;

const { invoke, log } = vi.hoisted(() => ({
  invoke: vi.fn(),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log,
}));

const MAIN: Workspace = {
  id: "w-main",
  name: "Principal",
  color: "#0A84FF",
  items: ["app:a1", "widget:c1", "app:a2"],
};
const WORK: Workspace = {
  id: "w-work",
  name: "Trabalho",
  color: "#FF9F0A",
  items: ["widget:c1"],
};
const SNAPSHOT: WorkspacesSnapshot = {
  workspaces: [MAIN, WORK],
  inUse: "w-main",
};

/** Lets settled promises run their callbacks. */
const flush = () => act(() => Promise.resolve());

function renderPanel(
  selected: Workspace = MAIN,
  snapshot: WorkspacesSnapshot = SNAPSHOT,
) {
  const onSelect = vi.fn();
  const onEditApps = vi.fn();
  render(
    <WorkspacesPanel
      snapshot={snapshot}
      selected={selected}
      onSelect={onSelect}
      onEditApps={onEditApps}
      onEditItems={vi.fn()}
    />,
  );
  return { onSelect, onEditApps };
}

describe("workspaces", () => {
  beforeEach(() => {
    invoke.mockReset();
    log.warn.mockReset();
  });

  it("show only what a workspace has, in its order", () => {
    const apps = [{ id: "a1" }, { id: "a2" }, { id: "a3" }];
    expect(inWorkspace(apps, MAIN, "app")).toEqual([
      { id: "a1" },
      { id: "a2" },
    ]);
    expect(inWorkspace(apps, WORK, "app")).toEqual([]);
    // Without workspaces (they can't be read): everything.
    expect(inWorkspace(apps, null, "app")).toEqual(apps);
  });

  it("take names of one short line", () => {
    expect(isValidWorkspaceName("Foco")).toBe(true);
    expect(isValidWorkspaceName("")).toBe(false);
    expect(isValidWorkspaceName(" Foco")).toBe(false);
    expect(isValidWorkspaceName("a".repeat(33))).toBe(false);
  });

  it("lists every workspace with its shortcut, the edited one pressed", () => {
    renderPanel();
    const list = screen.getByRole("group", { name: text.listLabel });
    const chips = within(list).getAllByRole("button");
    expect(chips.map((chip) => chip.textContent)).toEqual([
      "PrincipalCtrl+Alt+1",
      "TrabalhoCtrl+Alt+2",
      text.create,
    ]);
    expect(chips[0]).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("2 apps e 1 widgets")).toBeInTheDocument();
  });

  it("chooses another workspace to edit without switching any dock", () => {
    const { onSelect } = renderPanel();
    fireEvent.click(screen.getByRole("button", { name: /Trabalho/ }));
    expect(onSelect).toHaveBeenCalledWith("w-work");
    expect(invoke).not.toHaveBeenCalled();
  });

  it("makes a new workspace and edits it", async () => {
    invoke.mockResolvedValue({ ...WORK, id: "w-new", name: "Espaço 3" });
    const { onSelect } = renderPanel();
    fireEvent.click(screen.getByRole("button", { name: text.create }));
    await flush();
    expect(invoke).toHaveBeenCalledWith("create_workspace", {});
    expect(onSelect).toHaveBeenCalledWith("w-new");
  });

  it("renames when the name is done, and not with an invalid one", async () => {
    invoke.mockResolvedValue(undefined);
    renderPanel();
    const field = screen.getByRole("textbox", { name: text.name });

    fireEvent.change(field, { target: { value: " " } });
    fireEvent.blur(field);
    expect(invoke).not.toHaveBeenCalled();
    expect(screen.getByText(text.errors.invalidName)).toBeInTheDocument();

    fireEvent.change(field, { target: { value: "Casa" } });
    fireEvent.blur(field);
    await flush();
    expect(invoke).toHaveBeenCalledWith("rename_workspace", {
      id: "w-main",
      name: "Casa",
    });
  });

  it("recolors with a swatch of the palette", () => {
    invoke.mockResolvedValue(undefined);
    renderPanel();
    fireEvent.click(
      screen.getByRole("radio", { name: text.colors["#30D158"] }),
    );
    expect(invoke).toHaveBeenCalledWith("recolor_workspace", {
      id: "w-main",
      color: "#30D158",
    });
  });

  it("deletes after a confirmation, then edits another one", async () => {
    invoke.mockResolvedValue(undefined);
    const { onSelect } = renderPanel();
    fireEvent.click(
      screen.getByRole("button", {
        name: `Excluir ${MAIN.name}`,
      }),
    );
    expect(invoke).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: text.confirm }));
    await flush();
    expect(invoke).toHaveBeenCalledWith("delete_workspace", { id: "w-main" });
    expect(onSelect).toHaveBeenCalledWith("w-work");
  });

  it("can't delete the last workspace", () => {
    renderPanel(MAIN, { workspaces: [MAIN], inUse: "w-main" });
    expect(
      screen.getByRole("button", { name: `Excluir ${MAIN.name}` }),
    ).toBeDisabled();
  });

  it("says why a change was refused", async () => {
    invoke.mockRejectedValue({ code: "limitReached" });
    renderPanel();
    fireEvent.click(screen.getByRole("button", { name: text.create }));
    await flush();
    expect(screen.getByText(text.errors.limitReached)).toBeInTheDocument();
  });

  it("opens the programs of the workspace being edited", () => {
    const { onEditApps } = renderPanel(WORK);
    fireEvent.click(screen.getByRole("button", { name: text.editApps }));
    expect(onEditApps).toHaveBeenCalled();
  });

  it("offers a choice above the lists only with two or more", () => {
    const onSelect = vi.fn();
    const { rerender } = render(
      <WorkspacePicker
        workspaces={[MAIN]}
        selected={MAIN}
        onSelect={onSelect}
      />,
    );
    expect(screen.queryByRole("group", { name: text.editing })).toBeNull();

    rerender(
      <WorkspacePicker
        workspaces={[MAIN, WORK]}
        selected={WORK}
        onSelect={onSelect}
      />,
    );
    const group = screen.getByRole("group", { name: text.editing });
    expect(
      within(group).getByRole("button", { name: "Trabalho" }),
    ).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(within(group).getByRole("button", { name: "Principal" }));
    expect(onSelect).toHaveBeenCalledWith("w-main");
  });
});
