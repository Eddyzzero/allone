// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { autoplayOn, focusLeaves } from "./carousel";

describe("carrousel", () => {
  it("autoplay en pause au survol ou au focus", () => {
    expect(autoplayOn(false, false, 3)).toBe(true);
    expect(autoplayOn(true, false, 3)).toBe(false);
    expect(autoplayOn(false, true, 3)).toBe(false);
    expect(autoplayOn(false, false, 1)).toBe(false);
  });
  it("le focus ne quitte pas la section vers un de ses descendants", () => {
    const s = document.createElement("section");
    const b = document.createElement("button");
    s.append(b);
    expect(focusLeaves(s, b)).toBe(false);
    expect(focusLeaves(s, document.body)).toBe(true);
    expect(focusLeaves(s, null)).toBe(true);
  });
});
