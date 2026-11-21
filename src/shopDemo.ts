// Boutique de démonstration pour le mode navigateur (npm run dev), où l'appel
// direct à la boutique Steam est bloqué. Jeux inventés, sans images.

import type { ShopItem, ShopShelf } from "./types";

const DAY = 86400;
const now = Math.floor(Date.now() / 1000);

function item(appId: number, name: string, extra: Partial<ShopItem>): ShopItem {
  return {
    appId,
    name,
    description: "Jeu de démonstration : dans l'application, la description vient de la boutique Steam.",
    developers: ["Studio de démo"],
    releaseDate: now - 200 * DAY,
    comingSoon: false,
    isFree: false,
    price: "19,99€",
    originalPrice: null,
    discountPct: 0,
    discountEnds: null,
    reviewPct: 88,
    reviewCount: 4210,
    reviewLabel: "très positives",
    capsule: null,
    thumb: null,
    screenshots: [],
    french: "Textes",
    controller: "full",
    deck: 3,
    ...extra,
  };
}

const promo = (pct: number, price: string, was: string, days: number): Partial<ShopItem> => ({
  discountPct: pct,
  price,
  originalPrice: was,
  discountEnds: now + days * DAY,
});

const hollow = item(9001, "Hollow Circuit", { ...promo(50, "12,49€", "24,99€", 6), reviewPct: 91 });
const ashen = item(9002, "Ashen Meridian", { ...promo(40, "35,99€", "59,99€", 6), french: "Voix et textes" });
const tide = item(9003, "Tidebound", { price: "19,99€", reviewPct: 96, reviewLabel: "extrêmement positives" });
const harvest = item(9004, "Neon Harvest", { ...promo(35, "9,74€", "14,99€", 9), releaseDate: now - 5 * DAY, reviewPct: 79, reviewLabel: "plutôt positives", deck: 2 });
const salt = item(9005, "Saltwind Tactics", { price: "29,99€", releaseDate: now - 3 * DAY, controller: "partial" });
const orbital = item(9006, "Orbital Drift", { ...promo(50, "4,99€", "9,99€", 0.5), reviewPct: 74, reviewLabel: "plutôt positives", french: null });
const paper = item(9007, "Paper Kingdoms", { ...promo(30, "24,49€", "34,99€", 10), controller: null, deck: 1 });
const choir = item(9008, "Iron Choir", { isFree: true, price: null, reviewPct: 61, reviewLabel: "moyennes", reviewCount: 88400 });
const vanta = item(9009, "Vanta Protocol", {
  comingSoon: true,
  releaseDate: null,
  price: null,
  reviewPct: null,
  reviewCount: 0,
  reviewLabel: null,
  deck: 0,
});

export function demoShelves(): ShopShelf[] {
  return [
    { id: "specials", label: "Promotions", items: [hollow, ashen, harvest, orbital, paper] },
    { id: "top_sellers", label: "Meilleures ventes", items: [ashen, tide, choir, hollow, paper] },
    { id: "new_releases", label: "Nouveautés", items: [harvest, salt] },
    { id: "coming_soon", label: "À venir", items: [vanta] },
  ];
}

export function demoSearch(query: string): ShopItem[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  const all = [hollow, ashen, tide, harvest, salt, orbital, paper, choir, vanta];
  return all.filter((i) => i.name.toLowerCase().includes(q));
}
