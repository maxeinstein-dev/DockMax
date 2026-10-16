import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import type {
  ClockConfig,
  WidgetCommandError,
  WidgetInstance,
} from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import type { WidgetUpdate } from "../contract";
import ClockSettings from "./ClockSettings";

// `src` doesn't load the Node types; only `TZ` is needed here.
declare const process: { env: Record<string, string | undefined> };

/** Runs the tests of the enclosing `describe` with `timeZone` as the
 * system zone (Node re-reads `TZ` when it changes). */
function inZone(timeZone: string): void {
  const before = process.env.TZ;
  const system = new Intl.DateTimeFormat().resolvedOptions().timeZone;
  beforeAll(() => {
    process.env.TZ = timeZone;
  });
  afterAll(() => {
    process.env.TZ = before ?? system;
  });
}

const text = ptBR.widgets;

const CONFIG: ClockConfig = {
  timeZone: "America/Sao_Paulo",
  hourCycle: "h23",
  showSeconds: false,
  iconStyle: "digital",
};

const clock = (
  config: Partial<ClockConfig> = {},
  label: string | null = null,
): WidgetInstance<"clock"> => ({
  id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6071",
  label,
  kind: "clock",
  config: { ...CONFIG, ...config },
});

type OnUpdate = (update: WidgetUpdate<"clock">) => Promise<void>;

function renderSettings(
  instance: WidgetInstance<"clock">,
  {
    onUpdate = vi.fn<OnUpdate>(() => Promise.resolve()),
    onRemove = vi.fn(() => Promise.resolve()),
    onBack = vi.fn(),
  }: {
    onUpdate?: OnUpdate;
    onRemove?: () => Promise<void>;
    onBack?: () => void;
  } = {},
) {
  const view = render(
    <ClockSettings
      instance={instance}
      onUpdate={onUpdate}
      onRemove={onRemove}
      onBack={onBack}
    />,
  );
  const rerender = (next: WidgetInstance<"clock">) =>
    view.rerender(
      <ClockSettings
        instance={next}
        onUpdate={onUpdate}
        onRemove={onRemove}
        onBack={onBack}
      />,
    );
  return { ...view, rerender, onUpdate, onRemove, onBack };
}

const rejected = (code: WidgetCommandError["code"]) => {
  const error: WidgetCommandError = { code };
  return () => Promise.reject(error);
};

/** A promise and the functions that settle it. */
function deferred() {
  let resolve!: () => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<void>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const labelField = () => screen.getByRole("textbox", { name: "Rótulo" });
const zoneField = () =>
  screen.getByRole<HTMLSelectElement>("combobox", { name: "Fuso horário" });
const option = (name: string) => screen.getByRole("button", { name });

describe("ClockSettings", () => {
  it("is the Ajustes do relógio view, focused on Voltar", () => {
    const { onBack } = renderSettings(clock());

    expect(
      screen.getByRole("heading", { name: "Ajustes do relógio" }),
    ).toBeInTheDocument();
    const back = screen.getByRole("button", { name: "Voltar" });
    expect(back).toHaveFocus();
    fireEvent.click(back);
    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it("has every setting of the clock, then Remover do dock, then the footer", () => {
    renderSettings(clock());

    expect(labelField()).toBeInTheDocument();
    expect(zoneField()).toBeInTheDocument();
    for (const group of ["Formato", "Segundos no painel", "Ícone no dock"]) {
      expect(screen.getByRole("group", { name: group })).toBeInTheDocument();
    }
    expect(
      screen.getByRole("button", { name: "Remover do dock" }),
    ).toBeInTheDocument();
    expect(screen.getByText(text.common.footer)).toBeInTheDocument();
  });

  it("shows the instance's values", () => {
    renderSettings(
      clock(
        { hourCycle: "h12", showSeconds: true, iconStyle: "analog" },
        "Escritório",
      ),
    );

    expect(labelField()).toHaveValue("Escritório");
    expect(zoneField()).toHaveValue("America/Sao_Paulo");
    expect(option("12 horas")).toHaveAttribute("aria-pressed", "true");
    expect(option("Mostrar")).toHaveAttribute("aria-pressed", "true");
    expect(option("Analógico")).toHaveAttribute("aria-pressed", "true");
  });
});

describe("the label", () => {
  it("is empty without one, with the kind's name as placeholder", () => {
    renderSettings(clock());
    expect(labelField()).toHaveValue("");
    expect(labelField()).toHaveAttribute("placeholder", "Relógio");
  });

  it("saves when the field loses the focus, with the whole config", () => {
    const { onUpdate } = renderSettings(clock());

    fireEvent.change(labelField(), { target: { value: "Lisboa" } });
    expect(onUpdate).not.toHaveBeenCalled();
    fireEvent.blur(labelField());

    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
      label: "Lisboa",
      config: CONFIG,
    });
  });

  it("saves on Enter", () => {
    const { onUpdate } = renderSettings(clock());

    fireEvent.change(labelField(), { target: { value: "Lisboa" } });
    fireEvent.keyDown(labelField(), { key: "Enter" });

    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
      label: "Lisboa",
      config: CONFIG,
    });
  });

  it("goes back to the kind's name when emptied", () => {
    const { onUpdate } = renderSettings(clock({}, "Lisboa"));

    fireEvent.change(labelField(), { target: { value: "   " } });
    fireEvent.blur(labelField());

    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
      label: null,
      config: CONFIG,
    });
  });

  it("doesn't save what didn't change", () => {
    const { onUpdate } = renderSettings(clock({}, "Lisboa"));

    fireEvent.blur(labelField());
    fireEvent.change(labelField(), { target: { value: "Lisboa" } });
    fireEvent.keyDown(labelField(), { key: "Enter" });

    expect(onUpdate).not.toHaveBeenCalled();
  });

  it("counts the limit in code points, as Rust does", () => {
    const { onUpdate } = renderSettings(clock());
    // 40 emoji: 40 code points, 80 UTF-16 units.
    const forty = "🕐".repeat(40);

    fireEvent.change(labelField(), { target: { value: forty } });
    expect(labelField()).not.toHaveAttribute("aria-invalid");
    expect(labelField()).not.toHaveAttribute("maxlength");
    fireEvent.blur(labelField());

    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
      label: forty,
      config: CONFIG,
    });
  });

  it("refuses more than 40 code points without saving, with the message in the footer", () => {
    const { onUpdate } = renderSettings(clock({}, "Lisboa"));

    fireEvent.change(labelField(), { target: { value: "a".repeat(41) } });
    fireEvent.blur(labelField());

    expect(onUpdate).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toHaveTextContent(
      text.errors.invalidLabel,
    );
    expect(labelField()).toHaveAttribute("aria-invalid", "true");
    expect(labelField()).toHaveValue("Lisboa");
  });

  it("saves the label trimmed", () => {
    const { onUpdate } = renderSettings(clock());

    fireEvent.change(labelField(), { target: { value: "  Lisboa  " } });
    fireEvent.blur(labelField());

    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
      label: "Lisboa",
      config: CONFIG,
    });
  });

  it("counts the limit after trimming", () => {
    const { onUpdate } = renderSettings(clock());
    const forty = "a".repeat(40);

    fireEvent.change(labelField(), { target: { value: ` ${forty} ` } });
    fireEvent.blur(labelField());

    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
      label: forty,
      config: CONFIG,
    });
  });

  it("goes back to the saved label when Rust refuses it", async () => {
    const { onUpdate } = renderSettings(clock({}, "Lisboa"), {
      onUpdate: vi.fn<OnUpdate>(rejected("invalidLabel")),
    });

    fireEvent.change(labelField(), { target: { value: "\u0007" } });
    await act(async () => {
      fireEvent.blur(labelField());
    });

    expect(onUpdate).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("alert")).toHaveTextContent(
      text.errors.invalidLabel,
    );
    expect(labelField()).toHaveValue("Lisboa");
  });

  it("follows the instance once Rust accepts it", () => {
    const { rerender } = renderSettings(clock());

    fireEvent.change(labelField(), { target: { value: "Lisboa" } });
    fireEvent.blur(labelField());
    rerender(clock({}, "Lisboa"));

    expect(labelField()).toHaveValue("Lisboa");
  });
});

describe("the time zone", () => {
  inZone("America/Sao_Paulo");

  it("starts with the system zone, then Intl's IANA list", () => {
    renderSettings(clock());

    const options = [...zoneField().options];
    expect(options[0]).toHaveValue("");
    expect(options[0]).toHaveTextContent("Do sistema (Sao Paulo)");
    const tokyo = options.find((o) => o.value === "Asia/Tokyo");
    expect(tokyo).toHaveTextContent("Tokyo · Asia/Tokyo");
    // The exact spelling of the base, never lower-cased or rewritten.
    expect(options.some((o) => o.value === "America/Sao_Paulo")).toBe(true);
  });

  it("selects the system zone for a clock that follows it", () => {
    renderSettings(clock({ timeZone: null }));
    expect(zoneField()).toHaveValue("");
  });

  it("saves the chosen zone at once", () => {
    const { onUpdate } = renderSettings(clock());

    fireEvent.change(zoneField(), { target: { value: "Asia/Tokyo" } });

    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
      label: null,
      config: { ...CONFIG, timeZone: "Asia/Tokyo" },
    });
    expect(zoneField()).toHaveValue("Asia/Tokyo");
  });

  it("saves the system zone as null", () => {
    const { onUpdate } = renderSettings(clock());

    fireEvent.change(zoneField(), { target: { value: "" } });

    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
      label: null,
      config: { ...CONFIG, timeZone: null },
    });
  });

  it("keeps a configured zone Intl doesn't list, as written", () => {
    renderSettings(clock({ timeZone: "Mars/Olympus" }));

    expect(zoneField()).toHaveValue("Mars/Olympus");
    const options = [...zoneField().options];
    expect(options.filter((o) => o.value === "Mars/Olympus")).toHaveLength(1);
  });

  it("goes back to the saved zone when Rust refuses it", async () => {
    renderSettings(clock(), {
      onUpdate: vi.fn<OnUpdate>(rejected("invalidTimeZone")),
    });

    await act(async () => {
      fireEvent.change(zoneField(), { target: { value: "Asia/Tokyo" } });
    });

    expect(screen.getByRole("alert")).toHaveTextContent(
      text.errors.invalidTimeZone,
    );
    expect(zoneField()).toHaveValue("America/Sao_Paulo");
  });
});

describe("the system zone option, when Intl can't name it", () => {
  inZone("Mars/Base");

  it("says only Do sistema", () => {
    renderSettings(clock({ timeZone: null }));
    expect(zoneField().options[0]).toHaveTextContent(/^Do sistema$/);
  });
});

describe("the segmented settings", () => {
  it.each([
    ["12 horas", { hourCycle: "h12" }],
    ["Mostrar", { showSeconds: true }],
    ["Analógico", { iconStyle: "analog" }],
  ] as const)("%s saves at once", (name, change) => {
    const { onUpdate } = renderSettings(clock());

    fireEvent.click(option(name));

    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
      label: null,
      config: { ...CONFIG, ...change },
    });
    expect(option(name)).toHaveAttribute("aria-pressed", "true");
  });

  it("switches back to 24 h and hides the seconds", () => {
    const { onUpdate } = renderSettings(
      clock({ hourCycle: "h12", showSeconds: true, iconStyle: "analog" }),
    );

    fireEvent.click(option("24 horas"));
    expect(onUpdate).toHaveBeenLastCalledWith({
      label: null,
      config: { ...CONFIG, showSeconds: true, iconStyle: "analog" },
    });
  });

  it("builds each change on the ones still on their way", () => {
    const pending = deferred();
    const { onUpdate } = renderSettings(clock(), {
      onUpdate: vi.fn<OnUpdate>(() => pending.promise),
    });

    fireEvent.click(option("12 horas"));
    fireEvent.click(option("Mostrar"));

    expect(onUpdate).toHaveBeenLastCalledWith({
      label: null,
      config: { ...CONFIG, hourCycle: "h12", showSeconds: true },
    });
  });

  it("keeps a label still on its way when another field changes", () => {
    const pending = deferred();
    const { onUpdate } = renderSettings(clock(), {
      onUpdate: vi.fn<OnUpdate>(() => pending.promise),
    });

    fireEvent.change(labelField(), { target: { value: "Lisboa" } });
    fireEvent.blur(labelField());
    fireEvent.click(option("Analógico"));

    expect(onUpdate).toHaveBeenLastCalledWith({
      label: "Lisboa",
      config: { ...CONFIG, iconStyle: "analog" },
    });
  });

  it("keeps the choice when the command answers before widgets-changed", async () => {
    const { rerender } = renderSettings(clock());

    await act(async () => {
      fireEvent.click(option("12 horas"));
    });
    // The command has answered; the new instance hasn't arrived yet.
    expect(option("12 horas")).toHaveAttribute("aria-pressed", "true");

    rerender(clock({ hourCycle: "h12" }));
    expect(option("12 horas")).toHaveAttribute("aria-pressed", "true");
  });

  it("keeps the latest choice while an earlier one's instance arrives", () => {
    const { rerender } = renderSettings(clock(), {
      onUpdate: vi.fn<OnUpdate>(() => new Promise(() => {})),
    });

    fireEvent.click(option("12 horas"));
    fireEvent.click(option("Mostrar"));
    rerender(clock({ hourCycle: "h12" }));

    expect(option("12 horas")).toHaveAttribute("aria-pressed", "true");
    expect(option("Mostrar")).toHaveAttribute("aria-pressed", "true");
  });

  it("follows another window's change once its own has answered", async () => {
    const { rerender } = renderSettings(clock());

    await act(async () => {
      fireEvent.click(option("12 horas"));
    });
    rerender(clock({ iconStyle: "analog" }));

    expect(option("24 horas")).toHaveAttribute("aria-pressed", "true");
    expect(option("Analógico")).toHaveAttribute("aria-pressed", "true");
  });

  it("goes back to the instance and shows the message when refused", async () => {
    const pending = deferred();
    renderSettings(clock(), {
      onUpdate: vi.fn<OnUpdate>(() => pending.promise),
    });

    fireEvent.click(option("12 horas"));
    expect(option("12 horas")).toHaveAttribute("aria-pressed", "true");
    const error: WidgetCommandError = { code: "writeFailed" };
    await act(async () => {
      pending.reject(error);
    });

    expect(option("24 horas")).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("alert")).toHaveTextContent(
      text.errors.writeFailed,
    );
  });

  it("clears the message on the next change", async () => {
    let calls = 0;
    const error: WidgetCommandError = { code: "writeFailed" };
    renderSettings(clock(), {
      onUpdate: vi.fn<OnUpdate>(() =>
        calls++ === 0 ? Promise.reject(error) : new Promise(() => {}),
      ),
    });

    await act(async () => {
      fireEvent.click(option("12 horas"));
    });
    expect(screen.getByRole("alert")).toBeInTheDocument();
    fireEvent.click(option("Mostrar"));

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByText(text.common.footer)).toBeInTheDocument();
  });

  it("ignores the outcome of a change a newer one replaced", async () => {
    const first = deferred();
    const second = deferred();
    const queue = [first, second];
    renderSettings(clock(), {
      onUpdate: vi.fn<OnUpdate>(() => queue.shift()!.promise),
    });

    fireEvent.click(option("12 horas"));
    fireEvent.click(option("Mostrar"));
    await act(async () => {
      first.reject({ code: "writeFailed" });
    });

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(option("12 horas")).toHaveAttribute("aria-pressed", "true");
    expect(option("Mostrar")).toHaveAttribute("aria-pressed", "true");
  });

  it("shows a generic message for an error without a code", async () => {
    renderSettings(clock(), {
      onUpdate: vi.fn<OnUpdate>(() => Promise.reject(new Error("boom"))),
    });

    await act(async () => {
      fireEvent.click(option("Mostrar"));
    });

    expect(screen.getByRole("alert")).toHaveTextContent(
      text.errors.writeFailed,
    );
  });
});

describe("Remover do dock", () => {
  it("removes without asking", () => {
    const { onRemove } = renderSettings(clock());

    fireEvent.click(screen.getByRole("button", { name: "Remover do dock" }));

    expect(onRemove).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("is off while the removal runs", () => {
    const { onRemove } = renderSettings(clock(), {
      onRemove: vi.fn(() => new Promise<void>(() => {})),
    });

    const remove = screen.getByRole("button", { name: "Remover do dock" });
    fireEvent.click(remove);
    fireEvent.click(remove);

    expect(remove).toBeDisabled();
    expect(onRemove).toHaveBeenCalledTimes(1);
  });

  it("shows the message and can try again when refused", async () => {
    renderSettings(clock(), { onRemove: vi.fn(rejected("notFound")) });

    const remove = screen.getByRole("button", { name: "Remover do dock" });
    await act(async () => {
      fireEvent.click(remove);
    });

    expect(screen.getByRole("alert")).toHaveTextContent(text.errors.notFound);
    expect(remove).toBeEnabled();
  });
});
