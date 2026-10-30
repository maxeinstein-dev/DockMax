import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import type { WidgetInstance } from "../../core/widgets";
import type { WidgetUpdate } from "../contract";
import ProgressSettings from "./ProgressSettings";
import { progressInstance } from "./testing";
import { getAlert, queryAlert } from "../../test/alerts";

type Update = WidgetUpdate<"time-progress">;

/** The host: applies an accepted update to the instance, as
 * `widgets-changed` does; `reject` makes every update fail with it. */
function Host({
  initial,
  onUpdate,
  onRemove,
  onBack,
  reject,
}: {
  initial: WidgetInstance<"time-progress">;
  onUpdate: (update: Update) => void;
  onRemove: () => Promise<void>;
  onBack: () => void;
  reject?: { code: string };
}) {
  const [instance, setInstance] = useState(initial);
  return (
    <ProgressSettings
      instance={instance}
      onUpdate={async (update) => {
        onUpdate(update);
        if (reject) throw reject;
        setInstance({ ...instance, ...update });
      }}
      onRemove={onRemove}
      onBack={onBack}
    />
  );
}

function settings(
  options: {
    instance?: WidgetInstance<"time-progress">;
    reject?: { code: string };
    onRemove?: () => Promise<void>;
  } = {},
) {
  const onUpdate = vi.fn<(update: Update) => void>();
  const onRemove = vi.fn(options.onRemove ?? (async () => {}));
  const onBack = vi.fn();
  render(
    <Host
      initial={options.instance ?? progressInstance()}
      onUpdate={onUpdate}
      onRemove={onRemove}
      onBack={onBack}
      reject={options.reject}
    />,
  );
  return { onUpdate, onRemove, onBack };
}

function periodsList() {
  return screen.getByRole("list", { name: "Períodos, na ordem do painel" });
}

function checkbox(name: string) {
  return within(periodsList()).getByRole("checkbox", { name });
}

/** The periods' names in the list's order, checked ones marked with ✓. */
function order() {
  return within(periodsList())
    .getAllByRole("checkbox")
    .map(
      (box) =>
        `${(box as HTMLInputElement).checked ? "✓" : ""}${box.closest("li")?.querySelector(".period-name")?.textContent ?? ""}`,
    );
}

async function flush() {
  await act(async () => {});
}

describe("ProgressSettings", () => {
  it("has the settings title and Voltar, focused when it mounts", () => {
    const { onBack } = settings();

    expect(
      screen.getByRole("heading", { name: "Ajustes do progresso" }),
    ).toBeInTheDocument();
    const back = screen.getByRole("button", { name: "Voltar" });
    expect(back).toHaveFocus();
    fireEvent.click(back);
    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it("says it saves by itself, in the footer", () => {
    settings();
    expect(screen.getByRole("contentinfo")).toHaveTextContent(
      "Salvo automaticamente",
    );
  });

  describe("label", () => {
    it("saves the label when the field is done, with the config as it is", async () => {
      const instance = progressInstance();
      const { onUpdate } = settings({ instance });
      const field = screen.getByRole("textbox", { name: "Rótulo" });
      expect(field).toHaveAttribute("placeholder", "Progresso do tempo");

      fireEvent.change(field, { target: { value: "  Trabalho " } });
      expect(onUpdate).not.toHaveBeenCalled();
      fireEvent.blur(field);
      await flush();

      expect(onUpdate).toHaveBeenCalledWith({
        label: "Trabalho",
        config: instance.config,
      });
      expect(field).toHaveValue("Trabalho");
    });

    it("clears the label to the kind's name, and skips an unchanged one", async () => {
      const { onUpdate } = settings({
        instance: progressInstance({}, "Trabalho"),
      });
      const field = screen.getByRole("textbox", { name: "Rótulo" });

      fireEvent.blur(field);
      await flush();
      expect(onUpdate).not.toHaveBeenCalled();

      fireEvent.change(field, { target: { value: "   " } });
      fireEvent.keyDown(field, { key: "Enter" });
      await flush();
      expect(onUpdate).toHaveBeenCalledWith(
        expect.objectContaining({ label: null }),
      );
    });

    it("refuses a label over 40 code points without sending it", async () => {
      const { onUpdate } = settings();
      const field = screen.getByRole("textbox", { name: "Rótulo" });

      fireEvent.change(field, { target: { value: "é".repeat(41) } });
      fireEvent.blur(field);
      await flush();

      expect(onUpdate).not.toHaveBeenCalled();
      expect(getAlert()).toHaveTextContent(
        "Use de 1 a 40 caracteres, com ao menos um visível.",
      );
      expect(field).toHaveValue("");
    });

    it("counts code points, not UTF-16 units: 40 emoji fit", async () => {
      const { onUpdate } = settings();
      const field = screen.getByRole("textbox", { name: "Rótulo" });

      fireEvent.change(field, { target: { value: "😀".repeat(40) } });
      fireEvent.blur(field);
      await flush();

      expect(onUpdate).toHaveBeenCalledWith(
        expect.objectContaining({ label: "😀".repeat(40) }),
      );
    });
  });

  describe("periods", () => {
    it("lists the chosen periods first, in order, then the others", () => {
      settings({
        instance: progressInstance({ periods: ["month", "day"] }),
      });
      expect(order()).toEqual(["✓Mês", "✓Dia", "Semana", "Ano", "Expediente"]);
      expect(screen.getByText("O primeiro período é o do anel no ícone."));
    });

    it("adds a checked period at the end and removes an unchecked one", async () => {
      const { onUpdate } = settings();

      fireEvent.click(checkbox("Expediente seg. a sex."));
      await flush();
      expect(onUpdate).toHaveBeenLastCalledWith(
        expect.objectContaining({
          config: expect.objectContaining({
            periods: ["day", "week", "month", "year", "workday"],
          }),
        }),
      );

      fireEvent.click(checkbox("Semana"));
      await flush();
      expect(onUpdate).toHaveBeenLastCalledWith(
        expect.objectContaining({
          config: expect.objectContaining({
            periods: ["day", "month", "year", "workday"],
          }),
        }),
      );
      expect(order()).toEqual([
        "✓Dia",
        "✓Mês",
        "✓Ano",
        "✓Expediente",
        "Semana",
      ]);
    });

    it("keeps at least one period: the last checked can't be unchecked", () => {
      settings({ instance: progressInstance({ periods: ["year"] }) });

      expect(checkbox("Ano")).toBeChecked();
      expect(checkbox("Ano")).toBeDisabled();
      expect(checkbox("Dia")).toBeEnabled();
    });

    it("moves a period up and down, only among the checked ones", async () => {
      const { onUpdate } = settings({
        instance: progressInstance({ periods: ["day", "week"] }),
      });

      expect(screen.getByRole("button", { name: "Subir Dia" })).toBeDisabled();
      expect(
        screen.getByRole("button", { name: "Descer Semana" }),
      ).toBeDisabled();
      expect(screen.getByRole("button", { name: "Subir Mês" })).toBeDisabled();
      expect(screen.getByRole("button", { name: "Descer Mês" })).toBeDisabled();

      fireEvent.click(screen.getByRole("button", { name: "Descer Dia" }));
      await flush();
      expect(onUpdate).toHaveBeenLastCalledWith(
        expect.objectContaining({
          config: expect.objectContaining({ periods: ["week", "day"] }),
        }),
      );
      expect(order().slice(0, 2)).toEqual(["✓Semana", "✓Dia"]);
      // The moved period reached the end: the focus stays on its row.
      expect(screen.getByRole("button", { name: "Subir Dia" })).toHaveFocus();

      fireEvent.click(screen.getByRole("button", { name: "Subir Dia" }));
      await flush();
      expect(onUpdate).toHaveBeenLastCalledWith(
        expect.objectContaining({
          config: expect.objectContaining({ periods: ["day", "week"] }),
        }),
      );
    });
  });

  describe("workday", () => {
    it("shows the workday hours on its line only while it is checked", async () => {
      settings();
      expect(screen.queryByLabelText("Início do expediente")).toBeNull();

      fireEvent.click(checkbox("Expediente seg. a sex."));
      await flush();

      const start = screen.getByLabelText("Início do expediente");
      const end = screen.getByLabelText("Fim do expediente");
      expect(start).toHaveValue("09:00");
      expect(end).toHaveValue("18:00");
      expect(start.closest("li")).toHaveTextContent(/^das\s*às$/);
    });

    it("saves an hour when the field is done", async () => {
      const instance = progressInstance({ periods: ["workday"] });
      const { onUpdate } = settings({ instance });
      const end = screen.getByLabelText("Fim do expediente");

      fireEvent.change(end, { target: { value: "17:30" } });
      expect(onUpdate).not.toHaveBeenCalled();
      fireEvent.blur(end);
      await flush();

      expect(onUpdate).toHaveBeenCalledWith({
        label: null,
        config: {
          ...instance.config,
          workday: { start: "09:00", end: "17:30" },
        },
      });
    });

    it("puts back a cleared hour without saving", async () => {
      const { onUpdate } = settings({
        instance: progressInstance({ periods: ["workday"] }),
      });
      const start = screen.getByLabelText("Início do expediente");

      fireEvent.change(start, { target: { value: "" } });
      fireEvent.blur(start);
      await flush();

      expect(onUpdate).not.toHaveBeenCalled();
      expect(start).toHaveValue("09:00");
    });
  });

  it("chooses the first day of the week", async () => {
    const { onUpdate } = settings();
    const group = screen.getByRole("group", { name: "Semana começa" });

    expect(
      within(group).getByRole("button", { name: "Segunda" }),
    ).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(within(group).getByRole("button", { name: "Domingo" }));
    await flush();

    expect(onUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        config: expect.objectContaining({ weekStart: "sunday" }),
      }),
    );
    expect(
      within(group).getByRole("button", { name: "Domingo" }),
    ).toHaveAttribute("aria-pressed", "true");
  });

  it("chooses how many icons wide the tile is", async () => {
    const { onUpdate } = settings();
    const group = screen.getByRole("group", { name: "Largura no dock" });

    expect(
      within(group).getByRole("button", { name: "1 ícone" }),
    ).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(within(group).getByRole("button", { name: "3 ícones" }));
    await flush();

    expect(onUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        config: expect.objectContaining({ width: 3 }),
      }),
    );
  });

  describe("a refused change", () => {
    it("shows the error's message and goes back to the instance", async () => {
      const { onUpdate } = settings({
        instance: progressInstance({ periods: ["workday"] }),
        reject: { code: "invalidWorkday" },
      });
      const end = screen.getByLabelText("Fim do expediente");

      fireEvent.change(end, { target: { value: "08:00" } });
      fireEvent.blur(end);
      await flush();

      expect(onUpdate).toHaveBeenCalledTimes(1);
      expect(getAlert()).toHaveTextContent(
        "O expediente precisa terminar depois de começar, no mesmo dia.",
      );
      expect(end).toHaveValue("18:00");
    });

    it("puts the periods back too", async () => {
      settings({ reject: { code: "invalidPeriods" } });

      fireEvent.click(checkbox("Semana"));
      await flush();

      expect(checkbox("Semana")).toBeChecked();
      expect(getAlert()).toHaveTextContent(
        "Escolha ao menos um período, sem repetir.",
      );
    });

    it("clears the message on the next accepted change", async () => {
      const { onUpdate } = settings();
      const field = screen.getByRole("textbox", { name: "Rótulo" });
      fireEvent.change(field, { target: { value: "x".repeat(41) } });
      fireEvent.blur(field);
      await flush();
      expect(getAlert()).toBeInTheDocument();

      fireEvent.click(checkbox("Ano"));
      await flush();
      expect(onUpdate).toHaveBeenCalledTimes(1);
      expect(queryAlert()).toBeNull();
    });
  });

  describe("Remover do dock", () => {
    it("removes without asking", async () => {
      const { onRemove } = settings();

      fireEvent.click(screen.getByRole("button", { name: "Remover do dock" }));
      await flush();

      expect(onRemove).toHaveBeenCalledTimes(1);
      expect(queryAlert()).toBeNull();
    });

    it("shows why a removal failed", async () => {
      settings({
        onRemove: async () => {
          throw { code: "writeFailed" };
        },
      });

      fireEvent.click(screen.getByRole("button", { name: "Remover do dock" }));
      await flush();

      expect(getAlert()).toHaveTextContent(
        "Não foi possível salvar a mudança. Tente de novo.",
      );
      expect(
        screen.getByRole("button", { name: "Remover do dock" }),
      ).toBeEnabled();
    });
  });
});
