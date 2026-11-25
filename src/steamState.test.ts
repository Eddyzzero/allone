import { describe, expect, it, vi } from "vitest";
import * as api from "./api";
import { installSteam, isNotInstalled, steamAppId } from "./steamState";
import type { Game } from "./types";

vi.mock("./api", () => ({ installSteamGame: vi.fn() }));

const game = (over: Partial<Game>): Game =>
  ({ id: "g1", title: "Jeu", systemId: "steam", kind: "store", path: "", store: { store: "steam", id: "1426210" }, ...over }) as Game;

describe("isNotInstalled", () => {
  it("vrai pour un jeu Steam dont le manifeste n'est pas trouvé", () => {
    expect(isNotInstalled(game({}), { g1: false })).toBe(true);
  });
  it("faux pour un jeu Steam installé", () => {
    expect(isNotInstalled(game({}), { g1: true })).toBe(false);
  });
  it("faux tant que l'état n'est pas connu", () => {
    expect(isNotInstalled(game({}), {})).toBe(false);
  });
  it("faux pour un jeu émulé introuvable (c'est « Introuvable », pas « Non installé »)", () => {
    expect(isNotInstalled(game({ kind: "emulated", store: null }), { g1: false })).toBe(false);
  });
});

describe("installSteam", () => {
  it("ouvre l'installation Steam du jeu et prévient", async () => {
    vi.mocked(api.installSteamGame).mockResolvedValueOnce();
    const notify = vi.fn();
    await installSteam(game({ title: "Devil May Cry 5", store: { store: "steam", id: "601150" } }), notify);
    expect(api.installSteamGame).toHaveBeenCalledWith(601150);
    expect(notify).toHaveBeenCalledWith("Steam ouvre l'installation de Devil May Cry 5.");
  });
  it("affiche l'erreur si Steam ne s'ouvre pas", async () => {
    vi.mocked(api.installSteamGame).mockRejectedValueOnce("Impossible d'ouvrir Steam : x");
    const notify = vi.fn();
    await installSteam(game({}), notify);
    expect(notify).toHaveBeenCalledWith("Impossible d'ouvrir Steam : x", "error");
  });
});

describe("steamAppId", () => {
  it("lit l'identifiant Steam", () => {
    expect(steamAppId(game({}))).toBe(1426210);
  });
  it("null sans boutique Steam", () => {
    expect(steamAppId(game({ store: null }))).toBeNull();
  });
});
