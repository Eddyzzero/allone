import * as api from "./api";
import type { Game } from "./types";

type Notify = (text: string, tone?: "info" | "error") => void;

/** Ouvre la fenêtre d'installation de Steam pour ce jeu et prévient l'utilisateur. */
export async function installSteam(game: Game, notify: Notify): Promise<void> {
  const appId = steamAppId(game);
  if (appId == null) return;
  try {
    await api.installSteamGame(appId);
    notify(`Steam ouvre l'installation de ${game.title}.`);
  } catch (e) {
    notify(String(e), "error");
  }
}

/** Jeu Steam de la bibliothèque qui n'est pas (ou pas encore entièrement) installé. */
export function isNotInstalled(game: Game, fileOk: Record<string, boolean>): boolean {
  return game.kind === "store" && fileOk[game.id] === false;
}

/** Identifiant Steam du jeu, ou `null` si ce n'est pas un jeu Steam. */
export function steamAppId(game: Game): number | null {
  return game.store?.store === "steam" ? Number(game.store.id) : null;
}
