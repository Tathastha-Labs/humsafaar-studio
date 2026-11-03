export const REFINE_SYSTEM_PROMPT =
  "You rewrite one shot description for a film. Keep it a single self-contained visual description that an image model can use as its prompt. Reply with only the new description, no numbering, quotes or commentary.";

/** The message that asks the writing model to change one shot the way the user described. */
export const refineRequest = (shot: string, instruction: string): string =>
  `Current shot: ${shot.trim()}\nChange to make: ${instruction.trim()}`;

/** Models sometimes wrap the answer in quotes or a leading number; keep just the description. */
export const cleanRefined = (reply: string): string =>
  reply.trim().split("\n")[0].replace(/^\s*(?:\d+[.)]|[-*])\s*/, "").replace(/^["“](.*)["”]$/, "$1").trim();
