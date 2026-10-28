import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { TodoConfig, WidgetInstance } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import type { WidgetUpdate } from "../contract";
import TodoPanel from "./TodoPanel";
import TodoSettings from "./TodoSettings";

const text = ptBR.widgets.todo;

type OnUpdate = (update: WidgetUpdate<"todo">) => Promise<void>;

function todo(items: TodoConfig["items"] = []): WidgetInstance<"todo"> {
  return {
    id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6093",
    label: null,
    kind: "todo",
    config: { items },
  };
}

const SAMPLE = [
  { id: "t1", text: "Comprar pão", done: false },
  { id: "t2", text: "Pagar a luz", done: true },
];

function panel(
  instance: WidgetInstance<"todo">,
  onUpdate = vi.fn<OnUpdate>(() => Promise.resolve()),
) {
  render(
    <TodoPanel
      instance={instance}
      now={new Date()}
      onAdjust={() => undefined}
      returnedFromSettings={false}
      onUpdate={onUpdate}
    />,
  );
  return onUpdate;
}

function lastItems(onUpdate: ReturnType<typeof vi.fn<OnUpdate>>) {
  const calls = onUpdate.mock.calls;
  return calls[calls.length - 1]?.[0].config.items;
}

describe("TodoPanel", () => {
  it("adds a task, trimmed, at the end", () => {
    const onUpdate = panel(todo(SAMPLE));
    fireEvent.change(screen.getByLabelText(text.new), {
      target: { value: "  Ligar para o banco " },
    });
    fireEvent.click(screen.getByRole("button", { name: text.add }));
    const items = lastItems(onUpdate);
    expect(items?.map((item) => item.text)).toEqual([
      "Comprar pão",
      "Pagar a luz",
      "Ligar para o banco",
    ]);
    expect(items?.[2]?.done).toBe(false);
    expect(screen.getByText("Ligar para o banco")).toBeInTheDocument();
  });

  it("marks a task done and moves it among the done", () => {
    const onUpdate = panel(todo(SAMPLE));
    fireEvent.click(screen.getByRole("checkbox", { name: "Comprar pão" }));
    expect(lastItems(onUpdate)?.[0]).toEqual({
      id: "t1",
      text: "Comprar pão",
      done: true,
    });
    const done = screen.getByRole("list", { name: "2 feitas" });
    expect(within(done).getByText("Comprar pão")).toBeInTheDocument();
    expect(screen.getByText(text.allDone)).toBeInTheDocument();
  });

  it("removes a task", () => {
    const onUpdate = panel(todo(SAMPLE));
    fireEvent.click(
      screen.getByRole("button", { name: "Remover a tarefa Pagar a luz" }),
    );
    expect(lastItems(onUpdate)).toEqual([SAMPLE[0]]);
  });

  it("goes back to the saved list when Rust refuses", async () => {
    panel(
      todo(SAMPLE),
      vi.fn<OnUpdate>(() => Promise.reject(new Error("no"))),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Remover a tarefa Pagar a luz" }),
    );
    expect(
      await screen.findByText(ptBR.widgets.errors.invalidTasks),
    ).toBeInTheDocument();
    expect(screen.getByText("Pagar a luz")).toBeInTheDocument();
  });

  it("says when the list is empty, and counts the open ones", () => {
    panel(todo());
    expect(screen.getByText(text.empty)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: text.add })).toBeDisabled();
  });
});

describe("TodoSettings", () => {
  it("clears the done tasks", () => {
    const onUpdate = vi.fn<OnUpdate>(() => Promise.resolve());
    render(
      <TodoSettings
        instance={todo(SAMPLE)}
        onUpdate={onUpdate}
        onRemove={() => Promise.resolve()}
        onBack={() => undefined}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: text.clearDone }));
    expect(lastItems(onUpdate)).toEqual([SAMPLE[0]]);
  });
});
