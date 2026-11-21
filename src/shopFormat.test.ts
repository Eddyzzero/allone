import { describe, expect, it } from "vitest";
import { frDate, offerEnds, priceOf, priceText, releaseText, reviewText, reviewTone, screenshotThumb } from "./shopFormat";
import type { ShopItem } from "./types";

const base: ShopItem = {
  appId: 1,
  name: "Jeu",
  developers: [],
  comingSoon: false,
  isFree: false,
  price: "29,99€",
  discountPct: 0,
  reviewCount: 10,
  reviewPct: 93,
  reviewLabel: "très positives",
  screenshots: [],
  deck: 0,
};

describe("shopFormat", () => {
  it("ajoute une espace insécable avant l'euro", () => {
    expect(priceText("29,99€")).toBe("29,99 €");
    expect(priceText("1 039,00 €")).toBe("1 039,00 €");
  });

  it("affiche gratuit, le prix, ou rien avant la mise en vente", () => {
    expect(priceOf({ ...base, isFree: true, price: null })).toBe("Gratuit");
    expect(priceOf(base)).toBe("29,99 €");
    expect(priceOf({ ...base, price: null })).toBeNull();
  });

  it("met une majuscule au libellé des évaluations", () => {
    expect(reviewText(base)).toBe("Très positives");
    expect(reviewText({ ...base, reviewCount: 0 })).toBe("Pas encore d'évaluations");
  });

  it("annonce la fin d'une promotion", () => {
    const now = 1_000_000;
    expect(offerEnds({ ...base, discountPct: 50, discountEnds: now + 5 * 3600 }, now)).toBe("Se termine dans 5 h");
    expect(offerEnds({ ...base, discountPct: 50, discountEnds: now + 600 }, now)).toBe("Se termine dans moins d'une heure");
    expect(offerEnds({ ...base, discountPct: 50, discountEnds: now - 1 }, now)).toBeNull();
    expect(offerEnds({ ...base, discountEnds: now + 99999 }, now)).toBeNull();
    expect(offerEnds({ ...base, discountPct: 30, discountEnds: now + 5 * 86400 }, now)).toMatch(/^Offre valable jusqu'au /);
  });

  it("écrit « 1er » pour le premier du mois", () => {
    const first = new Date(2026, 11, 1, 12).getTime() / 1000;
    const tenth = new Date(2026, 11, 10, 12).getTime() / 1000;
    expect(frDate(first)).toBe("1er décembre 2026");
    expect(frDate(first, false)).toBe("1er décembre");
    expect(frDate(tenth)).toBe("10 décembre 2026");
  });

  it("gère les jeux à venir sans date précise", () => {
    expect(releaseText({ ...base, comingSoon: true, releaseDate: null })).toBe("Date à annoncer");
  });

  it("demande la vignette 600×338 des captures", () => {
    expect(screenshotThumb("https://x/steam/apps/1/ss_a.jpg?t=1")).toBe("https://x/steam/apps/1/ss_a.600x338.jpg?t=1");
    expect(screenshotThumb("https://x/ss_b.jpg")).toBe("https://x/ss_b.600x338.jpg");
  });

  it("classe les évaluations", () => {
    expect(reviewTone(base)).toBe("good");
    expect(reviewTone({ ...base, reviewPct: 60 })).toBe("mixed");
    expect(reviewTone({ ...base, reviewPct: 20 })).toBe("bad");
    expect(reviewTone({ ...base, reviewCount: 0, reviewPct: null })).toBe("none");
  });
});
