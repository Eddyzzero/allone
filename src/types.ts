// Miroir des structures Rust (src-tauri/src/library.rs).

export type LaunchKind = "emulated" | "native" | "store";

export interface StoreRef {
  store: string;
  id: string;
}

export interface MetaRef {
  source: "launchbox" | "steam";
  id: string;
  locked: boolean;
}

export interface Emulator {
  id: string;
  name: string;
  exePath: string;
  /** Gabarit d'arguments ; `{rom}` est remplacé par le chemin du jeu. */
  args: string[];
  workingDir?: string | null;
}

export interface System {
  id: string;
  name: string;
  shortName: string;
  color: string;
  year?: number | null;
  emulatorId?: string | null;
  romDirs: string[];
  extensions: string[];
  /** Plateforme LaunchBox (images et infos automatiques) ; `null` = aucune. */
  lbPlatform?: string | null;
  /** Logo : `/systems/<id>.svg` ou chemin local ; `null` = aucun. */
  logo?: string | null;
}

/** Plateforme du catalogue LaunchBox. */
export interface LbPlatform {
  name: string;
  year?: number | null;
  manufacturer?: string | null;
  category?: string | null;
}

/** Logo de console proposé par la recherche Wikimedia Commons. */
export interface LogoCandidate {
  title: string;
  thumbUrl: string;
  url: string;
  width: number;
  height: number;
  size: number;
}

export interface Game {
  id: string;
  title: string;
  systemId: string;
  kind: LaunchKind;
  path: string;
  args: string[];
  workingDir?: string | null;
  emulatorOverride?: string | null;
  cover?: string | null;
  hero?: string | null;
  logo?: string | null;
  store?: StoreRef | null;
  metadata?: MetaRef | null;
  year?: number | null;
  genre?: string | null;
  description?: string | null;
  favorite: boolean;
  playtimeSecs: number;
  /** Part de `playtimeSecs` importée de Playnite. */
  playniteSecs?: number;
  lastPlayed?: number | null;
  addedAt: number;
}

export interface Library {
  version: number;
  /** Compteur de modifications ; une sauvegarde périmée est refusée (STALE). */
  revision: number;
  emulators: Emulator[];
  systems: System[];
  games: Game[];
  /** Jeux retirés à ne plus réimporter : `steam:<id>` ou chemin en minuscules. */
  ignored?: string[];
}

export interface LaunchPlan {
  program: string;
  args: string[];
  workingDir?: string | null;
  emulator?: string | null;
}

export interface FoundGame {
  path: string;
  title: string;
  systemId: string;
  sizeBytes: number;
}

export interface GameEvent {
  gameId: string;
  sessionSecs: number;
  error?: string | null;
}

/** Fiche LaunchBox renvoyée par la recherche (images = noms de fichiers LaunchBox). */
export interface LbGame {
  id: number;
  name: string;
  altNames: string[];
  platform: string;
  year?: number | null;
  overview?: string | null;
  genres: string[];
  developer?: string | null;
  publisher?: string | null;
  cover?: string | null;
  hero?: string | null;
  logo?: string | null;
}

/** Résultat d'un import du temps de jeu Playnite. */
export interface PlayniteImport {
  /** Jeux dont le temps de jeu a changé. */
  updated: number;
  /** Jeux Playnite retrouvés dans allone. */
  matched: number;
  /** Temps ajouté au total (secondes). */
  addedSecs: number;
  /** Jeux Steam ignorés (leur temps vient déjà de Steam). */
  steamSkipped: number;
  /** Jeux Playnite avec du temps de jeu mais absents d'allone. */
  unmatched: string[];
}

/** Compte Steam du PC et présence d'une clé d'API (miroir de `SteamAccountStatus` côté Rust). */
export interface SteamAccountStatus {
  accountName: string | null;
  hasKey: boolean;
}

export interface MetadataStatus {
  steamRoot: string | null;
  steamGames: number;
  /** Date de la base LaunchBox (secondes Unix). */
  launchboxDate: number | null;
  busy: boolean;
}

export type MetadataPhase = "steam" | "launchbox-download" | "launchbox-index" | "images" | "idle";

export interface MetadataProgress {
  /** Pour `launchbox-download`, `done` et `total` sont en octets. */
  done: number;
  total: number;
  title: string | null;
  phase: MetadataPhase;
}

/** Jeu de la boutique Steam (miroir de `stores::steam_shop::ShopItem`). */
export interface ShopItem {
  appId: number;
  name: string;
  description?: string | null;
  developers: string[];
  /** Date de sortie (secondes Unix), absente si Steam n'annonce pas de jour précis. */
  releaseDate?: number | null;
  comingSoon: boolean;
  isFree: boolean;
  /** Prix affiché par Steam (« 29,99€ »), absent si le jeu n'est pas encore en vente. */
  price?: string | null;
  originalPrice?: string | null;
  discountPct: number;
  discountEnds?: number | null;
  reviewPct?: number | null;
  reviewCount: number;
  reviewLabel?: string | null;
  capsule?: string | null;
  thumb?: string | null;
  screenshots: string[];
  french?: string | null;
  controller?: "full" | "partial" | null;
  /** Steam Deck : 3 vérifié, 2 jouable, 1 non pris en charge, 0 inconnu. */
  deck: number;
}

/** Rayon de la boutique Steam. */
export interface ShopShelf {
  id: string;
  label: string;
  items: ShopItem[];
}
