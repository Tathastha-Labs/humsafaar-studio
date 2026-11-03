/** Where shot `i` ends up after swapping shots `a` and `b`. */
export const swapped = (a: number, b: number) => (i: number) => (i === a ? b : i === b ? a : i);

/** Where shot `i` ends up after deleting shot `d`; null when it is the deleted one. */
export const afterDelete = (d: number) => (i: number) => (i === d ? null : i > d ? i - 1 : i);
