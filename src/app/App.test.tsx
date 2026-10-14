import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("App", () => {
  it("renders the dock placeholder buttons", () => {
    render(<App />);

    expect(
      screen.getByRole("button", { name: "Configurações" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Sobre" })).toBeInTheDocument();
  });
});
