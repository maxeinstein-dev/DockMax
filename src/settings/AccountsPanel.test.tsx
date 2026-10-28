import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ptBR } from "../i18n/pt-BR";
import AccountsPanel from "./AccountsPanel";
import css from "./AccountsPanel.css?raw";

const text = ptBR.accounts;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: () => Promise.resolve(() => undefined),
}));

/** Answers each command as Rust would, from `answers`. */
function answer(answers: Record<string, unknown>) {
  invoke.mockImplementation((command: string) => {
    const value = answers[command];
    if (value instanceof Error) return Promise.reject(value.message);
    return Promise.resolve(value);
  });
}

beforeEach(() => {
  invoke.mockReset();
});

describe("AccountsPanel", () => {
  it("asks for the Google app first, and can't connect without it", async () => {
    answer({ google_status: { clientId: null, accounts: [] } });
    render(<AccountsPanel />);

    expect(await screen.findByText(text.client.intro)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: text.list.connect }),
    ).toBeDisabled();
    expect(screen.getByRole("status")).toHaveTextContent(text.list.needsClient);
  });

  it("sends the client once, with the secret in a password field", async () => {
    answer({
      google_status: { clientId: null, accounts: [] },
      set_google_client: undefined,
    });
    render(<AccountsPanel />);

    const secret = await screen.findByLabelText(text.client.secret);
    expect(secret).toHaveAttribute("type", "password");
    fireEvent.change(screen.getByLabelText(text.client.id), {
      target: { value: "1.apps.googleusercontent.com" },
    });
    fireEvent.change(secret, { target: { value: "GOCSPX-x" } });
    fireEvent.click(screen.getByRole("button", { name: text.client.save }));

    expect(invoke).toHaveBeenCalledWith("set_google_client", {
      id: "1.apps.googleusercontent.com",
      secret: "GOCSPX-x",
    });
  });

  it("shows why a client was refused", async () => {
    answer({
      google_status: { clientId: null, accounts: [] },
      set_google_client: new Error("invalidClientId"),
    });
    render(<AccountsPanel />);

    fireEvent.change(await screen.findByLabelText(text.client.id), {
      target: { value: "x" },
    });
    fireEvent.change(screen.getByLabelText(text.client.secret), {
      target: { value: "y" },
    });
    fireEvent.click(screen.getByRole("button", { name: text.client.save }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      text.errors.invalidClientId,
    );
  });

  it("lists the accounts and their calendars on demand", async () => {
    answer({
      google_status: {
        clientId: "1.apps.googleusercontent.com",
        accounts: ["max@example.com"],
      },
      google_calendars: [
        { id: "max@example.com", name: "Max", color: "#9fc6e7", primary: true },
        { id: "f@x", name: "Feriados", color: null, primary: false },
      ],
    });
    render(<AccountsPanel />);

    expect(await screen.findByText("max@example.com")).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Ver agendas de max@example.com" }),
    );
    expect(await screen.findByText("Feriados")).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("google_calendars", {
      email: "max@example.com",
    });
  });

  it("connects through the browser and says so", async () => {
    answer({
      google_status: { clientId: "1.apps.googleusercontent.com", accounts: [] },
      connect_google_account: "noiva@example.com",
    });
    render(<AccountsPanel />);

    fireEvent.click(
      await screen.findByRole("button", { name: text.list.connect }),
    );
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "noiva@example.com foi conectada.",
      ),
    );
  });

  it("asks before disconnecting", async () => {
    answer({
      google_status: {
        clientId: "1.apps.googleusercontent.com",
        accounts: ["max@example.com"],
      },
      disconnect_google_account: undefined,
    });
    render(<AccountsPanel />);

    fireEvent.click(
      await screen.findByRole("button", {
        name: "Desconectar max@example.com",
      }),
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "disconnect_google_account",
      expect.anything(),
    );
    fireEvent.click(
      screen.getByRole("button", { name: text.list.confirmDisconnect.confirm }),
    );
    expect(invoke).toHaveBeenCalledWith("disconnect_google_account", {
      email: "max@example.com",
    });
  });

  it("sets a calendar's color through the CSSOM only", () => {
    expect(css).toContain("var(--swatch");
  });
});
