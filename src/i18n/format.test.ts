import { describe, expect, it } from "vitest";
import { format, plural } from ".";
import { ptBR } from "./pt-BR";

describe("format", () => {
  it("replaces each placeholder by its value", () => {
    expect(format("{count} de {max}", { count: 3, max: 12 })).toBe("3 de 12");
    expect(format("Subir {name}", { name: "Relógio" })).toBe("Subir Relógio");
  });

  it("replaces a placeholder used twice", () => {
    expect(format("{a}-{a}", { a: "x" })).toBe("x-x");
  });

  it("leaves a placeholder without a value as written", () => {
    expect(format("{zone} · {offset}", { zone: "Lisboa" })).toBe(
      "Lisboa · {offset}",
    );
  });

  it("never reads a value from the prototype", () => {
    expect(format("{toString}", {})).toBe("{toString}");
  });

  it("inserts values as they are, without reading them as templates", () => {
    expect(format("{a}", { a: "{b}", b: "no" })).toBe("{b}");
  });
});

describe("plural", () => {
  const { lines } = ptBR.widgets.note;

  it("uses `one` for 1 and `other` for any other count", () => {
    expect(plural(lines, 1)).toBe("1 linha");
    expect(plural(lines, 0)).toBe("0 linhas");
    expect(plural(lines, 3)).toBe("3 linhas");
  });

  it("writes the remaining time of each unit", () => {
    const { units } = ptBR.widgets.progress;
    expect(plural(units.day, 1)).toBe("1 dia");
    expect(plural(units.day, 6)).toBe("6 dias");
    expect(plural(units.hour, 3)).toBe("3 h");
    expect(plural(units.minute, 45)).toBe("45 min");
    expect(
      format(ptBR.widgets.progress.remaining, {
        time: [plural(units.day, 2), plural(units.hour, 3)].join(" "),
      }),
    ).toBe("faltam 2 dias 3 h");
  });
});
