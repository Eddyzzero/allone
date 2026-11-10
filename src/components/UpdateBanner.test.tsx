// @vitest-environment jsdom
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

const noop = () => {};
const base = {
  available: null as { version: string; notes: string } | null,
  dismissed: false, dismiss: noop, ready: false, checking: false,
  downloading: null as { done: number; total: number } | null,
  check: async () => null, install: async () => {},
  whatsNew: null as { version: string; notes: string[] } | null, closeWhatsNew: noop,
};
const state = vi.hoisted(() => ({ updater: {} as Record<string, unknown>, running: new Set<string>() }));
vi.mock("../updater", () => ({ useUpdater: () => state.updater }));
vi.mock("../store", () => ({ useStore: () => ({ running: state.running }) }));

import UpdateBanner from "./UpdateBanner";

const render = (patch: Partial<typeof base>, running: string[] = []) => {
  state.updater = { ...base, ...patch };
  state.running = new Set(running);
  return renderToStaticMarkup(<UpdateBanner />);
};

describe("UpdateBanner", () => {
  it("ne rend rien sans mise à jour ni nouveautés", () => {
    expect(render({})).toBe("");
  });
  it("annonce la version, ses nouveautés et le bouton", () => {
    const html = render({ available: { version: "0.4.0", notes: "- Un\n- Deux\n- Trois\n- Quatre\n- Cinq" } });
    expect(html).toContain("Nouvelle version 0.4.0 disponible");
    expect(html).toContain("<li>Un</li>");
    expect(html).not.toContain("<li>Quatre</li>");
    expect(html).toContain("et 2 autres changements");
    expect(html).toContain("Mettre à jour");
    expect(html).toContain("Plus tard");
  });
  it("se cache après « Plus tard »", () => {
    expect(render({ available: { version: "0.4.0", notes: "" }, dismissed: true })).toBe("");
  });
  it("désactive la mise à jour pendant un jeu", () => {
    const html = render({ available: { version: "0.4.0", notes: "" } }, ["g1"]);
    expect(html).toMatch(/<button[^>]*disabled[^>]*>Mettre à jour/);
    expect(html).toContain("Ferme le jeu en cours");
  });
  it("montre la progression du téléchargement", () => {
    expect(render({ available: { version: "0.4.0", notes: "" }, downloading: { done: 50, total: 200 } })).toContain("Téléchargement… 25 %");
  });
  it("affiche les nouveautés après une mise à jour", () => {
    const html = render({ whatsNew: { version: "0.4.0", notes: ["Un", "Deux"] } });
    expect(html).toContain("allone a été mis à jour en version 0.4.0");
    expect(html).toContain("<li>Deux</li>");
    expect(html).toContain("Fermer les nouveautés");
  });
});
