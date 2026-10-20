import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import NoteTile from "./NoteTile";

describe("NoteTile", () => {
  it("is the fixed, decorative notes glyph", () => {
    const { container } = render(<NoteTile />);

    const svg = container.querySelector("svg");
    expect(svg).toHaveClass("dock-glyph");
    expect(svg).toHaveAttribute("viewBox", "0 0 24 24");
    expect(svg).toHaveAttribute("aria-hidden", "true");
    expect(container).toHaveTextContent("");
  });
});
