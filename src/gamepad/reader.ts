// Lecture pure d'un instantané de manette « standard » : transforme l'état en actions (sans DOM).
import { axisToDir, createRepeater, type Dir } from "./logic";

export type PadAction =
  | { type: "move"; dir: Dir; repeat: boolean }
  | { type: "confirm" }
  | { type: "back" }
  | { type: "section"; delta: 1 | -1 };

export interface PadSnapshot {
  buttons: boolean[];
  axes: number[];
}

const BTN_A = 0;
const BTN_B = 1;
const BTN_LB = 4;
const BTN_RB = 5;
const DPAD: [number, Dir][] = [
  [12, "up"],
  [13, "down"],
  [14, "left"],
  [15, "right"],
];
const DIRS: Dir[] = ["up", "down", "left", "right"];

export function createPadReader() {
  const repeaters: Record<Dir, ReturnType<typeof createRepeater>> = {
    up: createRepeater(),
    down: createRepeater(),
    left: createRepeater(),
    right: createRepeater(),
  };
  let prev: boolean[] = [];
  const swallowed = new Set<Dir>();
  // Directions tenues depuis un pas déjà émis : les pas suivants sont des répétitions.
  const heldPrev = new Set<Dir>();

  const edge = (s: PadSnapshot, i: number) => !!s.buttons[i] && !prev[i];

  return {
    read(s: PadSnapshot, now: number, ignored: boolean): PadAction[] {
      const stick = axisToDir(s.axes[0] ?? 0, s.axes[1] ?? 0, 0.5);
      const heldDir = (d: Dir) =>
        DPAD.some(([i, dir]) => dir === d && !!s.buttons[i]) || stick === d;
      if (ignored) {
        // Boutons et directions déjà tenus ne déclenchent rien tant qu'ils ne sont pas relâchés.
        prev = s.buttons.slice();
        DIRS.forEach((d) => {
          repeaters[d].reset();
          heldPrev.delete(d);
          if (heldDir(d)) swallowed.add(d);
        });
        return [];
      }
      const actions: PadAction[] = [];
      if (edge(s, BTN_A)) actions.push({ type: "confirm" });
      if (edge(s, BTN_B)) actions.push({ type: "back" });
      if (edge(s, BTN_LB)) actions.push({ type: "section", delta: -1 });
      if (edge(s, BTN_RB)) actions.push({ type: "section", delta: 1 });

      for (const d of DIRS) {
        const raw = heldDir(d);
        if (!raw) swallowed.delete(d);
        const held = raw && !swallowed.has(d);
        const wasHeld = heldPrev.has(d);
        if (held) heldPrev.add(d);
        else heldPrev.delete(d);
        if (repeaters[d].step(held, now)) actions.push({ type: "move", dir: d, repeat: wasHeld });
      }
      prev = s.buttons.slice();
      return actions;
    },
    /** Mémorise tout ce qui est tenu sans rien déclencher (changement de manette). */
    swallowHeld(s: PadSnapshot): void {
      this.read(s, 0, true);
    },
    reset(): void {
      prev = [];
      swallowed.clear();
      heldPrev.clear();
      DIRS.forEach((d) => repeaters[d].reset());
    },
  };
}

export interface PadInput {
  index: number;
  snap: PadSnapshot;
}

/**
 * Choix de la manette qui pilote : une manette ne prend la main que sur une vraie entrée
 * (nouvel appui, ou stick qui franchit la zone morte), jamais sur le bruit d'un stick au repos.
 */
export function createDriverSelector() {
  const prev = new Map<number, { buttons: boolean[]; dir: Dir | null }>();
  let driver: number | null = null;
  return {
    pick(pads: PadInput[]): { driver: number | null; switched: boolean } {
      let edgePad: number | null = null;
      const seen = new Set<number>();
      for (const { index, snap } of pads) {
        seen.add(index);
        const dir = axisToDir(snap.axes[0] ?? 0, snap.axes[1] ?? 0, 0.5);
        const old = prev.get(index);
        const pressedEdge = snap.buttons.some((b, i) => b && !old?.buttons[i]);
        const stickEdge = dir !== null && dir !== old?.dir;
        if (old && (pressedEdge || stickEdge)) edgePad = index;
        prev.set(index, { buttons: snap.buttons.slice(), dir });
      }
      for (const k of Array.from(prev.keys())) if (!seen.has(k)) prev.delete(k);

      if (driver === null || !seen.has(driver)) {
        driver = pads.length > 0 ? pads[0].index : null;
        return { driver, switched: false };
      }
      if (edgePad !== null && edgePad !== driver) {
        driver = edgePad;
        return { driver, switched: true };
      }
      return { driver, switched: false };
    },
  };
}
