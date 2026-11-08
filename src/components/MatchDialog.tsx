import PadHint from "./PadHint";
import { useEffect, useRef, useState } from "react";
import * as api from "../api";
import { useStore } from "../store";
import type { Game, LbGame } from "../types";
import { Close } from "./Icons";

const MAX_RESULTS = 20;

/** Choix manuel du bon jeu dans la base LaunchBox. */
export default function MatchDialog({ game, onClose }: { game: Game; onClose: () => void }) {
  const { notify, library } = useStore();
  const platform = library?.systems.find((s) => s.id === game.systemId)?.lbPlatform ?? null;
  const [query, setQuery] = useState(game.title);
  const [allPlatforms, setAllPlatforms] = useState(platform === null);
  const [results, setResults] = useState<LbGame[]>([]);
  const [loading, setLoading] = useState(true);
  const [ready, setReady] = useState(true);
  const [busy, setBusy] = useState(false);
  const dialogRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Échap pour fermer, focus dans la fenêtre à l'ouverture.
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    inputRef.current?.focus();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && closeRef.current();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // La base est-elle prête ? Vérifié une seule fois à l'ouverture.
  useEffect(() => {
    let cancelled = false;
    api.metadataStatus().then((st) => !cancelled && setReady(st.launchboxDate !== null), () => !cancelled && setReady(false));
    return () => { cancelled = true; };
  }, []);

  // Recherche avec un délai de 250 ms après la dernière frappe.
  useEffect(() => {
    if (!ready || !query.trim()) {
      setResults([]);
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    const timer = setTimeout(async () => {
      try {
        const found = await api.searchLaunchbox(query, allPlatforms ? null : platform);
        if (!cancelled) setResults(found.slice(0, MAX_RESULTS));
      } catch (e) {
        if (!cancelled) { setResults([]); notify(String(e), "error"); }
      } finally {
        if (!cancelled) setLoading(false);
      }
    }, 250);
    return () => { cancelled = true; clearTimeout(timer); };
  }, [query, allPlatforms, platform, ready, notify]);

  async function choose(lb: LbGame) {
    if (busy) return;
    setBusy(true);
    try {
      await api.setLaunchboxMatch(game.id, lb.id);
      notify("Mise à jour lancée");
      onClose();
    } catch (e) {
      notify(String(e), "error");
      setBusy(false);
    }
  }

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="dialog" role="dialog" aria-modal="true" aria-labelledby="match-title" tabIndex={-1} ref={dialogRef}>
        <header>
          <h2 id="match-title">Choisir le bon jeu</h2>
          <PadHint action="back" /><button type="button" className="x" aria-label="Fermer" onClick={onClose}><Close /></button>
        </header>
        <div className="content">
          <div>
            <label className="label" htmlFor="match-q">Rechercher dans LaunchBox</label>
            <input id="match-q" ref={inputRef} className="input" value={query} onChange={(e) => setQuery(e.target.value)} />
          </div>
          <label className="check">
            <input type="checkbox" checked={allPlatforms} disabled={platform === null} onChange={(e) => setAllPlatforms(e.target.checked)} />
            Toutes mes plateformes
          </label>
          <div className="match-list" aria-live="polite">
            {!ready ? (
              <div className="empty" style={{ padding: 28 }}>La base LaunchBox n'est pas encore prête.</div>
            ) : !loading && query.trim() && results.length === 0 ? (
              <div className="empty" style={{ padding: 28 }}>Aucun jeu trouvé.</div>
            ) : (
              results.map((r) => (
                <button key={r.id} type="button" className="match-item" disabled={busy} onClick={() => choose(r)}>
                  <span className="thumb">
                    {r.cover && <img src={`https://images.launchbox-app.com/${r.cover}`} alt="" loading="lazy" />}
                  </span>
                  <span className="meta">
                    <span className="n">{r.name}</span>
                    <span className="s">{[r.year, r.platform].filter(Boolean).join(" · ")}</span>
                  </span>
                </button>
              ))
            )}
          </div>
        </div>
        <footer>
          <button type="button" className="btn btn-ghost" style={{ marginLeft: "auto" }} onClick={onClose}>Annuler</button>
        </footer>
      </div>
    </div>
  );
}
