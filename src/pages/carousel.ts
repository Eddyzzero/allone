/** Le carrousel tourne seul, sauf survol, focus à l'intérieur, ou moins de deux jeux. */
export function autoplayOn(hover: boolean, focused: boolean, count: number): boolean {
  return !hover && !focused && count >= 2;
}

/** Vrai si le focus quitte vraiment la section (la cible suivante est hors de celle-ci). */
export function focusLeaves(section: Node, next: EventTarget | null): boolean {
  return !(next instanceof Node && section.contains(next));
}
