import { describe, expect, it } from "vitest";
import { createDriverSelector, createPadReader, type PadSnapshot } from "./reader";

function snap(pressed: number[] = [], axes: number[] = [0, 0]): PadSnapshot {
  const buttons = Array.from({ length: 16 }, (_, i) => pressed.includes(i));
  return { buttons, axes };
}

describe("createPadReader", () => {
  it("A enfoncé deux lectures de suite : un seul confirm", () => {
    const r = createPadReader();
    expect(r.read(snap([0]), 0, false)).toEqual([{ type: "confirm" }]);
    expect(r.read(snap([0]), 16, false)).toEqual([]);
  });

  it("B donne back", () => {
    const r = createPadReader();
    expect(r.read(snap([1]), 0, false)).toEqual([{ type: "back" }]);
  });

  it("croix droite maintenue : répétition à 0, 400 puis 520 ms", () => {
    const r = createPadReader();
    const fired: number[] = [];
    for (let t = 0; t <= 600; t += 8) {
      const a = r.read(snap([15]), t, false);
      if (a.some((x) => x.type === "move" && x.dir === "right")) fired.push(t);
    }
    expect(fired).toEqual([0, 400, 520]);
  });

  it("stick (0.9, 0) donne move right", () => {
    const r = createPadReader();
    expect(r.read(snap([], [0.9, 0]), 0, false)).toEqual([{ type: "move", dir: "right", repeat: false }]);
  });

  it("premier pas repeat=false, pas suivants repeat=true", () => {
    const r = createPadReader();
    expect(r.read(snap([15]), 0, false)).toEqual([{ type: "move", dir: "right", repeat: false }]);
    expect(r.read(snap([15]), 400, false)).toEqual([{ type: "move", dir: "right", repeat: true }]);
    r.read(snap(), 500, false);
    expect(r.read(snap([15]), 600, false)).toEqual([{ type: "move", dir: "right", repeat: false }]);
  });

  it("LB donne section -1, RB section 1", () => {
    const r = createPadReader();
    expect(r.read(snap([4]), 0, false)).toEqual([{ type: "section", delta: -1 }]);
    r.read(snap(), 16, false);
    expect(r.read(snap([5]), 32, false)).toEqual([{ type: "section", delta: 1 }]);
  });

  it.each([
    ["croix droite", () => snap([15]), () => snap()],
    ["stick droit", () => snap([], [0.9, 0]), () => snap()],
  ])("ignored state swallows held direction (%s)", (_n, held, neutral) => {
    const r = createPadReader();
    expect(r.read(held(), 0, true)).toEqual([]);
    for (let t = 16; t <= 600; t += 100) expect(r.read(held(), t, false)).toEqual([]);
    expect(r.read(neutral(), 700, false)).toEqual([]);
    expect(r.read(held(), 716, false)).toEqual([{ type: "move", dir: "right", repeat: false }]);
  });

  it("ignored state swallows held buttons", () => {
    const r = createPadReader();
    expect(r.read(snap([0]), 0, true)).toEqual([]);
    expect(r.read(snap([0]), 16, false)).toEqual([]);
    expect(r.read(snap(), 32, false)).toEqual([]);
    expect(r.read(snap([0]), 48, false)).toEqual([{ type: "confirm" }]);
  });
});

describe("createDriverSelector", () => {
  it("le bruit de stick d'une autre manette ne la fait pas prendre la main", () => {
    const sel = createDriverSelector();
    sel.pick([{ index: 0, snap: snap() }, { index: 1, snap: snap() }]);
    for (let i = 0; i < 20; i++) {
      const j = i % 2 ? 0.05 : -0.05;
      const r = sel.pick([{ index: 0, snap: snap() }, { index: 1, snap: snap([], [j, -j]) }]);
      expect(r).toEqual({ driver: 0, switched: false });
    }
  });

  it("un appui sur l'autre manette lui donne la main", () => {
    const sel = createDriverSelector();
    sel.pick([{ index: 0, snap: snap() }, { index: 1, snap: snap() }]);
    const r = sel.pick([{ index: 0, snap: snap() }, { index: 1, snap: snap([0]) }]);
    expect(r).toEqual({ driver: 1, switched: true });
  });

  it("A tenu sur la manette 1 ne redéclenche pas après un aller-retour", () => {
    const sel = createDriverSelector();
    const reader = createPadReader();
    const frame = (a: PadSnapshot, b: PadSnapshot, t: number) => {
      const { driver, switched } = sel.pick([{ index: 0, snap: a }, { index: 1, snap: b }]);
      const cur = driver === 1 ? b : a;
      if (switched) {
        reader.swallowHeld(cur);
        return [];
      }
      return reader.read(cur, t, false);
    };
    expect(frame(snap(), snap(), 0)).toEqual([]);
    expect(frame(snap([0]), snap(), 16)).toEqual([{ type: "confirm" }]);
    // la manette 2 prend la main (appui sur B), A reste tenu sur la 1
    expect(frame(snap([0]), snap([1]), 32)).toEqual([]);
    // la manette 1 reprend la main (nouvel appui sur LB) avec A toujours tenu
    expect(frame(snap([0, 4]), snap([]), 48)).toEqual([]);
    expect(frame(snap([0]), snap([]), 64)).toEqual([]);
  });
});
