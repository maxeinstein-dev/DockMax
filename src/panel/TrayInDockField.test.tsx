import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { TrayIcon } from "../core/tray";
import { ptBR } from "../i18n/pt-BR";
import TrayInDockField from "./TrayInDockField";

const text = ptBR.tray.inDock;

const { invoke, listen } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const icon = (key: string, tip: string, id: string | null): TrayIcon => ({
  key,
  id,
  tip,
  image: null,
  hidden: false,
  clickable: true,
});

const DOCKER = icon("1:1", "Docker Desktop", "path:c:\\docker.exe#1");
const FORTI = icon("2:1", "FortiClient", "path:c:\\forti.exe#129");

async function renderField(
  icons: TrayIcon[],
  inDock: string[],
  onChange = vi.fn(),
) {
  invoke.mockImplementation((command: string) =>
    Promise.resolve(command === "list_tray" ? icons : undefined),
  );
  render(<TrayInDockField inDock={inDock} onChange={onChange} />);
  await act(async () => {});
  return onChange;
}

const row = (name: string) => screen.getByRole("group", { name });

describe("TrayInDockField", () => {
  beforeEach(() => invoke.mockReset());

  it("offers Painel or Dock for each tray icon, the dock's chosen", async () => {
    await renderField([DOCKER, FORTI], [FORTI.id ?? ""]);

    expect(
      within(row("Docker Desktop")).getByRole("button", { name: text.panel }),
    ).toHaveAttribute("aria-pressed", "true");
    expect(
      within(row("FortiClient")).getByRole("button", { name: text.dock }),
    ).toHaveAttribute("aria-pressed", "true");
  });

  it("puts an icon in the dock, at the end, and takes it out", async () => {
    const onChange = await renderField([DOCKER, FORTI], [FORTI.id ?? ""]);

    fireEvent.click(
      within(row("Docker Desktop")).getByRole("button", { name: text.dock }),
    );
    fireEvent.click(
      within(row("FortiClient")).getByRole("button", { name: text.panel }),
    );

    expect(onChange.mock.calls).toEqual([[[FORTI.id, DOCKER.id]], [[]]]);
  });

  it("refuses a seventh icon in the dock", async () => {
    const six = Array.from({ length: 6 }, (_, n) => `path:c:\\app${n}.exe#1`);
    const onChange = await renderField([DOCKER], six);

    fireEvent.click(
      within(row("Docker Desktop")).getByRole("button", { name: text.dock }),
    );

    expect(onChange).not.toHaveBeenCalled();
  });

  it("leaves out the icons without a lasting name, and says when there is none", async () => {
    await renderField([icon("3:1", "Sem arquivo", null)], []);

    expect(screen.queryByRole("group")).toBeNull();
    expect(screen.getByText(text.none)).toBeInTheDocument();
  });
});
