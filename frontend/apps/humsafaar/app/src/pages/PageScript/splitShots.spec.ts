import { describe, expect, it } from "vitest";
import { splitShots } from "./splitShots";

describe("splitShots", () => {
  it("drops list numbering, bullets and blank lines", () => {
    expect(splitShots("1. Wide shot of a pier\n\n2) Close on her hands\n- Dawn light")).toEqual([
      "Wide shot of a pier",
      "Close on her hands",
      "Dawn light",
    ]);
  });
});
