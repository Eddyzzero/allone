// Accès au back-end Rust. Dans un simple navigateur (sans Tauri), on
// bascule sur une bibliothèque de démonstration pour pouvoir travailler
// l'interface sans compiler le Rust.

import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { demoLibrary } from "./demo";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type {
  FoundGame,
  Game,
  GameEvent,
  LaunchPlan,
  Library,
  LbGame,
  LbPlatform,
  LogoCandidate,
  MetadataProgress,
  MetadataStatus,
} from "./types";

export const inTauri = isTauri();

// ---- Mode démo (navigateur) -------------------------------------------------

const DEMO_KEY = "allone.demoLibrary";
let demo: Library | null = null;
const demoListeners: Record<string, ((e: GameEvent) => void)[]> = {};

function demoLib(): Library {
  if (!demo) {
    try {
      const raw = localStorage.getItem(DEMO_KEY);
      demo = raw ? (JSON.parse(raw) as Library) : demoLibrary();
    } catch {
      demo = demoLibrary();
    }
  }
  return demo;
}

function demoSave(lib: Library) {
  demo = lib;
  try {
    localStorage.setItem(DEMO_KEY, JSON.stringify(lib));
  } catch {
    /* stockage indisponible : on reste en mémoire */
  }
}

function demoPlan(lib: Library, game: Game): LaunchPlan {
  const dir = (p: string) => p.slice(0, Math.max(p.lastIndexOf("\\"), p.lastIndexOf("/")));
  if (game.kind === "store") throw new Error("Lancement Steam pas encore disponible");
  if (game.kind === "native") {
    return { program: game.path, args: game.args, workingDir: game.workingDir ?? dir(game.path), emulator: null };
  }
  const sys = lib.systems.find((s) => s.id === game.systemId);
  const emu = lib.emulators.find((e) => e.id === (game.emulatorOverride ?? sys?.emulatorId));
  if (!emu) throw new Error(`Aucun émulateur n'est associé à « ${game.systemId} ».`);
  const args = emu.args.map((a) => a.replaceAll("{rom}", game.path));
  if (!emu.args.some((a) => a.includes("{rom}"))) args.push(game.path);
  return { program: emu.exePath, args, workingDir: emu.workingDir ?? dir(emu.exePath), emulator: emu.name };
}

function demoEmit(name: string, e: GameEvent) {
  (demoListeners[name] ?? []).forEach((cb) => cb(e));
}

// ---- API publique -----------------------------------------------------------

export async function getLibrary(): Promise<Library> {
  if (inTauri) return invoke<Library>("get_library");
  return structuredClone(demoLib());
}

/** Enregistre la bibliothèque ; renvoie la nouvelle révision (erreur `STALE` si périmée). */
export async function saveLibrary(library: Library): Promise<number> {
  if (inTauri) return invoke<number>("save_library", { library });
  const saved = structuredClone(library);
  saved.revision += 1;
  demoSave(saved);
  return saved.revision;
}

export async function checkFiles(): Promise<Record<string, boolean>> {
  if (inTauri) return invoke("check_files");
  // En démo, un jeu passe pour « introuvable » afin de montrer cet état.
  return Object.fromEntries(demoLib().games.map((g) => [g.id, g.id !== "seed-9"]));
}

export async function previewLaunch(gameId: string): Promise<LaunchPlan> {
  if (inTauri) return invoke("preview_launch", { gameId });
  const lib = demoLib();
  const game = lib.games.find((g) => g.id === gameId);
  if (!game) throw new Error("Jeu inconnu");
  return demoPlan(lib, game);
}

export async function launchGame(gameId: string): Promise<LaunchPlan> {
  if (inTauri) return invoke("launch_game", { gameId });
  const plan = await previewLaunch(gameId);
  const lib = demoLib();
  const game = lib.games.find((g) => g.id === gameId)!;
  game.lastPlayed = Math.floor(Date.now() / 1000);
  demoSave(lib);
  demoEmit("game-started", { gameId, sessionSecs: 0 });
  // Simule une session de jeu de quelques secondes.
  setTimeout(() => {
    const l = demoLib();
    const g = l.games.find((x) => x.id === gameId);
    if (g) g.playtimeSecs += 4;
    demoSave(l);
    demoEmit("game-exited", { gameId, sessionSecs: 4 });
  }, 4000);
  return plan;
}

export async function scanSystem(systemId: string): Promise<FoundGame[]> {
  if (inTauri) return invoke("scan_system", { systemId });
  return [];
}

export async function addGame(args: {
  path: string;
  systemId: string;
  title?: string;
  native?: boolean;
}): Promise<Game> {
  if (inTauri) return invoke("add_game", args);
  const lib = demoLib();
  const file = args.path.split(/[\\/]/).pop() ?? args.path;
  const g: Game = {
    id: `g-${Date.now().toString(16)}`,
    title: args.title?.trim() || file.replace(/\.[^.]+$/, "").replace(/\s*[([].*?[)\]]/g, "").trim(),
    systemId: args.systemId,
    kind: args.native ?? /\.(exe|appimage|sh)$/i.test(file) ? "native" : "emulated",
    path: args.path,
    args: [],
    favorite: false,
    playtimeSecs: 0,
    addedAt: Math.floor(Date.now() / 1000),
  };
  lib.games.push(g);
  demoSave(lib);
  return structuredClone(g);
}

export async function onGameEvent(
  name: "game-started" | "game-exited",
  cb: (e: GameEvent) => void,
): Promise<UnlistenFn> {
  if (inTauri) return listen<GameEvent>(name, (ev) => cb(ev.payload));
  (demoListeners[name] ??= []).push(cb);
  return () => {
    demoListeners[name] = (demoListeners[name] ?? []).filter((x) => x !== cb);
  };
}

/** Prévient quand la bibliothèque a été modifiée côté Rust (tâches de fond). */
export async function onLibraryChanged(cb: () => void): Promise<UnlistenFn> {
  if (inTauri) return listen("library-changed", () => cb());
  return () => {};
}

/** Avancement des tâches de fond (Steam, base LaunchBox, images). */
export async function onMetadataProgress(cb: (p: MetadataProgress) => void): Promise<UnlistenFn> {
  if (inTauri) return listen<MetadataProgress>("metadata-progress", (ev) => cb(ev.payload));
  return () => {};
}

/** Erreur d'une tâche de fond (message déjà en français). */
export async function onMetadataError(cb: (message: string) => void): Promise<UnlistenFn> {
  if (inTauri) return listen<string>("metadata-error", (ev) => cb(ev.payload));
  return () => {};
}

/** Prévient quand la fenêtre reprend le focus (Tauri uniquement). */
export async function onWindowFocus(cb: () => void): Promise<UnlistenFn> {
  if (!inTauri) return () => {};
  return getCurrentWindow().onFocusChanged(({ payload: focused }) => {
    if (focused) cb();
  });
}

/** Importe les jeux Steam installés (tâche de fond). */
export async function syncSteam(): Promise<void> {
  if (inTauri) await invoke("sync_steam");
}

/** Récupère images et infos (tâche de fond) ; sans `gameIds`, toute la bibliothèque. */
export async function refreshMetadata(gameIds?: string[], force = false): Promise<void> {
  if (inTauri) await invoke("refresh_metadata", { gameIds: gameIds ?? null, force });
}

export async function setLaunchboxMatch(gameId: string, lbId: number): Promise<void> {
  if (inTauri) await invoke("set_launchbox_match", { gameId, lbId });
}

export async function updateLaunchboxDb(): Promise<void> {
  if (inTauri) await invoke("update_launchbox_db");
}

export async function searchLaunchbox(query: string, platform?: string | null): Promise<LbGame[]> {
  if (inTauri) return invoke<LbGame[]>("search_launchbox", { query, platform: platform ?? null });
  return [];
}

export async function metadataStatus(): Promise<MetadataStatus> {
  if (inTauri) return invoke<MetadataStatus>("metadata_status");
  return { steamRoot: null, steamGames: 0, launchboxDate: null, busy: false };
}

/** Choisit un fichier sur le disque (boîte de dialogue native). */
export async function pickFile(opts: { title: string; extensions?: string[] }): Promise<string | null> {
  if (!inTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const filters = opts.extensions?.length ? [{ name: "Jeux", extensions: opts.extensions }] : undefined;
  const result = await open({ title: opts.title, multiple: false, directory: false, filters });
  return typeof result === "string" ? result : null;
}

/** Choisit un dossier sur le disque. */
export async function pickFolder(title: string): Promise<string | null> {
  if (!inTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({ title, multiple: false, directory: true });
  return typeof result === "string" ? result : null;
}

/** Choisit une image (logo) sur le disque. */
export async function pickImage(title: string): Promise<string | null> {
  return pickFile({ title, extensions: ["svg", "png", "jpg", "jpeg", "webp"] });
}

/** Plateformes du catalogue LaunchBox (vide sans base ou en démo). */
export async function listLbPlatforms(): Promise<LbPlatform[]> {
  if (inTauri) return invoke<LbPlatform[]>("list_lb_platforms");
  return [];
}

/** Cherche des logos de console en ligne (vide en démo). */
export async function searchLogos(query: string): Promise<LogoCandidate[]> {
  if (inTauri) return invoke<LogoCandidate[]>("search_logos", { query });
  return [];
}

/** Enregistre le logo d'un système et renvoie son chemin (à écrire dans `system.logo`). */
export async function saveLogo(systemId: string, source: string): Promise<string | null> {
  if (inTauri) return invoke<string>("save_logo", { systemId, source });
  return null;
}

/** Supprime le fichier logo d'une console (sans effet en démo). */
export async function deleteLogo(systemId: string): Promise<void> {
  if (inTauri) await invoke("delete_logo", { systemId });
}

/** URL affichable pour une image : fichier local (via Tauri) ou chemin web. */
export function imageSrc(path?: string | null, version?: string | number): string | null {
  if (!path) return null;
  if (path.startsWith("/demo/") || path.startsWith("/systems/") || /^(https?:|data:|blob:)/.test(path)) return path;
  if (!inTauri) return null;
  const url = convertFileSrc(path);
  // Les images gardent le même chemin quand elles sont remplacées : on force le rechargement.
  return version === undefined ? url : `${url}${url.includes("?") ? "&" : "?"}v=${version}`;
}
