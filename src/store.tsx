import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import * as api from "./api";
import type { Game, Library, MetadataProgress } from "./types";

interface Toast {
  id: number;
  text: string;
  tone: "ok" | "error" | "info";
}

interface Store {
  library: Library | null;
  fileOk: Record<string, boolean>;
  running: Set<string>;
  toasts: Toast[];
  progress: MetadataProgress | null;
  /** Version à passer à `imageSrc` pour contourner le cache des images. */
  imageVersion: string;
  /** Force le rechargement des images (logo ou jaquette remplacé sous le même chemin). */
  bumpMedia: () => void;
  launch: (game: Game) => Promise<void>;
  toggleFavorite: (game: Game) => Promise<unknown>;
  /** Résout à `true` si la bibliothèque a bien été enregistrée. */
  update: (mutate: (lib: Library) => void) => Promise<boolean>;
  addGame: (args: Parameters<typeof api.addGame>[0]) => Promise<Game>;
  notify: (text: string, tone?: Toast["tone"]) => void;
  reload: () => Promise<void>;
}

const Ctx = createContext<Store | null>(null);

export function useStore(): Store {
  const s = useContext(Ctx);
  if (!s) throw new Error("useStore hors de LibraryProvider");
  return s;
}

let toastId = 0;

export function LibraryProvider({ children }: { children: ReactNode }) {
  const [library, setLibrary] = useState<Library | null>(null);
  const [fileOk, setFileOk] = useState<Record<string, boolean>>({});
  const [running, setRunning] = useState<Set<string>>(new Set());
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [progress, setProgress] = useState<MetadataProgress | null>(null);
  const [mediaEpoch, setMediaEpoch] = useState(0);
  // Une synchro Steam part déjà au lancement : on compte à partir de maintenant.
  const lastSteamSync = useRef(Date.now());

  const notify = useCallback((text: string, tone: Toast["tone"] = "info") => {
    const id = ++toastId;
    setToasts((t) => [...t, { id, text, tone }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 4500);
  }, []);

  const reload = useCallback(async () => {
    const [lib, files] = await Promise.all([api.getLibrary(), api.checkFiles()]);
    setLibrary(lib);
    setFileOk(files);
  }, []);

  useEffect(() => {
    reload().catch((e) => notify(String(e), "error"));
  }, [reload, notify]);

  // Rechargement de la fenêtre pendant une tâche : on repart d'une progression neutre.
  useEffect(() => {
    api.metadataStatus().then((st) => {
      if (st.busy) setProgress((p) => p ?? { phase: "images", done: 0, total: 0, title: null });
    }, () => {});
  }, []);

  useEffect(() => {
    const offs: Promise<() => void>[] = [
      api.onGameEvent("game-started", (e) => setRunning((r) => new Set(r).add(e.gameId))),
      api.onGameEvent("game-exited", (e) => {
        setRunning((r) => {
          const next = new Set(r);
          next.delete(e.gameId);
          return next;
        });
        if (e.error) notify(e.error, "error");
        reload();
      }),
      api.onLibraryChanged(() => {
        reload();
      }),
      api.onMetadataProgress((p) => {
        setProgress(p);
        if (p.phase === "idle") setMediaEpoch((n) => n + 1);
      }),
      api.onMetadataError((message) => notify(message, "error")),
      // Au retour dans la fenêtre : nouvelle synchro Steam, au plus une fois par minute.
      api.onWindowFocus(() => {
        const now = Date.now();
        if (now - lastSteamSync.current < 60_000) return;
        lastSteamSync.current = now;
        api.syncSteam().catch((e) => console.warn("Synchronisation Steam impossible :", e));
      }),
    ];
    return () => {
      offs.forEach((p) => p.then((off) => off()));
    };
  }, [reload, notify]);

  const update = useCallback(
    async (mutate: (lib: Library) => void) => {
      if (!library) return false;
      let base = library;
      // Si une tâche de fond a modifié la bibliothèque entre-temps (STALE),
      // on recharge, on réapplique la modification sur la version fraîche
      // et on renvoie une seule fois.
      for (let attempt = 0; attempt < 2; attempt++) {
        const next = structuredClone(base);
        mutate(next);
        setLibrary(next);
        try {
          next.revision = await api.saveLibrary(next);
          setLibrary(next);
          setFileOk(await api.checkFiles());
          return true;
        } catch (e) {
          if (attempt === 0 && String(e).includes("STALE")) {
            base = await api.getLibrary();
            continue;
          }
          notify(`Enregistrement impossible : ${e}`, "error");
          reload();
          return false;
        }
      }
      return false;
    },
    [library, notify, reload],
  );

  const toggleFavorite = useCallback(
    (game: Game) =>
      update((lib) => {
        const g = lib.games.find((x) => x.id === game.id);
        if (g) g.favorite = !g.favorite;
      }),
    [update],
  );

  const launch = useCallback(
    async (game: Game) => {
      try {
        const plan = await api.launchGame(game.id);
        notify(
          plan.emulator ? `${game.title} se lance avec ${plan.emulator}…` : `${game.title} se lance…`,
          "ok",
        );
        if (!api.inTauri) notify("Mode navigateur : lancement simulé.", "info");
        reload();
      } catch (e) {
        notify(String(e), "error");
      }
    },
    [notify, reload],
  );

  const addGame = useCallback(
    async (args: Parameters<typeof api.addGame>[0]) => {
      const g = await api.addGame(args);
      await reload();
      api.refreshMetadata([g.id]).catch((e) => notify(String(e), "error"));
      return g;
    },
    [reload, notify],
  );

  const imageVersion = String(mediaEpoch);
  const bumpMedia = useCallback(() => setMediaEpoch((n) => n + 1), []);

  const value = useMemo<Store>(
    () => ({ library, fileOk, running, toasts, progress, imageVersion, bumpMedia, launch, toggleFavorite, update, addGame, notify, reload }),
    [library, fileOk, running, toasts, progress, imageVersion, bumpMedia, launch, toggleFavorite, update, addGame, notify, reload],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}
