import { describe, expect, it } from "vitest";
import { dragOffset, dropIndex, dropMark } from "./drag";

describe("dropIndex", () => {
  // Four programs at rest, centres at 10, 30, 50 and 70.
  const centers = [10, 30, 50, 70];

  it("counts the other programs whose centre is before the pointer", () => {
    expect(dropIndex(centers, 0, 0)).toBe(0);
    expect(dropIndex(centers, 0, 35)).toBe(1);
    expect(dropIndex(centers, 0, 100)).toBe(3);
  });

  it("does not count the dragged program itself", () => {
    // Over its own place it is where it was.
    expect(dropIndex(centers, 1, 30)).toBe(1);
    expect(dropIndex(centers, 2, 50)).toBe(2);
    // Dragged left, it takes the place of the one it passes.
    expect(dropIndex(centers, 2, 20)).toBe(1);
    expect(dropIndex(centers, 3, 5)).toBe(0);
  });

  it("is the end past the last program and the start before the first", () => {
    expect(dropIndex(centers, 1, 1000)).toBe(3);
    expect(dropIndex(centers, 1, -1000)).toBe(0);
  });

  it("is where it is with a single program", () => {
    expect(dropIndex([10], 0, 500)).toBe(0);
  });
});

describe("dropMark", () => {
  it("draws nothing when the program would stay", () => {
    expect(dropMark(4, 2, 2)).toBeNull();
    expect(dropMark(1, 0, 0)).toBeNull();
  });

  it("stands before the program the dragged one would take the place of", () => {
    expect(dropMark(4, 3, 1)).toEqual({ index: 1, side: "before" });
    expect(dropMark(4, 3, 0)).toEqual({ index: 0, side: "before" });
    // Dragged right over one: before the one after it.
    expect(dropMark(4, 0, 1)).toEqual({ index: 2, side: "before" });
  });

  it("stands after the last program for the end of the bar", () => {
    expect(dropMark(4, 0, 3)).toEqual({ index: 3, side: "after" });
    expect(dropMark(3, 1, 2)).toEqual({ index: 2, side: "after" });
  });
});

describe("dragOffset", () => {
  // A tile 20 wide at rest from 40, in a bar that goes from 0 to 200.
  it("follows the pointer from where the drag began", () => {
    expect(dragOffset(130, 100, 40, 20, 0, 200)).toBe(30);
    expect(dragOffset(70, 100, 40, 20, 0, 200)).toBe(-30);
  });

  it("stops at the ends of the bar", () => {
    expect(dragOffset(500, 100, 40, 20, 0, 200)).toBe(140);
    expect(dragOffset(-500, 100, 40, 20, 0, 200)).toBe(-40);
  });
});
