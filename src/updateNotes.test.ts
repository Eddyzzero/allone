import { describe, expect, it } from "vitest";
import { CHECK_EVERY_MS, FOCUS_RECHECK_MS, parseNotes, shouldCheck, whatsNewOnStart } from "./updateNotes";

describe("shouldCheck", () => {
  it("vérifie la première fois", () => {
    expect(shouldCheck(1000, null, FOCUS_RECHECK_MS)).toBe(true);
  });
  it("attend l'écart minimal", () => {
    expect(shouldCheck(FOCUS_RECHECK_MS - 1, 0, FOCUS_RECHECK_MS)).toBe(false);
    expect(shouldCheck(FOCUS_RECHECK_MS, 0, FOCUS_RECHECK_MS)).toBe(true);
    expect(CHECK_EVERY_MS).toBeGreaterThan(FOCUS_RECHECK_MS);
  });
});

describe("parseNotes", () => {
  it("garde les points d'une section du CHANGELOG", () => {
    const body = "- Covers fill the whole card.\n- **Gamepad** support.\n\n";
    expect(parseNotes(body)).toEqual(["Covers fill the whole card.", "Gamepad support."]);
  });
  it("accepte un texte sans puces, ignore les titres et les retours Windows", () => {
    expect(parseNotes("## 0.3.1\r\nUne ligne\r\n\r\nUne autre")).toEqual(["Une ligne", "Une autre"]);
  });
  it("rend une liste vide pour un texte vide", () => {
    expect(parseNotes("")).toEqual([]);
  });
});

describe("whatsNewOnStart", () => {
  const pending = { version: "0.3.2", notes: "- A\n- B" };
  it("rien au tout premier lancement", () => {
    expect(whatsNewOnStart("0.3.2", null, null)).toBeNull();
  });
  it("rien si la version n'a pas changé", () => {
    expect(whatsNewOnStart("0.3.2", "0.3.2", pending)).toBeNull();
  });
  it("montre le texte de la mise à jour installée depuis allone", () => {
    expect(whatsNewOnStart("0.3.2", "0.3.1", pending)).toEqual({ version: "0.3.2", notes: ["A", "B"] });
  });
  it("signale une nouvelle version même sans texte connu", () => {
    expect(whatsNewOnStart("0.3.2", "0.3.1", null)).toEqual({ version: "0.3.2", notes: [] });
    expect(whatsNewOnStart("0.3.2", "0.3.1", { version: "0.3.1", notes: "- vieux" })).toEqual({ version: "0.3.2", notes: [] });
  });
  it("montre le texte même si l'ancienne version n'était pas mémorisée", () => {
    expect(whatsNewOnStart("0.3.2", null, pending)).toEqual({ version: "0.3.2", notes: ["A", "B"] });
  });
});
