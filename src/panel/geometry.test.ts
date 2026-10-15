import { describe, expect, it } from "vitest";
import css from "./Panel.css?raw";
import { PANEL_INSET, measureFrame } from "./geometry";

describe("panel geometry", () => {
  it("keeps the CSS inset equal to the one sent to Rust", () => {
    const declared = /--panel-inset:\s*(\d+)px/.exec(css);

    expect(declared?.[1]).toBe(String(PANEL_INSET));
  });

  it("rounds the frame size up to whole pixels", () => {
    const frame = document.createElement("div");
    frame.getBoundingClientRect = () =>
      ({ width: 352, height: 401.2 }) as DOMRect;

    expect(measureFrame(frame)).toEqual({ width: 352, height: 402 });
  });
});
