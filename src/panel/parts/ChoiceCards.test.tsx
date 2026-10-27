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
  within(screen.getByRole("group", { name: "Barra" })).getByRole("button", {
    name,
  });

describe("ChoiceCards", () => {
  it("names each card by its name and describes it by its hint", () => {
    renderCards("visible");

    expect(card("Visível")).toHaveAccessibleDescription("Logo acima da barra.");
    expect(card("Automática")).not.toHaveAccessibleDescription();
    expect(card("Visível")).toHaveAttribute("aria-pressed", "true");
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
});
