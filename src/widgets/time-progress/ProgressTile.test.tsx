import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import tokens from "../../themes/tokens.css?raw";
import css from "./time-progress.css?raw";
import ProgressTile from "./ProgressTile";
import { FRIDAY_NOON, SATURDAY_NOON, progressInstance } from "./testing";

const CIRCUMFERENCE = 2 * Math.PI * 9;

function tile(instance = progressInstance(), now = FRIDAY_NOON) {
  const { container } = render(<ProgressTile instance={instance} now={now} />);
  const root = container.firstElementChild as HTMLElement;
  return {
    root,
    arc: root.querySelector(".progress-ring-arc"),
    track: root.querySelector(".progress-ring-track"),
    pct: root.querySelector(".progress-tile-pct"),
  };
}

describe("ProgressTile", () => {
  it("is decorative: the item's name comes from the dock", () => {
    const { root } = tile();
    expect(root).toHaveAttribute("aria-hidden", "true");
  });

  it("draws the first period's ring with its percentage in the center", () => {
    const { arc, track, pct } = tile();

    expect(track).not.toBeNull();
    const [dash, gap] = (arc?.getAttribute("stroke-dasharray") ?? "")
      .split(" ")
      .map(Number);
    expect(dash).toBeCloseTo(CIRCUMFERENCE / 2, 1);
    expect(gap).toBeCloseTo(CIRCUMFERENCE, 1);
    expect(pct).toHaveTextContent(/^50%$/);
  });

  it("follows the first period only, rounding down", () => {
    // Workday 09:00–18:00 at 12:00: 33.3%.
    const { pct } = tile(progressInstance({ periods: ["workday", "day"] }));
    expect(pct).toHaveTextContent(/^33%$/);
  });

  it("shows an empty ring and a dash outside the workday", () => {
    const { arc, track, pct } = tile(
      progressInstance({ periods: ["workday", "day"] }),
      SATURDAY_NOON,
    );

    expect(track).not.toBeNull();
    expect(arc).toBeNull();
    expect(pct).toHaveTextContent(/^–$/);
  });

  it("follows the instant it is given", () => {
    const instance = progressInstance();
    const { rerender, container } = render(
      <ProgressTile instance={instance} now={FRIDAY_NOON} />,
    );
    rerender(
      <ProgressTile instance={instance} now={new Date(2026, 9, 16, 18, 0)} />,
    );
    expect(container.querySelector(".progress-tile-pct")).toHaveTextContent(
      /^75%$/,
    );
  });
});

describe("time-progress.css", () => {
  it("hides the number in the Pequeno size, keeping the ring", () => {
    expect(css).toMatch(
      /:root\[data-size="s"\]\s+\.progress-tile-pct\s*\{\s*display:\s*none;/,
    );
  });

  it("uses only tokens that tokens.css defines, plus the bar's own --f", () => {
    const defined = new Set(
      [...tokens.matchAll(/(--[\w-]+)\s*:/g)].map((match) => match[1]),
    );
    defined.add("--f");
    const used = [...css.matchAll(/var\((--[\w-]+)/g)].map((match) => match[1]);
    expect(used.length).toBeGreaterThan(0);
    expect(used.filter((token) => !defined.has(token))).toEqual([]);
  });

  it("writes no color literal: colors come from the tokens", () => {
    expect(css).not.toMatch(/#[0-9a-f]{3,8}\b|\brgba?\(|\bhsla?\(/i);
  });
});
