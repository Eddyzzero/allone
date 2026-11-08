import { describe, expect, it } from "vitest";
import {
  axisToDir,
  buttonGlyphs,
  createRepeater,
  shouldFocusAfterNav,
  padFamily,
  padName,
  pickNeighbor,
  type Rect,
} from "./logic";

const XBOX = "Xbox 360 Controller (XInput STANDARD GAMEPAD)";
const DS = "DualSense Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 0ce6)";
const GENERIC = "Generic USB Joystick";

describe("padFamily / padName", () => {
  it("détecte la famille", () => {
    expect(padFamily(XBOX)).toBe("xbox");
    expect(padFamily(DS)).toBe("playstation");
    expect(padFamily(GENERIC)).toBe("generic");
  });
  it("nomme la manette", () => {
    expect(padName(XBOX)).toBe("Manette Xbox");
    expect(padName(DS)).toBe("DualSense");
    expect(padName(GENERIC)).toBe("Manette");
    expect(padName("DualShock 4 v2")).toBe("DualShock 4");
  });
  it("glyphes des boutons", () => {
    expect(buttonGlyphs("playstation")).toEqual({ confirm: "✕", back: "○" });
    expect(buttonGlyphs("xbox")).toEqual({ confirm: "A", back: "B" });
    expect(buttonGlyphs("generic")).toEqual({ confirm: "A", back: "B" });
  });
});

describe("axisToDir", () => {
  it("ignores drift", () => {
    expect(axisToDir(0.3, -0.2)).toBeNull();
    expect(axisToDir(0.8, 0.1)).toBe("right");
    expect(axisToDir(0.2, -0.9)).toBe("up");
  });
});

describe("pickNeighbor", () => {
  // grille 3 colonnes x 2 lignes, cartes 100x150 espacées de 20
  const cards: Rect[] = [];
  for (let r = 0; r < 2; r++) {
    for (let c = 0; c < 3; c++) {
      const left = c * 120;
      const top = r * 170;
      cards.push({ left, top, right: left + 100, bottom: top + 150 });
    }
  }
  // index de la carte choisie dans la grille complète (-1 si aucune)
  const pick = (from: number, dir: "up" | "down" | "left" | "right") => {
    const rest = cards.filter((_, k) => k !== from);
    const i = pickNeighbor(cards[from], rest, dir);
    return i < 0 ? -1 : cards.indexOf(rest[i]);
  };

  it("se déplace dans la grille", () => {
    expect(pick(0, "right")).toBe(1);
    expect(pick(0, "down")).toBe(3);
    expect(pick(0, "left")).toBe(-1);
  });
  it("préfère la carte alignée", () => {
    expect(pick(4, "up")).toBe(1);
  });
});

describe("createRepeater", () => {
  it("répète après le délai initial", () => {
    const r = createRepeater();
    expect(r.step(true, 0)).toBe(true);
    expect(r.step(true, 300)).toBe(false);
    expect(r.step(true, 400)).toBe(true);
    expect(r.step(true, 520)).toBe(true);
    expect(r.step(true, 560)).toBe(false);
  });
  it("ne rattrape pas les pas manqués après un gel", () => {
    const r = createRepeater();
    expect(r.step(true, 0)).toBe(true);
    expect(r.step(true, 5000)).toBe(true);
    expect(r.step(true, 5010)).toBe(false);
  });
  it("repart à zéro au relâchement", () => {
    const r = createRepeater();
    r.step(true, 0);
    r.step(true, 400);
    expect(r.step(false, 600)).toBe(false);
    expect(r.step(true, 610)).toBe(true);
  });
});

describe("shouldFocusAfterNav", () => {
  it("is false when nothing is armed", () => {
    expect(shouldFocusAfterNav(null, "/game/1")).toBe(false);
  });
  it("is false when the pathname did not change", () => {
    expect(shouldFocusAfterNav("/", "/")).toBe(false);
  });
  it("is true when armed and the pathname changed", () => {
    expect(shouldFocusAfterNav("/", "/game/1")).toBe(true);
  });
});
