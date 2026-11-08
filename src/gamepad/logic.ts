// Logique pure de navigation à la manette (sans DOM, testable seule).

export type Dir = "up" | "down" | "left" | "right";
export type PadFamily = "xbox" | "playstation" | "generic";
export interface Rect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** Famille de manette d'après l'identifiant fourni par le navigateur. */
export function padFamily(id: string): PadFamily {
  const s = id.toLowerCase();
  if (s.includes("xbox") || s.includes("xinput") || s.includes("045e")) return "xbox";
  if (
    s.includes("dualsense") ||
    s.includes("dualshock") ||
    s.includes("wireless controller") ||
    s.includes("054c")
  ) {
    return "playstation";
  }
  return "generic";
}

/** Nom lisible de la manette. */
export function padName(id: string): string {
  const s = id.toLowerCase();
  if (s.includes("dualsense")) return "DualSense";
  if (s.includes("dualshock")) return "DualShock 4";
  switch (padFamily(id)) {
    case "xbox":
      return "Manette Xbox";
    case "playstation":
      return "Manette PlayStation";
    default:
      return "Manette";
  }
}

/** Glyphes des boutons de validation / retour selon la famille. */
export function buttonGlyphs(f: PadFamily): { confirm: string; back: string } {
  return f === "playstation" ? { confirm: "✕", back: "○" } : { confirm: "A", back: "B" };
}

/** Direction dominante d'un stick ; null sous la zone morte. y négatif = haut. */
export function axisToDir(x: number, y: number, deadzone = 0.5): Dir | null {
  const ax = Math.abs(x);
  const ay = Math.abs(y);
  if (Math.max(ax, ay) < deadzone) return null;
  if (ax >= ay) return x > 0 ? "right" : "left";
  return y > 0 ? "down" : "up";
}

/** Index du meilleur candidat dans la direction donnée, ou -1. */
export function pickNeighbor(from: Rect, candidates: Rect[], dir: Dir): number {
  const horizontal = dir === "left" || dir === "right";
  const TOL = 1;
  let best = -1;
  let bestScore = Infinity;
  candidates.forEach((c, i) => {
    let main: number;
    switch (dir) {
      case "right":
        if (c.left < from.right - TOL) return;
        main = c.left - from.right;
        break;
      case "left":
        if (c.right > from.left + TOL) return;
        main = from.left - c.right;
        break;
      case "down":
        if (c.top < from.bottom - TOL) return;
        main = c.top - from.bottom;
        break;
      default:
        if (c.bottom > from.top + TOL) return;
        main = from.top - c.bottom;
    }
    // Axe perpendiculaire : 0 si les projections se chevauchent, sinon écart des centres.
    const [fa, fb, ca, cb] = horizontal
      ? [from.top, from.bottom, c.top, c.bottom]
      : [from.left, from.right, c.left, c.right];
    const overlap = ca < fb && cb > fa;
    const offset = overlap ? 0 : Math.abs((ca + cb) / 2 - (fa + fb) / 2);
    const score = Math.max(0, main) + 2 * offset;
    if (score < bestScore) {
      bestScore = score;
      best = i;
    }
  });
  return best;
}

/** Faut-il redonner le focus au contenu ? Oui si une action manette a armé le drapeau depuis un autre chemin. */
export function shouldFocusAfterNav(armedFrom: string | null, pathname: string): boolean {
  return armedFrom !== null && armedFrom !== pathname;
}

/** Répétition d'une direction maintenue : un pas immédiat, puis après un délai, à intervalle fixe. */
export function createRepeater(initialDelay = 400, interval = 120) {
  let next: number | null = null;
  return {
    step(pressed: boolean, now: number): boolean {
      if (!pressed) {
        next = null;
        return false;
      }
      if (next === null) {
        next = now + initialDelay;
        return true;
      }
      if (now >= next) {
        next += interval;
        // Après un gel, pas de rafale de rattrapage : un seul pas par appel.
        if (next <= now) next = now + interval;
        return true;
      }
      return false;
    },
    reset(): void {
      next = null;
    },
  };
}
