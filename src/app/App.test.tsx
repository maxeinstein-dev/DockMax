import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("App", () => {
  it("renders the placeholder", () => {
    render(<App />);

    expect(
      screen.getByRole("heading", { name: "DockMax" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Fase 0/)).toBeInTheDocument();
  });
});
