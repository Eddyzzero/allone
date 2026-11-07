// Thèmes de couleurs : chaque thème fixe les variables CSS de l'appli et les
// couleurs du dégradé animé en fond. Le choix et les thèmes perso sont gardés
// dans le navigateur de l'appli (localStorage).

import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

export interface Theme {
  id: string;
  name: string;
  /** Thème fourni avec l'appli (non supprimable). */
  builtin?: boolean;
  ground: string;
  acc: string;
  /** Texte posé sur l'accent (boutons principaux). */
  onAcc: string;
  /** Second accent : étiquettes, barres, éléments actifs secondaires. */
  acc2: string;
  text: string;
  muted: string;
  /** Couleurs du dégradé animé (2 à 6). */
  gradient: string[];
}

export const BUILTIN_THEMES: Theme[] = [
  {
    id: "vert",
    name: "Vert",
    builtin: true,
    ground: "#0e1110",
    acc: "#b4f25c",
    onAcc: "#04110a",
    acc2: "#7be0a8",
    text: "#eef3ef",
    muted: "#a3ada6",
    gradient: ["#0b1410", "#16352a", "#3f6b2f", "#b4f25c"],
  },
  {
    id: "crimson",
    name: "Crimson",
    builtin: true,
    ground: "#140707",
    acc: "#feed65",
    onAcc: "#2a0a07",
    acc2: "#e13a26",
    text: "#fff3ee",
    muted: "#d6b3ad",
    gradient: ["#2b0705", "#831613", "#e13a26", "#feed65"],
  },
];

export interface Appearance {
  themeId: string;
  custom: Theme[];
  /** Thèmes d'origine retirés par l'utilisateur (restaurables). */
  hidden: string[];
  /** Dégradé animé en fond (sinon dégradé figé). */
  animate: boolean;
}

const KEY = "allone.appearance";
const DEFAULT: Appearance = { themeId: "vert", custom: [], hidden: [], animate: true };

function load(): Appearance {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return DEFAULT;
    const a = JSON.parse(raw) as Partial<Appearance>;
    return {
      themeId: typeof a.themeId === "string" ? a.themeId : DEFAULT.themeId,
      custom: Array.isArray(a.custom) ? a.custom.filter(isTheme) : [],
      hidden: Array.isArray(a.hidden) ? a.hidden.filter((x): x is string => typeof x === "string") : [],
      animate: typeof a.animate === "boolean" ? a.animate : true,
    };
  } catch {
    return DEFAULT;
  }
}

function isTheme(t: unknown): t is Theme {
  const x = t as Theme;
  return !!x && typeof x.id === "string" && typeof x.name === "string" && Array.isArray(x.gradient)
    && [x.ground, x.acc, x.onAcc, x.acc2, x.text, x.muted].every((c) => typeof c === "string");
}

/** "#rrggbb" → [r, g, b] ; couleur invalide → noir. */
function rgb(hex: string): [number, number, number] {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return [0, 0, 0];
  const n = parseInt(m[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** Mélange deux couleurs (t = part de b). */
function mix(a: string, b: string, t: number): string {
  const [r1, g1, b1] = rgb(a);
  const [r2, g2, b2] = rgb(b);
  const c = (x: number, y: number) => Math.round(x + (y - x) * t).toString(16).padStart(2, "0");
  return `#${c(r1, r2)}${c(g1, g2)}${c(b1, b2)}`;
}

/** Variables CSS dérivées d'un thème (les autres nuances en sont tirées). */
export function themeVars(t: Theme): Record<string, string> {
  const [r, g, b] = rgb(t.ground);
  return {
    "--acc": t.acc,
    "--acc-soft": `${t.acc}1f`,
    "--on-acc": t.onAcc,
    "--acc-2": t.acc2,
    "--ground": t.ground,
    "--ground-rgb": `${r} ${g} ${b}`,
    "--text": t.text,
    "--text-2": mix(t.text, t.muted, 0.4),
    "--muted": t.muted,
    "--faint": mix(t.muted, t.ground, 0.25),
    "--nav": mix(t.text, t.muted, 0.6),
    "--elevated": mix(t.ground, t.text, 0.05),
    "--elevated-2": mix(t.ground, t.text, 0.08),
    "--hero-bg": mix(t.ground, t.text, 0.06),
    "--disabled": mix(t.muted, t.ground, 0.6),
  };
}

interface ThemeStore extends Appearance {
  themes: Theme[];
  theme: Theme;
  setTheme: (id: string) => void;
  setAnimate: (on: boolean) => void;
  /** Ajoute ou remplace un thème perso. */
  saveCustom: (t: Theme) => void;
  /** Supprime un thème perso ou retire un thème d'origine (jamais le dernier). */
  removeTheme: (id: string) => void;
  /** Remet les thèmes d'origine retirés. */
  restoreBuiltins: () => void;
}

function visible(a: Appearance): Theme[] {
  return [...BUILTIN_THEMES.filter((t) => !a.hidden.includes(t.id)), ...a.custom];
}

const Ctx = createContext<ThemeStore | null>(null);

export function useTheme(): ThemeStore {
  const s = useContext(Ctx);
  if (!s) throw new Error("useTheme hors de ThemeProvider");
  return s;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [app, setApp] = useState<Appearance>(load);

  useEffect(() => {
    try {
      localStorage.setItem(KEY, JSON.stringify(app));
    } catch {
      /* stockage indisponible : le choix vaut pour cette session */
    }
  }, [app]);

  const themes = useMemo(() => visible(app), [app]);
  const theme = themes.find((t) => t.id === app.themeId) ?? themes[0] ?? BUILTIN_THEMES[0];

  useEffect(() => {
    const root = document.documentElement;
    for (const [k, v] of Object.entries(themeVars(theme))) root.style.setProperty(k, v);
  }, [theme]);

  const setTheme = useCallback((id: string) => setApp((a) => ({ ...a, themeId: id })), []);
  const setAnimate = useCallback((on: boolean) => setApp((a) => ({ ...a, animate: on })), []);
  const saveCustom = useCallback(
    (t: Theme) =>
      setApp((a) => {
        const custom = a.custom.some((x) => x.id === t.id) ? a.custom.map((x) => (x.id === t.id ? t : x)) : [...a.custom, t];
        return { ...a, custom };
      }),
    [],
  );
  const removeTheme = useCallback(
    (id: string) =>
      setApp((a) => {
        if (visible(a).length <= 1) return a;
        const next: Appearance = BUILTIN_THEMES.some((t) => t.id === id)
          ? { ...a, hidden: [...a.hidden, id] }
          : { ...a, custom: a.custom.filter((x) => x.id !== id) };
        if (next.themeId === id) next.themeId = visible(next)[0].id;
        return next;
      }),
    [],
  );
  const restoreBuiltins = useCallback(() => setApp((a) => ({ ...a, hidden: [] })), []);

  const value = useMemo<ThemeStore>(
    () => ({ ...app, themes, theme, setTheme, setAnimate, saveCustom, removeTheme, restoreBuiltins }),
    [app, themes, theme, setTheme, setAnimate, saveCustom, removeTheme, restoreBuiltins],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}
