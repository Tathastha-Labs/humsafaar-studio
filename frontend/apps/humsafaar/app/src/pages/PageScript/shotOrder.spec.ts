import { describe, expect, it } from "vitest";
import { afterDelete, swapped } from "./shotOrder";

describe("shotOrder", () => {
  it("swaps two positions and leaves the rest", () => {
    expect([0, 1, 2].map(swapped(0, 1))).toEqual([1, 0, 2]);
  });
  it("drops the deleted position and shifts later ones up", () => {
    expect([0, 1, 2].map(afterDelete(1))).toEqual([0, null, 1]);
  });
});
