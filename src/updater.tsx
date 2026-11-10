import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { inTauri } from "./api";
import { useStore } from "./store";
import { CHECK_EVERY_MS, FOCUS_RECHECK_MS, shouldCheck, whatsNewOnStart, type PendingUpdate, type WhatsNew } from "./updateNotes";

type Update = import("@tauri-apps/plugin-updater").Update;

interface UpdateInfo {
  version: string;
  notes: string;
}

interface UpdaterState {
  available: UpdateInfo | null;
  /** Le bandeau a été fermé (« Plus tard ») pour cette version, jusqu'au prochain démarrage. */
  dismissed: boolean;
  dismiss: () => void;
  /** Téléchargée, en attente de la fermeture du jeu en cours. */
  ready: boolean;
  checking: boolean;
  downloading: { done: number; total: number } | null;
  /** Cherche une mise à jour ; lève une erreur si la vérification échoue. */
  check: () => Promise<UpdateInfo | null>;
  install: () => Promise<void>;
  /** Nouveautés de la version qui vient d'être installée (affichées une fois). */
  whatsNew: WhatsNew | null;
  closeWhatsNew: () => void;
}

const LAST_SEEN_KEY = "allone.update.lastSeenVersion";
const PENDING_KEY = "allone.update.pending";

// Le stockage du WebView peut être indisponible : on ne casse jamais l'appli pour ça.
function readStorage(key: string): string | null {
  try { return localStorage.getItem(key); } catch { return null; }
}
function writeStorage(key: string, value: string | null) {
  try { if (value === null) localStorage.removeItem(key); else localStorage.setItem(key, value); } catch { /* ignoré */ }
}
function readPending(): PendingUpdate | null {
  const raw = readStorage(PENDING_KEY);
  if (!raw) return null;
  try {
    const p = JSON.parse(raw);
    return typeof p?.version === "string" && typeof p?.notes === "string" ? p : null;
  } catch { return null; }
}

const Ctx = createContext<UpdaterState | null>(null);

export function UpdaterProvider({ children }: { children: ReactNode }) {
  const { running, notify } = useStore();
  const [available, setAvailable] = useState<UpdateInfo | null>(null);
  const [ready, setReady] = useState(false);
  const [checking, setChecking] = useState(false);
  const [downloading, setDownloading] = useState<UpdaterState["downloading"]>(null);
  const [dismissedVersion, setDismissedVersion] = useState<string | null>(null);
  const [whatsNew, setWhatsNew] = useState<WhatsNew | null>(null);
  const lastCheck = useRef<number | null>(null);
  const update = useRef<Update | null>(null);
  const busy = useRef(false);
  const downloaded = useRef(false);
  const runningRef = useRef(running);
  runningRef.current = running;

  const check = useCallback(async () => {
    if (!inTauri || busy.current) return null;
    setChecking(true);
    lastCheck.current = Date.now();
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
      // Gardé pour l'afficher au redémarrage (« Nouveautés de la version X »).
      writeStorage(PENDING_KEY, JSON.stringify({ version: u.version, notes: u.body ?? "" }));
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

  // Vérifications automatiques (application installée uniquement), en silence :
  // 5 s après le démarrage, puis toutes les 6 h, et au retour sur la fenêtre si la dernière date d'1 h.
  useEffect(() => {
    if (!inTauri || !import.meta.env.PROD) return;
    const silent = () => check().catch((e) => console.warn("Vérification des mises à jour impossible :", e));
    const first = setTimeout(silent, 5000);
    const every = setInterval(silent, CHECK_EVERY_MS);
    const onFocus = () => {
      if (shouldCheck(Date.now(), lastCheck.current, FOCUS_RECHECK_MS)) silent();
    };
    window.addEventListener("focus", onFocus);
    return () => {
      clearTimeout(first);
      clearInterval(every);
      window.removeEventListener("focus", onFocus);
    };
  }, [check]);

  // Au démarrage : la version a-t-elle changé depuis la dernière fois ? Si oui, on montre les nouveautés.
  useEffect(() => {
    if (!inTauri) return;
    let cancelled = false;
    import("@tauri-apps/api/app")
      .then((m) => m.getVersion())
      .then((current) => {
        if (cancelled) return;
        setWhatsNew(whatsNewOnStart(current, readStorage(LAST_SEEN_KEY), readPending()));
        writeStorage(LAST_SEEN_KEY, current);
        writeStorage(PENDING_KEY, null);
      })
      .catch((e) => console.warn("Version indisponible :", e));
    return () => { cancelled = true; };
  }, []);

  const dismiss = useCallback(() => setDismissedVersion(available?.version ?? null), [available]);
  const closeWhatsNew = useCallback(() => setWhatsNew(null), []);
  const dismissed = !!available && dismissedVersion === available.version;

  const value = useMemo(
    () => ({ available, dismissed, dismiss, ready, checking, downloading, check, install, whatsNew, closeWhatsNew }),
    [available, dismissed, dismiss, ready, checking, downloading, check, install, whatsNew, closeWhatsNew],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useUpdater(): UpdaterState {
  const v = useContext(Ctx);
  if (!v) throw new Error("useUpdater doit être utilisé dans <UpdaterProvider>");
  return v;
}
