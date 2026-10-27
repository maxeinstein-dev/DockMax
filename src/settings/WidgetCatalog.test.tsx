import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  CREATABLE_KINDS,
  MAX_WIDGETS,
  type WidgetInstance,
  type WidgetsSnapshot,
} from "../core/widgets";
import type { Workspace } from "../core/workspaces";
import { ptBR } from "../i18n/pt-BR";
import WidgetCatalog from "./WidgetCatalog";

const text = ptBR.widgets.catalog;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const NOTE: WidgetInstance = {
  id: "n1",
  label: null,
  kind: "note",
  config: {},
};

function snapshot(widgets: WidgetInstance[]): WidgetsSnapshot {
  return { storage: { state: "ok" }, widgets };
}

const WORK = {
  id: "w1",
  name: "Trabalho",
  color: "blue",
  items: ["widget:n1"],
} satisfies Workspace;

function addButton(name: string) {
  return screen.getByRole("button", {
    name: `${text.add} ${name}`,
  });
}

beforeEach(() => {
  invoke.mockReset();
});

describe("WidgetCatalog", () => {
  it("shows every kind that can be created, on its shelf", () => {
    render(<WidgetCatalog snapshot={snapshot([NOTE])} workspace={WORK} />);

    for (const kind of CREATABLE_KINDS) {
      expect(addButton(ptBR.widgets.kinds[kind])).toBeEnabled();
    }
    for (const shelf of Object.values(text.categories)) {
      expect(screen.getByRole("heading", { name: shelf })).toBeInTheDocument();
    }
  });

  it("narrows the shelves by name or description, without accents", () => {
    render(<WidgetCatalog snapshot={snapshot([])} workspace={WORK} />);

    fireEvent.change(screen.getByRole("searchbox", { name: text.search }), {
      target: { value: "relogio" },
    });
    expect(addButton("Relógio")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Nota/ })).toBeNull();
    expect(
      screen.queryByRole("heading", { name: text.categories.system }),
    ).toBeNull();

    fireEvent.change(screen.getByRole("searchbox"), {
      target: { value: "upload" },
    });
    expect(addButton("Atividade de rede")).toBeInTheDocument();

    fireEvent.change(screen.getByRole("searchbox"), {
      target: { value: "nada disso" },
    });
    expect(screen.getByText(text.none)).toBeInTheDocument();
  });

  it("adds to the workspace being edited and says so", async () => {
    invoke.mockResolvedValue({ ...NOTE, id: "a1", kind: "activity" });
    render(<WidgetCatalog snapshot={snapshot([])} workspace={WORK} />);

    fireEvent.click(addButton("Atividade"));

    expect(invoke).toHaveBeenCalledWith("add_widget", {
      kind: "activity",
      workspace: "w1",
    });
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "Atividade foi para o dock.",
      ),
    );
  });

  it("shows the message of a refusal", async () => {
    invoke.mockRejectedValue({ code: "limitReached" });
    render(<WidgetCatalog snapshot={snapshot([])} workspace={WORK} />);

    fireEvent.click(addButton("Relógio"));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      ptBR.widgets.errors.limitReached,
    );
  });

  it("turns the add buttons off at the limit", () => {
    const many = Array.from({ length: MAX_WIDGETS }, (_, index) => ({
      ...NOTE,
      id: `n${index}`,
    }));
    const full: Workspace = {
      ...WORK,
      items: many.map((instance) => `widget:${instance.id}`),
    };
    render(<WidgetCatalog snapshot={snapshot(many)} workspace={full} />);

    expect(addButton("Nota")).toBeDisabled();
  });

  it("is not shown while the widgets can't be used", () => {
    const { container } = render(
      <WidgetCatalog
        snapshot={{
          storage: { state: "unavailable", reason: "openFailed" },
          widgets: [],
        }}
        workspace={WORK}
      />,
    );

    expect(container).toBeEmptyDOMElement();
  });
});
