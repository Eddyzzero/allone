// Mise en forme des données de la boutique Steam pour l'affichage.

import type { ShopItem } from "./types";

/** « 29,99€ » → « 29,99 € » (espace insécable). */
export function priceText(price: string): string {
  return price.replace(/\s*€/, " €");
}

/** Prix affiché d'un jeu : gratuit, prix, ou rien si le jeu n'est pas encore en vente. */
export function priceOf(item: ShopItem): string | null {
  if (item.isFree) return "Gratuit";
  return item.price ? priceText(item.price) : null;
}

/** « très positives » → « Très positives ». */
export function reviewText(item: ShopItem): string {
  if (!item.reviewLabel || item.reviewCount === 0) return "Pas encore d'évaluations";
  return item.reviewLabel.charAt(0).toUpperCase() + item.reviewLabel.slice(1);
}

const MONTH = new Intl.DateTimeFormat("fr-BE", { month: "long" });

/** Date en français avec « 1er » pour le premier du mois (« 1er décembre 2026 »). */
export function frDate(unixSecs: number, withYear = true): string {
  const d = new Date(unixSecs * 1000);
  const day = d.getDate() === 1 ? "1er" : String(d.getDate());
  return `${day} ${MONTH.format(d)}${withYear ? ` ${d.getFullYear()}` : ""}`;
}

/** Fin d'une promotion, ou `null` s'il n'y en a pas (ou plus). */
export function offerEnds(item: ShopItem, nowSecs = Date.now() / 1000): string | null {
  if (!item.discountPct || !item.discountEnds) return null;
  const left = item.discountEnds - nowSecs;
  if (left <= 0) return null;
  if (left < 3600) return "Se termine dans moins d'une heure";
  if (left < 86400) return `Se termine dans ${Math.floor(left / 3600)} h`;
  return `Offre valable jusqu'au ${frDate(item.discountEnds, false)}`;
}

/** Date de sortie lisible. */
export function releaseText(item: ShopItem): string {
  if (!item.releaseDate) return item.comingSoon ? "Date à annoncer" : "Inconnue";
  return frDate(item.releaseDate);
}

/** Vignette réduite d'une capture Steam (600×338) ; Steam la sert à côté de l'originale. */
export function screenshotThumb(url: string): string {
  return url.replace(/\.jpg(\?|$)/, ".600x338.jpg$1");
}

export function deckText(deck: number): string | null {
  return deck === 3 ? "Vérifié" : deck === 2 ? "Jouable" : deck === 1 ? "Non pris en charge" : null;
}

export function controllerText(item: ShopItem): string {
  return item.controller === "full" ? "Prise en charge complète" : item.controller === "partial" ? "Partielle" : "Non indiquée";
}

/** Couleur de la barre d'évaluations. */
export function reviewTone(item: ShopItem): "good" | "mixed" | "bad" | "none" {
  if (item.reviewPct == null || item.reviewCount === 0) return "none";
  if (item.reviewPct >= 80) return "good";
  if (item.reviewPct >= 50) return "mixed";
  return "bad";
}
