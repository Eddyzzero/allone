import { Link, NavLink, useLocation } from "react-router-dom";
import { inTauri } from "../api";
import { ago } from "../format";
import { useStore } from "../store";
import { useUpdater } from "../updater";
import Cover from "./Cover";
import { Books, Heart, Home, Logo, Pad, Sliders } from "./Icons";

export default function Sidebar() {
  const { library, running, progress } = useStore();
  const { pathname } = useLocation();
  const { available, ready, downloading, install } = useUpdater();
  const games = library?.games ?? [];
  const systems = library?.systems ?? [];

  const recent = [...games]
    .filter((g) => g.lastPlayed)
    .sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0))
    .slice(0, 4);
  const onSystemPage = pathname.startsWith("/system");
  const runningGame = games.find((g) => running.has(g.id));
  const task = progress && progress.phase !== "idle" ? progress : null;
  const taskLabel = !task
    ? null
    : task.phase === "launchbox-download"
      ? `Téléchargement de la base LaunchBox… ${Math.round(task.done / 1_000_000)}${task.total > 0 ? ` / ${Math.round(task.total / 1_000_000)}` : ""} Mo`
      : task.phase === "launchbox-index"
        ? "Analyse de la base LaunchBox…"
        : task.phase === "steam"
          ? "Synchronisation Steam…"
          : `Images : ${task.done}/${task.total}`;

  return (
    <aside className="sidebar">
      <Link to="/" className="brand">
        <span className="brand-logo"><Logo /></span>
        <span className="brand-name">allone</span>
      </Link>

      <nav className="nav" aria-label="Navigation principale">
        <NavLink to="/" end><Home />Accueil</NavLink>
        <NavLink to="/library"><Books />Bibliothèque</NavLink>
        <NavLink to={systems[0] ? `/system/${systems[0].id}` : "/library"} className={onSystemPage ? "active" : ""}>
          <Pad />Systèmes
        </NavLink>
        <NavLink to="/favorites"><Heart filled={false} />Favoris</NavLink>
        <NavLink to="/settings"><Sliders />Réglages</NavLink>
      </nav>

      {onSystemPage ? (
        <section className="side-section">
          <h2 className="side-title" style={{ marginBottom: 4 }}>Mes systèmes</h2>
          {systems.map((s) => {
            const active = pathname === `/system/${s.id}`;
            return (
              <Link key={s.id} to={`/system/${s.id}`} className={`side-sys ${active ? "active" : ""}`} aria-current={active ? "page" : undefined}>
                <span className="dot" style={{ background: s.color }} />
                <span>{s.name}</span>
                <span className="n">{games.filter((g) => g.systemId === s.id).length}</span>
              </Link>
            );
          })}
        </section>
      ) : (
        recent.length > 0 && (
          <section className="side-section">
            <h2 className="side-title">Reprendre</h2>
            {recent.map((g) => (
              <Link key={g.id} to={`/game/${g.id}`} className="side-game">
                <Cover game={g} showTitle={false} className="thumb" />
                <span className="meta">
                  <span className="t">{g.title}</span>
                  <span className="s">
                    {systems.find((s) => s.id === g.systemId)?.shortName} · {ago(g.lastPlayed).toLowerCase()}
                  </span>
                </span>
              </Link>
            ))}
          </section>
        )
      )}

      {available && (
        <section className="side-card side-update" aria-live="polite">
          <strong>{ready ? "Mise à jour prête" : `Version ${available.version} disponible`}</strong>
          {downloading ? (
            <span>Téléchargement…{downloading.total > 0 ? ` ${Math.min(100, Math.round((downloading.done / downloading.total) * 100))} %` : ""}</span>
          ) : (
            <>
              <button type="button" className="btn btn-acc" disabled={running.size > 0} onClick={install}>{ready ? "Installer" : "Mettre à jour"}</button>
              {running.size > 0 && <span>{ready ? "Mise à jour prête — ferme le jeu pour l'installer" : "Ferme le jeu en cours d'abord"}</span>}
            </>
          )}
        </section>
      )}

      <section className="side-card" aria-live="polite">
        {task ? (
          <>
            <strong>{taskLabel}</strong>
            {task.title && <span>{task.title}</span>}
          </>
        ) : runningGame ? (
          <>
            <strong>En cours : {runningGame.title}</strong>
            <span>Le temps de jeu est compté jusqu'à la fermeture du jeu.</span>
          </>
        ) : (
          <>
            <strong>{games.length} jeux · {systems.length} systèmes</strong>
            <span>{inTauri ? "Bibliothèque enregistrée sur ce PC." : "Mode navigateur : données de démonstration."}</span>
          </>
        )}
      </section>
    </aside>
  );
}
