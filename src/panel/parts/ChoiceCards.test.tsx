import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import ChoiceCards from "./ChoiceCards";

const CHOICES = [
  { value: "visible", label: "Visível", hint: "Logo acima da barra." },
  { value: "autohide", label: "Automática" },
  { value: "hidden", label: "Oculta" },
] as const;

type Value = (typeof CHOICES)[number]["value"];

function renderCards(selected: Value | null, onSelect = vi.fn()) {
  render(
    <ChoiceCards<Value>
      label="Barra"
      choices={CHOICES}
      selected={selected}
      onSelect={onSelect}
      thumb={(value, chosen) => (
        <span data-testid={`thumb-${value}`} data-chosen={chosen} />
      )}
    />,
  );
  return onSelect;
}

const card = (name: string) =>
  within(screen.getByRole("radiogroup", { name: "Barra" })).getByRole("radio", {
    name,
  });

describe("ChoiceCards", () => {
  it("names each card by its name and describes it by its hint", () => {
    renderCards("visible");

    expect(card("Visível")).toHaveAccessibleDescription("Logo acima da barra.");
    expect(card("Automática")).not.toHaveAccessibleDescription();
    expect(card("Visível")).toHaveAttribute("aria-checked", "true");
    expect(card("Oculta")).toHaveAttribute("aria-checked", "false");
  });

  it("is one choice of many: a radio group of radios", () => {
    renderCards("visible");

    expect(
      within(screen.getByRole("radiogroup", { name: "Barra" }))
        .getAllByRole("radio")
        .map((radio) => radio.textContent),
    ).toEqual(["VisívelLogo acima da barra.", "Automática", "Oculta"]);
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("tells the picture which card is chosen", () => {
    renderCards("autohide");

    expect(screen.getByTestId("thumb-autohide")).toHaveAttribute(
      "data-chosen",
      "true",
    );
    expect(screen.getByTestId("thumb-hidden")).toHaveAttribute(
      "data-chosen",
      "false",
    );
  });

  it("has one Tab stop, at the chosen card, else the first", () => {
    renderCards(null);
    expect(card("Visível")).toHaveAttribute("tabindex", "0");
    expect(card("Oculta")).toHaveAttribute("tabindex", "-1");
  });

  it("moves with the arrows without choosing, and chooses on a click", () => {
    const onSelect = renderCards("hidden");

    card("Oculta").focus();
    fireEvent.keyDown(card("Oculta"), { key: "ArrowRight" });
    expect(card("Visível")).toHaveFocus();
    fireEvent.keyDown(card("Visível"), { key: "End" });
    expect(card("Oculta")).toHaveFocus();
    expect(onSelect).not.toHaveBeenCalled();

    fireEvent.click(card("Oculta"));
    expect(onSelect).not.toHaveBeenCalled();
    fireEvent.click(card("Automática"));
    expect(onSelect).toHaveBeenCalledWith("autohide");
  });

  it("goes up and down a row with the arrows in a single row, card by card", () => {
    renderCards("visible");

    card("Visível").focus();
    fireEvent.keyDown(card("Visível"), { key: "ArrowDown" });
    expect(card("Automática")).toHaveFocus();
    fireEvent.keyDown(card("Automática"), { key: "ArrowUp" });
    expect(card("Visível")).toHaveFocus();
    fireEvent.keyDown(card("Visível"), { key: "ArrowUp" });
    expect(card("Oculta")).toHaveFocus();
  });

  describe("in a grid of rows", () => {
    // Seven cards, three a row: 0 1 2 / 3 4 5 / 6.
    const NAMES = ["A", "B", "C", "D", "E", "F", "G"] as const;
    const COLUMNS = 3;

    function renderGrid() {
      const onSelect = vi.fn();
      render(
        <ChoiceCards<(typeof NAMES)[number]>
          label="Grade"
          choices={NAMES.map((name) => ({ value: name, label: name }))}
          selected="A"
          onSelect={onSelect}
          thumb={() => null}
        />,
      );
      // jsdom lays nothing out: each card says the row the grid put it in.
      screen.getAllByRole("radio").forEach((radio, index) => {
        Object.defineProperty(radio, "offsetTop", {
          value: Math.floor(index / COLUMNS) * 100,
        });
      });
      return onSelect;
    }

    const cell = (name: string) => screen.getByRole("radio", { name });

    /** Where `key` takes the focus from the card `from`. */
    function press(from: string, key: string): string | null {
      cell(from).focus();
      fireEvent.keyDown(cell(from), { key });
      return document.activeElement?.textContent ?? null;
    }

    it("goes down and up a row, not a card", () => {
      const onSelect = renderGrid();

      expect(press("B", "ArrowDown")).toBe("E");
      expect(press("E", "ArrowUp")).toBe("B");
      expect(press("D", "ArrowRight")).toBe("E");
      expect(press("E", "ArrowLeft")).toBe("D");
      expect(onSelect).not.toHaveBeenCalled();
    });

    it("wraps around the column, and reaches a short last row", () => {
      renderGrid();

      // Down from a card with none under it: the last card.
      expect(press("E", "ArrowDown")).toBe("G");
      // Down from the last row: the top of the column.
      expect(press("G", "ArrowDown")).toBe("A");
      // Up from the first row: the bottom of the column…
      expect(press("A", "ArrowUp")).toBe("G");
      // …or the row before when the last is too short.
      expect(press("C", "ArrowUp")).toBe("F");
    });
  });
});
