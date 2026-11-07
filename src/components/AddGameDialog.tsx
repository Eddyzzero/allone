import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import * as api from "../api";
import { bytes, fileName } from "../format";
import { useStore } from "../store";
import type { FoundGame, Game, LaunchKind } from "../types";
import Cover from "./Cover";
import { Check, Close, File, Folder, Play, Upload } from "./Icons";

type Step = "empty" | "filled" | "scan" | "done";

const NATIVE_RE = /\.(exe|appimage|sh|bat|lnk)$/i;

function cleanTitle(file: string) {
  return file
    .replace(/\.[^.]+$/, "")
    .replace(/[([][^)\]]*[)\]]/g, "")
    .replace(/_/g, " ")
    .replace(/\s+/g, " ")
    .replace(/^[-.\s]+|[-.\s]+$/g, "");
}

export default function AddGameDialog({ onClose }: { onClose: () => void }) {
  const { library, addGame, update, notify } = useStore();
  const navigate = useNavigate();
  const systems = (library?.systems ?? []).filter((s) => s.id !== "steam");
  const [step, setStep] = useState<Step>("empty");
  const [path, setPath] = useState("");
  const [manualPath, setManualPath] = useState("");
  const [title, setTitle] = useState("");
  const [systemId, setSystemId] = useState(systems[0]?.id ?? "");
  const [kind, setKind] = useState<LaunchKind>("emulated");
  const [detected, setDetected] = useState(false);
  const [fav, setFav] = useState(false);
  const [busy, setBusy] = useState(false);
  const [added, setAdded] = useState<Game[]>([]);
  const [found, setFound] = useState<FoundGame[]>([]);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const dialogRef = useRef<HTMLDivElement>(null);

  const allExts = useMemo(() => [...new Set(systems.flatMap((s) => s.extensions))], [systems]);

  // Échap pour fermer, focus dans la fenêtre à l'ouverture.
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    dialogRef.current?.focus();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && closeRef.current();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Glisser-déposer un fichier sur la fenêtre (appli Tauri uniquement).
  useEffect(() => {
    if (!api.inTauri) return;
    let off: (() => void) | undefined;
    import("@tauri-apps/api/webview").then(({ getCurrentWebview }) =>
      getCurrentWebview()
        .onDragDropEvent((ev) => {
          if (ev.payload.type === "drop" && ev.payload.paths[0]) choosePath(ev.payload.paths[0]);
        })
        .then((u) => (off = u)),
    );
    return () => off?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function choosePath(p: string) {
    const file = fileName(p);
    const ext = file.split(".").pop()?.toLowerCase() ?? "";
    const native = NATIVE_RE.test(file);
    const match = native ? undefined : systems.find((s) => s.extensions.includes(ext));
    setPath(p);
    setTitle(cleanTitle(native ? fileName(p.replace(/[\\/][^\\/]+$/, "")) || file : file) || file);
    setKind(native ? "native" : "emulated");
    if (match) setSystemId(match.id);
    setDetected(!!match);
    setStep("filled");
  }

  async function browse() {
    const p = await api.pickFile({ title: "Choisir un jeu ou un exécutable", extensions: [...allExts, "exe", "AppImage", "sh"] });
    if (p) choosePath(p);
  }

  async function submit() {
    if (!path || !systemId) return;
    setBusy(true);
    try {
      const g = await addGame({ path, systemId, title, native: kind === "native" });
      if (fav) await update((lib) => { const x = lib.games.find((y) => y.id === g.id); if (x) x.favorite = true; });
      setAdded([g]);
      setStep("done");
    } catch (e) {
      notify(String(e), "error");
    } finally {
      setBusy(false);
    }
  }

  async function scan(id: string) {
    setSystemId(id);
    setStep("scan");
    setBusy(true);
    try {
      const res = await api.scanSystem(id);
      setFound(res);
      setPicked(new Set(res.map((f) => f.path)));
    } catch (e) {
      notify(String(e), "error");
    } finally {
      setBusy(false);
    }
  }

  async function addFound() {
    setBusy(true);
    const list: Game[] = [];
    try {
      for (const f of found.filter((x) => picked.has(x.path))) {
        list.push(await addGame({ path: f.path, systemId: f.systemId, title: f.title, native: false }));
      }
      setAdded(list);
      setStep("done");
    } catch (e) {
      notify(String(e), "error");
    } finally {
      setBusy(false);
    }
  }

  const system = systems.find((s) => s.id === systemId);
  const preview: Game = {
    id: path || "preview", title: title || "Sans titre", systemId, kind, path, args: [], favorite: fav, playtimeSecs: 0, addedAt: 0,
  };

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="dialog" role="dialog" aria-modal="true" aria-labelledby="add-title" tabIndex={-1} ref={dialogRef}>
        <header>
          <h2 id="add-title">{step === "done" ? (added.length > 1 ? "Jeux ajoutés" : "Jeu ajouté") : step === "scan" ? "Jeux trouvés" : "Ajouter un jeu"}</h2>
          <button type="button" className="x" aria-label="Fermer" onClick={onClose}><Close /></button>
        </header>

        {step === "empty" && (
          <div className="content">
            {api.inTauri ? (
              <button type="button" className="drop" onClick={browse}>
                <span className="ico"><Upload /></span>
                <span className="big">Glisse ton jeu ou ton exécutable ici</span>
                <span className="small">ou <b>parcours ton ordinateur</b></span>
                <span className="exts">{allExts.map((e) => `.${e}`).join(" ")} · .exe pour un port PC</span>
              </button>
            ) : (
              <form className="drop" style={{ cursor: "default" }} onSubmit={(e) => { e.preventDefault(); if (manualPath.trim()) choosePath(manualPath.trim()); }}>
                <span className="ico"><Upload /></span>
                <span className="big">Chemin du jeu ou de l'exécutable</span>
                <span className="small">Dans l'appli, une fenêtre de sélection s'ouvre à la place.</span>
                <span style={{ display: "flex", gap: 8, width: "100%", marginTop: 8 }}>
                  <input className="input" placeholder="D:\Jeux\God of War III\Play SpartanPort.exe" value={manualPath} onChange={(e) => setManualPath(e.target.value)} />
                  <button type="submit" className="btn btn-acc" style={{ height: 44 }}>Suivant</button>
                </span>
              </form>
            )}
            <div className="sep">plusieurs jeux d'un coup ?</div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", justifyContent: "center" }}>
              {systems.map((s) => (
                <button key={s.id} type="button" className="btn btn-ghost" onClick={() => scan(s.id)}>
                  <Folder size={16} />Analyser {s.shortName}
                </button>
              ))}
            </div>
          </div>
        )}

        {step === "filled" && (
          <>
            <div className="content">
              <div className="file-row">
                <span className="ico"><File /></span>
                <span className="meta">
                  <span className="n">{fileName(path)}</span>
                  <span className="s" title={path}>{path}</span>
                </span>
                <button type="button" className="btn btn-ghost" onClick={() => setStep("empty")}>Changer</button>
              </div>

              <div className="form-grid">
                <Cover game={preview} />
                <div className="fields">
                  <div>
                    <label className="label" htmlFor="g-title">Titre</label>
                    <input id="g-title" className="input" value={title} onChange={(e) => setTitle(e.target.value)} />
                  </div>
                  <div>
                    <label className="label" htmlFor="g-system">
                      Système {detected && <span className="tag tag-acc">Détecté</span>}
                    </label>
                    <select id="g-system" className="input" value={systemId} onChange={(e) => setSystemId(e.target.value)}>
                      {systems.map((s) => <option key={s.id} value={s.id}>{s.name} ({s.shortName})</option>)}
                    </select>
                  </div>
                </div>
              </div>

              <div>
                <span className="label">Comment le lancer</span>
                <div className="radio-cards" role="radiogroup">
                  <label>
                    <input type="radio" name="kind" checked={kind === "emulated"} onChange={() => setKind("emulated")} />
                    <span className="t">Avec l'émulateur</span>
                    <span className="s">{library?.emulators.find((e) => e.id === system?.emulatorId)?.name ?? "Aucun émulateur"} ouvre ce fichier</span>
                  </label>
                  <label>
                    <input type="radio" name="kind" checked={kind === "native"} onChange={() => setKind("native")} />
                    <span className="t">Exécutable natif</span>
                    <span className="s">Port PC lancé directement</span>
                  </label>
                </div>
              </div>

              <label className="check">
                <input type="checkbox" checked={fav} onChange={(e) => setFav(e.target.checked)} />
                Ajouter aux favoris
              </label>
            </div>
            <footer>
              <span className="note">Jaquette et infos modifiables plus tard</span>
              <button type="button" className="btn btn-ghost" onClick={onClose}>Annuler</button>
              <button type="button" className="btn btn-acc" style={{ height: 40 }} disabled={busy || !title.trim()} onClick={submit}>Ajouter le jeu</button>
            </footer>
          </>
        )}

        {step === "scan" && (
          <>
            <div className="content">
              <p style={{ margin: 0, fontSize: 14, color: "var(--muted)" }}>
                {busy ? "Analyse en cours…" : `${found.length} nouveau(x) jeu(x) dans ${system?.romDirs.join(", ") || "aucun dossier configuré"}`}
              </p>
              {!busy && found.length === 0 && (
                <div className="empty" style={{ padding: 28 }}>
                  {api.inTauri ? "Rien de nouveau : tous les jeux de ce dossier sont déjà dans la bibliothèque." : "L'analyse des dossiers fonctionne dans l'appli (pas dans le navigateur)."}
                </div>
              )}
              <div className="found">
                {found.map((f) => (
                  <label key={f.path}>
                    <input type="checkbox" className="check" style={{ width: 18, height: 18, accentColor: "var(--acc)" }} checked={picked.has(f.path)}
                      onChange={(e) => setPicked((s) => { const n = new Set(s); if (e.target.checked) n.add(f.path); else n.delete(f.path); return n; })} />
                    <span className="t">{f.title}<br /><span className="s">{fileName(f.path)}</span></span>
                    <span className="s">{bytes(f.sizeBytes)}</span>
                  </label>
                ))}
              </div>
            </div>
            <footer>
              <button type="button" className="btn btn-ghost" style={{ marginRight: "auto" }} onClick={() => setStep("empty")}>Retour</button>
              <button type="button" className="btn btn-acc" style={{ height: 40 }} disabled={busy || picked.size === 0} onClick={addFound}>
                Ajouter {picked.size} jeu{picked.size > 1 ? "x" : ""}
              </button>
            </footer>
          </>
        )}

        {step === "done" && (
          <div className="content">
            <div className="done">
              <span className="ico"><Check size={28} /></span>
              <p className="big">{added.length > 1 ? `${added.length} jeux sont prêts` : `${added[0]?.title} est prêt`}</p>
              <p className="small">Ajouté à la bibliothèque · {system?.shortName}{added[0]?.kind === "native" ? " · port PC" : ""}</p>
              <div style={{ display: "flex", gap: 10, marginTop: 14 }}>
                <button type="button" className="btn btn-ghost" onClick={() => { setPath(""); setManualPath(""); setStep("empty"); }}>Ajouter un autre jeu</button>
                {added.length === 1 && (
                  <button type="button" className="btn btn-acc" style={{ height: 40 }} onClick={() => { onClose(); navigate(`/game/${added[0].id}`); }}>
                    <Play size={14} />Voir la fiche
                  </button>
                )}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
