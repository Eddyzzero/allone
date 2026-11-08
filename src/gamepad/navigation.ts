// Actions de la manette sur le DOM : focus spatial, validation, retour, rubriques.
import { pickNeighbor, type Dir, type Rect } from "./logic";

const FOCUSABLE = "a[href], button, input, select, textarea, [tabindex]";

const defaultVisible = (el: HTMLElement) => el.getClientRects().length > 0;
const defaultRect = (el: HTMLElement): Rect => {
  const r = el.getBoundingClientRect();
  return { left: r.left, top: r.top, right: r.right, bottom: r.bottom };
};

/** Zone où le focus est confiné : le dernier dialogue modal ouvert, sinon la page. */
export function focusRoot(doc: Document = document): HTMLElement {
  const dialogs = doc.querySelectorAll<HTMLElement>('[role="dialog"][aria-modal="true"]');
  return dialogs.length > 0 ? dialogs[dialogs.length - 1] : doc.body;
}

/** Éléments focusables et visibles d'une zone. */
export function focusables(root: HTMLElement, isVisible = defaultVisible): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
    (el) =>
      !(el as HTMLButtonElement).disabled &&
      el.getAttribute("tabindex") !== "-1" &&
      !el.closest('[aria-hidden="true"]') &&
      isVisible(el),
  );
}

/** Dernier élément déplacé à la manette : sert de repli quand il disparaît du DOM. */
let last: { el: HTMLElement; rect: Rect; root: HTMLElement } | null = null;
/** Élément de la page qui a ouvert le dialogue en cours (validé à la manette), rendu au retour. */
let pageOpener: HTMLElement | null = null;
let pendingRestore: HTMLElement | null = null;

function remember(el: HTMLElement, root: HTMLElement, rectOf: (el: HTMLElement) => Rect): void {
  last = { el, rect: rectOf(el), root };
}

function focusEl(el: HTMLElement, root: HTMLElement, rectOf: (el: HTMLElement) => Rect = defaultRect): void {
  el.focus({ preventScroll: true });
  remember(el, root, rectOf);
  // Absent sous jsdom.
  if (typeof el.scrollIntoView === "function") {
    el.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" });
  }
}

/** Premier focusable d'une zone, en préférant l'élément marqué data-pad-autofocus. */
function firstIn(container: HTMLElement, isVisible?: (el: HTMLElement) => boolean): HTMLElement | undefined {
  const list = focusables(container, isVisible);
  return list.find((el) => el.hasAttribute("data-pad-autofocus")) ?? list[0];
}

/** Choix du focus quand l'élément actif n'est plus parmi les focusables de la zone. */
function fallbackTarget(
  root: HTMLElement,
  list: HTMLElement[],
  isVisible: ((el: HTMLElement) => boolean) | undefined,
  rectOf: (el: HTMLElement) => Rect,
): HTMLElement | undefined {
  if (list.length === 0) return undefined;
  // Retour d'un dialogue : on rend le focus à l'élément qui l'avait ouvert.
  if (pendingRestore && root === document.body) {
    const el = pendingRestore;
    pendingRestore = null;
    if (el.isConnected && list.includes(el)) return el;
  }
  // 1. Le dernier élément déplacé, s'il est toujours là.
  if (last && last.root === root && last.el.isConnected && list.includes(last.el)) return last.el;
  // 2. Le plus proche de sa dernière position.
  if (last && last.root === root) {
    const c = { x: (last.rect.left + last.rect.right) / 2, y: (last.rect.top + last.rect.bottom) / 2 };
    let best: HTMLElement | undefined;
    let bestD = Infinity;
    for (const el of list) {
      const r = rectOf(el);
      const d = Math.hypot((r.left + r.right) / 2 - c.x, (r.top + r.bottom) / 2 - c.y);
      if (d < bestD) {
        bestD = d;
        best = el;
      }
    }
    if (best) return best;
  }
  // 3. Premier élément du contenu (hors en-tête du dialogue, dont la croix fermerait tout).
  if (root !== document.body) {
    const content = root.querySelector<HTMLElement>(".content");
    return (content && firstIn(content, isVisible)) || list[0];
  }
  const main = document.querySelector<HTMLElement>("main");
  return (main && firstIn(main, isVisible)) || list[0];
}

export function moveFocus(
  dir: Dir,
  opts: { isVisible?: (el: HTMLElement) => boolean; rect?: (el: HTMLElement) => Rect; repeat?: boolean } = {},
): void {
  const root = focusRoot();
  const list = focusables(root, opts.isVisible);
  const active = document.activeElement as HTMLElement | null;
  const rectOf = opts.rect ?? defaultRect;

  // Focus hors des focusables (racine, conteneur du dialogue, body, élément disparu) : repli.
  if (!active || !list.includes(active)) {
    const target = fallbackTarget(root, list, opts.isVisible, rectOf);
    if (target) focusEl(target, root, rectOf);
    return;
  }
  remember(active, root, rectOf);

  const horizontal = dir === "left" || dir === "right";

  if (horizontal && active instanceof HTMLSelectElement) {
    // Une direction tenue ne doit pas modifier (et enregistrer) un réglage en boucle.
    if (opts.repeat) return;
    const next = Math.max(
      0,
      Math.min(active.options.length - 1, active.selectedIndex + (dir === "right" ? 1 : -1)),
    );
    if (next !== active.selectedIndex) {
      active.selectedIndex = next;
      active.dispatchEvent(new Event("change", { bubbles: true }));
    }
    return;
  }

  // Seul l'élément marqué lui-même reçoit les flèches (le reste navigue dans l'espace).
  if (horizontal && active.hasAttribute("data-pad-keys")) {
    active.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: dir === "right" ? "ArrowRight" : "ArrowLeft",
        bubbles: true,
        cancelable: true,
      }),
    );
    return;
  }

  const others = list.filter((el) => el !== active);
  const i = pickNeighbor(
    rectOf(active),
    others.map((el) => rectOf(el)),
    dir,
  );
  if (i >= 0) focusEl(others[i], root, rectOf);
}

const CLICKABLE_INPUTS = ["checkbox", "radio", "button", "submit", "reset"];

/** Valide l'élément focalisé (sans cliquer dans un champ de saisie). */
export function activate(): void {
  const el = document.activeElement as HTMLElement | null;
  if (!el || el === document.body) return;
  if (el instanceof HTMLTextAreaElement || el instanceof HTMLSelectElement) return;
  if (el instanceof HTMLInputElement && !CLICKABLE_INPUTS.includes(el.type)) return;
  if (focusRoot() === document.body) pageOpener = el;
  el.click();
}

/** Retour : ferme le dialogue ouvert (Échap), sinon page précédente (sauf à l'accueil). */
export function goBack(navigate: (delta: number) => void, pathname: string): void {
  if (document.querySelector('[role="dialog"][aria-modal="true"]')) {
    const opener = pageOpener;
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    // Si c'était le dernier dialogue, le focus revient à l'élément qui l'avait ouvert.
    if (opener && opener.isConnected) {
      pendingRestore = opener;
      window.setTimeout(() => {
        if (focusRoot() !== document.body || !pendingRestore) return;
        const el = pendingRestore;
        const a = document.activeElement;
        if ((!a || a === document.body || !a.isConnected) && el.isConnected && focusables(document.body).includes(el)) {
          pendingRestore = null;
          focusEl(el, document.body);
        }
      }, 60);
    }
    return;
  }
  if (pathname !== "/") navigate(-1);
}

/** Rubrique suivante / précédente de la barre latérale ; sans boucle. */
export function switchSection(delta: 1 | -1): boolean {
  // Un dialogue ouvert garde la main : on ne change pas de page derrière lui.
  if (focusRoot() !== document.body) return false;
  const links = Array.from(document.querySelectorAll<HTMLElement>("aside.sidebar nav.nav a"));
  const current = links.findIndex((l) => l.classList.contains("active"));
  if (current < 0) return false;
  const target = links[current + delta];
  if (!target) return false;
  target.click();
  return true;
}

/** Donne le focus au premier élément focusable (ou à celui marqué data-pad-autofocus) de la première zone correspondant au sélecteur. */
export function focusFirstIn(selector: string, isVisible = defaultVisible): void {
  const root = document.querySelector<HTMLElement>(selector);
  if (!root) return;
  const first = firstIn(root, isVisible);
  if (first) focusEl(first, focusRoot());
}

/** Oublie le dernier élément mémorisé (tests). */
export function resetFocusMemory(): void {
  last = null;
  pageOpener = null;
  pendingRestore = null;
}
