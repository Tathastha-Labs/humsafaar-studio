// One shot per non-empty line, without the list numbering the model adds.
export const splitShots = (text: string): string[] =>
  text
    .split("\n")
    .map((line) => line.replace(/^\s*(?:\d+[.)]|[-*])\s*/, "").trim())
    .filter(Boolean);
