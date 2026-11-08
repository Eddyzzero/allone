// @vitest-environment jsdom
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

type State = { pad: { name: string; family: "xbox" | "playstation" | "generic" } | null; active: boolean };
const state = vi.hoisted(() => ({ value: { pad: null, active: false } as State }));
vi.mock("../gamepad/GamepadProvider", () => ({ useGamepad: () => state.value }));

import PadHint from "./PadHint";

describe("PadHint", () => {
  it("ne rend rien sans manette active", () => {
    state.value = { pad: null, active: false };
    expect(renderToStaticMarkup(<PadHint action="confirm" />)).toBe("");
    state.value = { pad: { name: "Manette Xbox", family: "xbox" }, active: false };
    expect(renderToStaticMarkup(<PadHint action="confirm" />)).toBe("");
  });
  it("rend ✕ et ○ pour une manette PlayStation", () => {
    state.value = { pad: { name: "Manette PlayStation", family: "playstation" }, active: true };
    expect(renderToStaticMarkup(<PadHint action="confirm" />)).toContain("✕");
    expect(renderToStaticMarkup(<PadHint action="back" />)).toContain("○");
  });
  it("rend A pour une manette Xbox", () => {
    state.value = { pad: { name: "Manette Xbox", family: "xbox" }, active: true };
    expect(renderToStaticMarkup(<PadHint action="confirm" />)).toContain(">A<");
  });
});
