import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { inTauri } from "./api";
import { useStore } from "./store";

type Update = import("@tauri-apps/plugin-updater").Update;

interface UpdateInfo {
  version: string;
  notes: string;
}

interface UpdaterState {
  available: UpdateInfo | null;
  /** Téléchargée, en attente de la fermeture du jeu en cours. */
  ready: boolean;
  checking: boolean;
  downloading: { done: number; total: number } | null;
  /** Cherche une mise à jour ; lève une erreur si la vérification échoue. */
  check: () => Promise<UpdateInfo | null>;
  install: () => Promise<void>;
}

const Ctx = createContext<UpdaterState | null>(null);

export function UpdaterProvider({ children }: { children: ReactNode }) {
  const { running, notify } = useStore();
  const [available, setAvailable] = useState<UpdateInfo | null>(null);
  const [ready, setReady] = useState(false);
  const [checking, setChecking] = useState(false);
  const [downloading, setDownloading] = useState<UpdaterState["downloading"]>(null);
  const update = useRef<Update | null>(null);
  const busy = useRef(false);
  const downloaded = useRef(false);
  const runningRef = useRef(running);
  runningRef.current = running;

  const check = useCallback(async () => {
    if (!inTauri || busy.current) return null;
    setChecking(true);
    try {
      const { check: checkUpdate } = await import("@tauri-apps/plugin-updater");
      const found = await checkUpdate();
      update.current = found;
      downloaded.current = false;
      setReady(false);
      const info = found ? { version: found.version, notes: found.body ?? "" } : null;
      setAvailable(info);
      return info;
    } finally {
      setChecking(false);
    }
  }, []);

  const install = useCallback(async () => {
    const u = update.current;
    if (!u || busy.current) return;
    // Jamais d'installation pendant qu'un jeu tourne.
    if (runningRef.current.size > 0) return;
    busy.current = true;
    let installed = false;
    try {
      if (!downloaded.current) {
        let done = 0;
        let total = 0;
        setDownloading({ done: 0, total: 0 });
        try {
          await u.download((e) => {
            if (e.event === "Started") total = e.data.contentLength ?? 0;
            else if (e.event === "Progress") done += e.data.chunkLength;
            setDownloading({ done, total });
          });
          downloaded.current = true;
        } finally {
          setDownloading(null);
        }
      }
      // Un jeu a pu être lancé pendant le téléchargement : on garde la mise à jour prête, sans la retélécharger.
      if (runningRef.current.size > 0) {
        setReady(true);
        return;
      }
      setReady(false);
      await u.install();
      installed = true;
    } catch (e) {
      console.warn("Mise à jour impossible :", e);
      notify("Mise à jour impossible : réessaie plus tard.", "error");
    } finally {
      busy.current = false;
    }
    if (!installed) return;
    // Sous Windows l'application se ferme pendant l'installation ; ailleurs on relance.
    try {
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    } catch (e) {
      console.warn("Relance impossible :", e);
      downloaded.current = false;
      update.current = null;
      setAvailable(null);
      notify("Mise à jour installée — redémarre allone", "info");
    }
  }, [notify]);

  // Vérification automatique 5 s après le démarrage (application installée uniquement), en silence.
  useEffect(() => {
    if (!inTauri || !import.meta.env.PROD) return;
    const t = setTimeout(() => {
      check().catch((e) => console.warn("Vérification des mises à jour impossible :", e));
    }, 5000);
    return () => clearTimeout(t);
  }, [check]);

  const value = useMemo(
    () => ({ available, ready, checking, downloading, check, install }),
    [available, ready, checking, downloading, check, install],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useUpdater(): UpdaterState {
  const v = useContext(Ctx);
  if (!v) throw new Error("useUpdater doit être utilisé dans <UpdaterProvider>");
  return v;
}
