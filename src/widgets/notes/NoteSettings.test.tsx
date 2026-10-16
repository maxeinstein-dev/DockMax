import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { WidgetInstance } from "../../core/widgets";
import { WIDGET_FIXTURES } from "../../core/widgets.fixtures";
import { ptBR } from "../../i18n/pt-BR";
import type { WidgetUpdate } from "../contract";
import NoteSettings from "./NoteSettings";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const NOTE = WIDGET_FIXTURES["note-default"];
const common = ptBR.widgets.common;
const confirm = ptBR.widgets.note.confirmRemove;
const errors = ptBR.widgets.errors;

/** What `get_note` gives. */
function stored(note: () => Promise<string>) {
  invoke.mockImplementation((command: string) =>
    command === "get_note"
      ? note()
      : Promise.reject(new Error(`unexpected ${command}`)),
  );
}

/** Lets the pending promises run. */
async function settle() {
  await act(async () => {});
}

function renderSettings(
  instance: WidgetInstance<"note"> = NOTE,
  {
    onUpdate = vi.fn<(update: WidgetUpdate<"note">) => Promise<void>>(() =>
      Promise.resolve(),
    ),
    onRemove = vi.fn<() => Promise<void>>(() => Promise.resolve()),
  } = {},
) {
  const onBack = vi.fn();
  const view = render(
    <NoteSettings
      instance={instance}
      onUpdate={onUpdate}
      onRemove={onRemove}
      onBack={onBack}
    />,
  );
  return { ...view, onUpdate, onRemove, onBack };
}

function labelField() {
  return screen.getByRole("textbox", { name: common.label });
}

function removeButton() {
  return screen.getByRole("button", { name: common.remove });
}

beforeEach(() => {
  invoke.mockReset();
  stored(() => Promise.resolve(""));
});

describe("NoteSettings", () => {
  it("has the title, Voltar focused, and the footer", async () => {
    const { onBack } = renderSettings();
    await settle();

    expect(
      screen.getByRole("heading", { name: ptBR.widgets.note.settingsTitle }),
    ).toBeInTheDocument();
    const back = screen.getByRole("button", { name: common.back });
    expect(back).toHaveFocus();
    fireEvent.click(back);
    expect(onBack).toHaveBeenCalledTimes(1);
    expect(screen.getByText(common.footer)).toBeInTheDocument();
  });
});

describe("the label", () => {
  it("shows the label, with the kind's name as placeholder", async () => {
    renderSettings({ ...NOTE, label: "Mercado" });
    await settle();

    expect(labelField()).toHaveValue("Mercado");
    expect(labelField()).toHaveAttribute(
      "placeholder",
      ptBR.widgets.kinds.note,
    );
  });

  it("saves the label when done with the field, with the empty config", async () => {
    const { onUpdate } = renderSettings();
    await settle();

    fireEvent.change(labelField(), { target: { value: "Mercado" } });
    expect(onUpdate).not.toHaveBeenCalled();
    fireEvent.blur(labelField());
    await settle();
    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
      label: "Mercado",
      config: {},
    });
  });

  it("saves on Enter too", async () => {
    const { onUpdate } = renderSettings();
    await settle();

    fireEvent.change(labelField(), { target: { value: "Ideias" } });
    fireEvent.keyDown(labelField(), { key: "Enter" });
    await settle();
    expect(onUpdate).toHaveBeenCalledWith({ label: "Ideias", config: {} });
  });

  it("goes back to the kind's name when emptied", async () => {
    const { onUpdate } = renderSettings({ ...NOTE, label: "Mercado" });
    await settle();

    fireEvent.change(labelField(), { target: { value: "   " } });
    fireEvent.blur(labelField());
    await settle();
    expect(onUpdate).toHaveBeenCalledWith({ label: null, config: {} });
  });

  it("saves nothing when the label didn't change", async () => {
    const { onUpdate } = renderSettings({ ...NOTE, label: "Mercado" });
    await settle();
    fireEvent.blur(labelField());
    await settle();
    expect(onUpdate).not.toHaveBeenCalled();
  });

  it("saves nothing when a note without a label stays without one", async () => {
    const { onUpdate } = renderSettings();
    await settle();
    fireEvent.change(labelField(), { target: { value: " " } });
    fireEvent.blur(labelField());
    await settle();
    expect(onUpdate).not.toHaveBeenCalled();
  });

  it("counts code points: 40 emoji pass, 41 don't", async () => {
    const { onUpdate } = renderSettings();
    await settle();

    fireEvent.change(labelField(), { target: { value: "🙂".repeat(41) } });
    fireEvent.blur(labelField());
    await settle();
    expect(onUpdate).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toHaveTextContent(errors.invalidLabel);
    expect(labelField()).toBeInvalid();

    fireEvent.change(labelField(), { target: { value: "🙂".repeat(40) } });
    fireEvent.blur(labelField());
    await settle();
    expect(onUpdate).toHaveBeenCalledWith({
      label: "🙂".repeat(40),
      config: {},
    });
  });

  it("shows a refusal and goes back to the accepted label", async () => {
    const { onUpdate } = renderSettings(
      { ...NOTE, label: "Mercado" },
      { onUpdate: vi.fn(() => Promise.reject({ code: "invalidLabel" })) },
    );
    await settle();

    fireEvent.change(labelField(), { target: { value: "​" } });
    fireEvent.blur(labelField());
    await settle();
    expect(onUpdate).toHaveBeenCalledOnce();
    expect(screen.getByRole("alert")).toHaveTextContent(errors.invalidLabel);
    expect(labelField()).toHaveValue("Mercado");
  });

  it("follows a label changed elsewhere", async () => {
    const { rerender, onUpdate, onRemove, onBack } = renderSettings();
    await settle();

    rerender(
      <NoteSettings
        instance={{ ...NOTE, label: "Leituras" }}
        onUpdate={onUpdate}
        onRemove={onRemove}
        onBack={onBack}
      />,
    );
    expect(labelField()).toHaveValue("Leituras");
  });
});

describe("Remover do dock", () => {
  it("removes an empty note without asking", async () => {
    stored(() => Promise.resolve(""));
    const { onRemove } = renderSettings();
    await settle();

    fireEvent.click(removeButton());
    expect(onRemove).toHaveBeenCalledOnce();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("asks first when the note has text, focused on Cancelar", async () => {
    stored(() => Promise.resolve("Pão\nLeite"));
    const { onRemove } = renderSettings();
    await settle();

    fireEvent.click(removeButton());
    expect(onRemove).not.toHaveBeenCalled();
    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent(`${confirm.title} ${confirm.detail}`);
    expect(screen.getByRole("button", { name: confirm.cancel })).toHaveFocus();
    expect(
      screen.queryByRole("button", { name: common.remove }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: confirm.confirm }));
    expect(onRemove).toHaveBeenCalledOnce();
  });

  it("cancelling brings the remove button back, focused", async () => {
    stored(() => Promise.resolve("texto"));
    const { onRemove } = renderSettings();
    await settle();

    fireEvent.click(removeButton());
    fireEvent.click(screen.getByRole("button", { name: confirm.cancel }));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(removeButton()).toHaveFocus();
    expect(onRemove).not.toHaveBeenCalled();
  });

  it("asks while the text is still being read, or can't be read", async () => {
    stored(() => new Promise(() => {}));
    const first = renderSettings();
    fireEvent.click(removeButton());
    expect(first.onRemove).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toHaveTextContent(confirm.title);
    first.unmount();

    stored(() => Promise.reject({ code: "writeFailed" }));
    const second = renderSettings();
    await settle();
    fireEvent.click(removeButton());
    expect(second.onRemove).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toHaveTextContent(confirm.title);
  });

  it("reads the note of this instance", async () => {
    renderSettings();
    await settle();
    expect(invoke).toHaveBeenCalledWith("get_note", { id: NOTE.id });
  });

  it("turns the buttons off while removing", async () => {
    stored(() => Promise.resolve("texto"));
    renderSettings(NOTE, { onRemove: vi.fn(() => new Promise(() => {})) });
    await settle();

    fireEvent.click(removeButton());
    fireEvent.click(screen.getByRole("button", { name: confirm.confirm }));
    await settle();
    expect(screen.getByRole("button", { name: confirm.cancel })).toBeDisabled();
    expect(
      screen.getByRole("button", { name: confirm.confirm }),
    ).toBeDisabled();
  });

  it("shows a failed removal", async () => {
    stored(() => Promise.resolve("texto"));
    renderSettings(NOTE, {
      onRemove: vi.fn(() => Promise.reject({ code: "notFound" })),
    });
    await settle();

    fireEvent.click(removeButton());
    fireEvent.click(screen.getByRole("button", { name: confirm.confirm }));
    await settle();
    expect(screen.getByRole("alert")).toHaveTextContent(errors.notFound);
    expect(removeButton()).toBeEnabled();
  });

  it("disables the button while an empty note is removed", async () => {
    renderSettings(NOTE, { onRemove: vi.fn(() => new Promise(() => {})) });
    await settle();

    fireEvent.click(removeButton());
    await settle();
    expect(removeButton()).toBeDisabled();
  });
});
