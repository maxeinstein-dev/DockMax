import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { version } from "../../package.json";
import type { UpdateState } from "../core/update";
import { ptBR } from "../i18n/pt-BR";
import AboutPanel from "./AboutPanel";
import { expectSaidIn, liveRegions } from "../test/alerts";

type Listener = (event: { payload: unknown }) => void;

const { invoke, listeners } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listeners: new Map<string, Listener>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((event: string, handler: Listener) => {
    listeners.set(event, handler);
    return Promise.resolve(() => {});
  }),
}));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

const text = ptBR.about.update;
const NONE: UpdateState = { state: "none" };
const AVAILABLE: UpdateState = {
  state: "available",
  version: "0.5.0",
  notes: null,
};

/** What Rust answers: `status` for `update_status`, the rest by command. */
function answer(
  status: UpdateState,
  others: Record<string, () => Promise<unknown>> = {},
) {
  invoke.mockImplementation((command: string) => {
    if (command === "update_status") return Promise.resolve(status);
    const other = others[command];
    return other ? other() : new Promise(() => {});
  });
}

async function renderAbout() {
  const view = render(<AboutPanel />);
  await act(async () => {});
  return view;
}

beforeEach(() => {
  listeners.clear();
  invoke.mockReset();
  answer(NONE);
});

/** The value next to a label of the key/value list. */
function value(label: string) {
  return screen.getByText(label).nextElementSibling;
}

describe("AboutPanel", () => {
  it("shows the app name and description", () => {
    render(<AboutPanel />);

    expect(
      screen.getByRole("heading", { name: "Dockyard" }),
    ).toBeInTheDocument();
    expect(screen.getByText(ptBR.about.description)).toBeInTheDocument();
  });

  it("shows the version of package.json", () => {
    render(<AboutPanel />);

    expect(version).toMatch(/^\d+\.\d+\.\d+/);
    expect(value("Versão")).toHaveTextContent(version);
  });

  it("shows the personal-use license", () => {
    render(<AboutPanel />);

    expect(value("Licença")).toHaveTextContent("Uso pessoal");
  });

  it("shows the repository as selectable text, not a link", () => {
    render(<AboutPanel />);

    const code = value("Código");
    expect(code).toHaveTextContent("github.com/maxeinstein-dev/Dockyard");
    expect(code).toHaveClass("about-code");
    // The panel can't open the browser (no opener plugin): no link.
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
  });

  it("has the footer with the close hint", () => {
    render(<AboutPanel />);

    expect(screen.getByText("Projeto independente")).toBeInTheDocument();
    expect(screen.getByText("Esc").tagName).toBe("KBD");
    expect(screen.getByText(/fecha/)).toBeInTheDocument();
  });

  describe("updates (spec 0019 G3)", () => {
    it("without a new version, offers to check and says it is current", async () => {
      answer(NONE, { check_for_update: () => Promise.resolve(NONE) });
      await renderAbout();

      expect(
        screen.queryByRole("button", { name: text.install }),
      ).not.toBeInTheDocument();
      await act(async () => {
        fireEvent.click(screen.getByRole("button", { name: text.check }));
      });

      expect(invoke).toHaveBeenCalledWith("check_for_update");
      expect(screen.getByRole("status")).toHaveTextContent(text.upToDate);
    });

    it("says why it could not check", async () => {
      answer(NONE, {
        check_for_update: () => Promise.reject("no network"),
      });
      await renderAbout();
      const before = liveRegions();

      await act(async () => {
        fireEvent.click(screen.getByRole("button", { name: text.check }));
      });

      expect(screen.getByRole("status")).toHaveTextContent(
        "Não deu para procurar atualizações: no network",
      );
      expectSaidIn(before, screen.getByRole("status"));
    });

    it("shows a version found by the check, with Atualizar", async () => {
      answer(NONE, { check_for_update: () => Promise.resolve(AVAILABLE) });
      await renderAbout();

      await act(async () => {
        fireEvent.click(screen.getByRole("button", { name: text.check }));
      });

      expect(screen.getByText("Versão nova: 0.5.0")).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: text.install }),
      ).toBeInTheDocument();
    });

    it("with a new version, shows it and installs only on Atualizar", async () => {
      answer(AVAILABLE);
      await renderAbout();

      expect(screen.getByText("Versão nova: 0.5.0")).toBeInTheDocument();
      expect(
        screen.queryByRole("button", { name: text.check }),
      ).not.toBeInTheDocument();
      expect(invoke).not.toHaveBeenCalledWith("install_update");

      await act(async () => {
        fireEvent.click(screen.getByRole("button", { name: text.install }));
      });

      expect(invoke).toHaveBeenCalledWith("install_update");
    });

    it("says why the update failed", async () => {
      answer(AVAILABLE, {
        install_update: () => Promise.reject("bad signature"),
      });
      await renderAbout();
      const before = liveRegions();

      await act(async () => {
        fireEvent.click(screen.getByRole("button", { name: text.install }));
      });

      expect(screen.getByRole("status")).toHaveTextContent(
        "Não deu para atualizar: bad signature",
      );
      expectSaidIn(before, screen.getByRole("status"));
    });

    it("follows the download Rust reports", async () => {
      await renderAbout();
      const before = liveRegions();

      act(() =>
        listeners.get("update-changed")?.({
          payload: {
            state: "downloading",
            version: "0.5.0",
            downloaded: 40,
            total: 100,
          },
        }),
      );

      expect(screen.getByRole("status")).toHaveTextContent(
        "Baixando a versão 0.5.0… 40%",
      );
      expect(
        screen.getByRole("progressbar", { name: text.progress }),
      ).toHaveAttribute("value", "40");

      act(() =>
        listeners.get("update-changed")?.({
          payload: { state: "installing", version: "0.5.0" },
        }),
      );

      expect(screen.getByRole("status")).toHaveTextContent(
        "Instalando a versão 0.5.0",
      );
      expectSaidIn(before, screen.getByRole("status"));
      expect(screen.queryByRole("button", { name: text.install })).toBeNull();
    });
  });
});
