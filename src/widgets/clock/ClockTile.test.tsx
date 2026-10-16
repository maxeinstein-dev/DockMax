import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { ClockConfig, WidgetInstance } from "../../core/widgets";
import tokens from "../../themes/tokens.css?raw";
import css from "./clock.css?raw";
import ClockTile from "./ClockTile";

// 2026-10-15 18:07:05 UTC = 15:07:05 in São Paulo.
const NOW = new Date(Date.UTC(2026, 9, 15, 18, 7, 5));

const clock = (config: Partial<ClockConfig> = {}): WidgetInstance<"clock"> => ({
  id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6071",
  label: null,
  kind: "clock",
  config: {
    timeZone: "America/Sao_Paulo",
    hourCycle: "h23",
    showSeconds: false,
    iconStyle: "digital",
    ...config,
  },
});

describe("ClockTile, digital", () => {
  it("shows HH:mm in 24 h, never the seconds", () => {
    const { container } = render(<ClockTile instance={clock()} now={NOW} />);
    expect(container).toHaveTextContent(/^15:07$/);
  });

  it("pads the hour in 24 h", () => {
    const morning = new Date(Date.UTC(2026, 9, 15, 12, 5));
    const { container } = render(
      <ClockTile instance={clock()} now={morning} />,
    );
    expect(container).toHaveTextContent(/^09:05$/);
  });

  it("shows h:mm in 12 h, without AM/PM", () => {
    const { container } = render(
      <ClockTile instance={clock({ hourCycle: "h12" })} now={NOW} />,
    );
    expect(container).toHaveTextContent(/^3:07$/);
  });

  it("ignores the panel's seconds setting", () => {
    const { container } = render(
      <ClockTile instance={clock({ showSeconds: true })} now={NOW} />,
    );
    expect(container).toHaveTextContent(/^15:07$/);
  });

  it("is decorative: the item's name is the label, never the time", () => {
    const { container } = render(<ClockTile instance={clock()} now={NOW} />);
    const face = container.firstElementChild;
    expect(face).toHaveAttribute("aria-hidden", "true");
    expect(face).toHaveClass("clock-tile-time");
  });
});

describe("ClockTile, analog", () => {
  const analog = (config: Partial<ClockConfig> = {}) =>
    render(
      <ClockTile
        instance={clock({ iconStyle: "analog", ...config })}
        now={NOW}
      />,
    );

  it("draws a dial without numbers, decorative", () => {
    const { container } = analog();
    const face = container.firstElementChild;
    expect(face?.tagName.toLowerCase()).toBe("svg");
    expect(face).toHaveAttribute("aria-hidden", "true");
    expect(face).toHaveAttribute("viewBox", "0 0 24 24");
    expect(face).toHaveClass("clock-tile-dial");
    expect(container).toHaveTextContent("");
  });

  it("has four marks, an hour and a minute hand, and no second hand", () => {
    const { container } = analog();
    expect(container.querySelectorAll(".clock-mark")).toHaveLength(4);
    expect(container.querySelectorAll("line")).toHaveLength(2);
  });

  it("points the hands at the time in the clock's zone", () => {
    const { container } = analog();
    // 15:07: minute hand at 42°, length 7.6; hour hand at 93.5°, length 5.
    const minute = container.querySelector(".clock-hand-minute");
    expect(minute).toHaveAttribute("x2", "17.09");
    expect(minute).toHaveAttribute("y2", "6.35");
    const hour = container.querySelector(".clock-hand-hour");
    expect(hour).toHaveAttribute("x2", "16.99");
    expect(hour).toHaveAttribute("y2", "12.31");
  });

  it("points the same way in 12 h", () => {
    const twelve = analog({ hourCycle: "h12" }).container;
    expect(twelve.querySelector(".clock-hand-hour")).toHaveAttribute(
      "x2",
      "16.99",
    );
  });

  it("points at 12 at midnight", () => {
    const midnight = new Date(Date.UTC(2026, 9, 15, 3, 0));
    const { container } = render(
      <ClockTile instance={clock({ iconStyle: "analog" })} now={midnight} />,
    );
    const hour = container.querySelector(".clock-hand-hour");
    expect(hour).toHaveAttribute("x2", "12.00");
    expect(hour).toHaveAttribute("y2", "7.00");
  });
});

describe("clock.css", () => {
  it("uses only tokens that tokens.css defines", () => {
    const defined = new Set(
      [...tokens.matchAll(/(--[\w-]+)\s*:/g)].map((match) => match[1]),
    );
    const used = [...css.matchAll(/var\((--[\w-]+)/g)].map((match) => match[1]);
    expect(used.length).toBeGreaterThan(0);
    expect(used.filter((token) => !defined.has(token))).toEqual([]);
  });
});
