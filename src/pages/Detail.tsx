import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import * as api from "../api";
import Cover from "../components/Cover";
import { ChevronLeft, Heart, Play, Terminal } from "../components/Icons";
import LogoTitle from "../components/LogoTitle";
import PadHint from "../components/PadHint";
import MatchDialog from "../components/MatchDialog";
import { ago, fileName, playtime } from "../format";
import { useStore } from "../store";
import type { LaunchPlan } from "../types";

function quote(s: string) {
  return /\s/.test(s) ? `"${s}"` : s;
}

export default function Detail() {
  const { id } = useParams();
  const navigate = useNavigate();
  const { library, fileOk, running, launch, toggleFavorite, update, notify, progress } = useStore();
  const game = library?.games.find((g) => g.id === id);
  const [plan, setPlan] = useState<LaunchPlan | null>(null);
  const [planError, setPlanError] = useState<string | null>(null);
  const [showCmd, setShowCmd] = useState(false);
  const [matching, setMatching] = useState(false);

  useEffect(() => {
    if (!game) return;
    api.previewLaunch(game.id).then(
      (p) => { setPlan(p); setPlanError(null); },
      (e) => { setPlan(null); setPlanError(String(e)); },
    );
  }, [game?.id, game?.kind, game?.systemId, game?.emulatorOverride, library?.emulators]);

  if (!library) return null;
  if (!game) {
    return <div className="empty">Ce jeu n'est plus dans la bibliothèque. <Link className="link-acc" to="/library">Bibliothèque</Link></div>;
  }

  const system = library.systems.find((s) => s.id === game.systemId);
  const ok = fileOk[game.id] !== false;
  const isRunning = running.has(game.id);
  const similar = library.games.filter((g) => g.systemId === game.systemId && g.id !== game.id).slice(0, 6);
  const emulators = library.emulators;
  const isStore = game.kind === "store";
  const sourceLabel = game.metadata?.source === "launchbox" ? "LaunchBox" : game.metadata?.source === "steam" ? "Steam" : "Aucune — pas trouvé";
  const canMatch = !isStore && (system?.lbPlatform != null || game.metadata?.source === "launchbox");

  return (
    <div className="detail">
      <section className="d-hero" style={{ background: "var(--hero-bg)" }}>
        <Cover game={game} variant="hero" showTitle={false} style={{ position: "absolute", inset: 0 }} />
        <span className="veil" />

        <nav className="d-nav" aria-label="Fil d'Ariane">
          <button type="button" className="back" onClick={() => navigate(-1)}><ChevronLeft />Retour</button>
          <span className="path">
            <Link to={`/system/${game.systemId}`}>{system?.shortName ?? game.systemId}</Link> / {game.title}
          </span>
          <span className="tools">
            <button type="button" className={`round ${game.favorite ? "on" : ""}`} onClick={() => toggleFavorite(game)}
              aria-label={game.favorite ? "Retirer des favoris" : "Ajouter aux favoris"} aria-pressed={game.favorite}>
              <Heart filled={game.favorite} />
            </button>
            <button type="button" className="round" aria-label="Voir la commande de lancement" aria-pressed={showCmd} onClick={() => setShowCmd((v) => !v)}>
              <Terminal />
            </button>
          </span>
        </nav>

        <div className="d-main">
          <Cover game={game} showTitle={false} />
          <div className="d-info">
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <span className="tag-lg tag">{system?.shortName ?? game.systemId}</span>
              {game.kind === "native" ? <span className="tag-lg tag tag-acc">Port PC natif</span> : null}
              {ok ? <span className="tag-lg tag tag-acc">Fichiers trouvés</span> : <span className="tag-lg tag tag-danger">Fichier introuvable</span>}
            </div>
            <LogoTitle title={game.title} logo={game.logo} />
            <p>{[game.year, game.genre, game.kind === "store" ? "Lancé par Steam" : game.kind === "native" ? "Exécutable natif" : plan?.emulator ? `via ${plan.emulator}` : null].filter(Boolean).join(" · ")}</p>
            <div className="row-actions">
              <button type="button" data-pad-autofocus className="btn btn-acc btn-lg" onClick={() => launch(game)} disabled={isRunning || !ok}>
                <Play />{isRunning ? "En cours…" : "Jouer"}{!(isRunning || !ok) && <PadHint action="confirm" />}
              </button>
              {!ok && <span style={{ fontSize: 14, color: "var(--danger)" }}>{planError ?? "Vérifie le chemin dans le panneau Fichier."}</span>}
            </div>
          </div>
          <dl className="stats">
            <div><dt>Temps de jeu</dt><dd>{playtime(game.playtimeSecs)}</dd></div>
            <div><dt>Dernière session</dt><dd>{ago(game.lastPlayed)}</dd></div>
            <div><dt>Ajouté</dt><dd>{ago(game.addedAt)}</dd></div>
          </dl>
        </div>
      </section>

      <div className="d-body">
        <div className="col">
          <section className="section" style={{ gap: 12 }}>
            <h2 className="h2">À propos</h2>
            <p className="prose">
              {game.description ?? "Pas encore de résumé. Les images et les infos sont récupérées automatiquement ; si elles ne correspondent pas, « Ce n'est pas le bon jeu ? » permet de choisir le bon."}
            </p>
            <div className="chips">
              {game.genre && <span>{game.genre}</span>}
              {system && <span>{system.name}</span>}
              <span>{game.kind === "native" ? "Port PC" : game.kind === "store" ? "Steam" : "Émulé"}</span>
            </div>
          </section>

          {showCmd && (
            <section className="section" style={{ gap: 8 }}>
              <h2 className="h2">Commande de lancement</h2>
              {plan ? (
                <pre className="cmd">{[plan.program, ...plan.args].map(quote).join(" ")}{"\n"}# dossier : {plan.workingDir ?? "—"}</pre>
              ) : (
                <pre className="cmd">{planError}</pre>
              )}
            </section>
          )}

          {similar.length > 0 && (
            <section className="section">
              <h2 className="h2">Sur le même système</h2>
              <div className="grid-small">
                {similar.map((g) => (
                  <Link key={g.id} to={`/game/${g.id}`} className="gcard">
                    <Cover game={g} showTitle={false} />
                    <span className="t" style={{ fontSize: 14 }}>{g.title}</span>
                  </Link>
                ))}
              </div>
            </section>
          )}
        </div>

        <aside>
          <section className="panel">
            <div className="head"><h2>Lancement</h2></div>
            {isStore && <div className="kv"><span className="k">Méthode</span><span className="v">Lancé par Steam</span></div>}
            {!isStore && (
              <div className="kv">
                <span className="k">Méthode</span>
                <select className="input" style={{ marginLeft: "auto", width: 200, height: 36, fontSize: 14 }} value={game.kind}
                  onChange={(e) => update((lib) => { const g = lib.games.find((x) => x.id === game.id); if (g) g.kind = e.target.value as typeof g.kind; })}>
                  <option value="emulated">Émulateur</option>
                  <option value="native">Exécutable natif</option>
                </select>
              </div>
            )}
            {game.kind === "emulated" && (
              <div className="kv">
                <span className="k">Émulateur</span>
                <select className="input" style={{ marginLeft: "auto", width: 200, height: 36, fontSize: 14 }} value={game.emulatorOverride ?? ""}
                  onChange={(e) => update((lib) => { const g = lib.games.find((x) => x.id === game.id); if (g) g.emulatorOverride = e.target.value || null; })}>
                  <option value="">Par défaut ({emulators.find((x) => x.id === system?.emulatorId)?.name ?? "aucun"})</option>
                  {emulators.map((em) => <option key={em.id} value={em.id}>{em.name}</option>)}
                </select>
              </div>
            )}
            <div className="kv"><span className="k">Programme</span><span className="v mono">{plan ? (isStore ? plan.program : fileName(plan.program)) : "—"}</span></div>
            <div className="kv"><span className="k">Dossier</span><span className="v mono">{plan?.workingDir ? fileName(plan.workingDir) : "—"}</span></div>
          </section>

          <section className="panel">
            <div className="head"><h2>Images et infos</h2></div>
            <div className="kv"><span className="k">Source</span><span className="v">{sourceLabel}</span></div>
            <div className="kv" style={{ flexWrap: "wrap", justifyContent: "flex-end", padding: "8px 0" }}>
              <button type="button" className="btn btn-ghost" disabled={!!progress && progress.phase !== "idle"} onClick={() => api.refreshMetadata([game.id], true).then(() => notify("Mise à jour lancée"), (e) => notify(String(e), "error"))}>Mettre à jour les images</button>
              {canMatch && <button type="button" className="btn btn-ghost" onClick={() => setMatching(true)}>Ce n'est pas le bon jeu ?</button>}
            </div>
          </section>

          <section className="panel">
            <div className="head">
              <h2>Fichier</h2>
              {api.inTauri && (
                <button type="button" className="link-acc" onClick={async () => {
                  const path = await api.pickFile({ title: "Choisir le fichier du jeu", extensions: game.kind === "native" ? ["exe", "AppImage", "sh"] : system?.extensions });
                  if (path) update((lib) => { const g = lib.games.find((x) => x.id === game.id); if (g) g.path = path; });
                }}>Modifier</button>
              )}
            </div>
            <div className="kv"><span className="k">Nom</span><span className="v mono">{fileName(game.path)}</span></div>
            <div className="kv"><span className="k">Chemin</span><span className="v mono" title={game.path}>{game.path}</span></div>
            <div className="kv"><span className="k">État</span><span className="v" style={{ color: ok ? "var(--acc)" : "var(--danger)" }}>{ok ? "Trouvé" : "Introuvable"}</span></div>
          </section>
        </aside>
      </div>
      {matching && <MatchDialog game={game} onClose={() => setMatching(false)} />}
    </div>
  );
}
