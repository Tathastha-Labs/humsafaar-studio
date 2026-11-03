import { describe, expect, it } from "vitest";
import { cleanRefined, refineRequest } from "./refineShot";

describe("refineShot", () => {
  it("asks for the change next to the current shot", () => {
    expect(refineRequest(" Wide pier ", " make it night ")).toBe("Current shot: Wide pier\nChange to make: make it night");
  });
  it("keeps only the description from a chatty reply", () => {
    expect(cleanRefined('1. "Wide pier at night"\nHope that helps!')).toBe("Wide pier at night");
  });
});
