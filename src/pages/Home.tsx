import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import Cover from "../components/Cover";
import GameCard from "../components/GameCard";
import { Clock, Heart, Play } from "../components/Icons";
import LogoTitle from "../components/LogoTitle";
import PadHint from "../components/PadHint";
import SystemLogo from "../components/SystemLogo";
import Topbar from "../components/Topbar";
import { autoplayOn, focusLeaves } from "./carousel";
import { ago, playtime } from "../format";
import { useStore } from "../store";
import type { Game } from "../types";

// Positions 3D du carrousel (reprises du design).
const POS: Record<string, { tf: string; z: number; op: number; br: number }> = {
  "0": { tf: "perspective(1400px) translateX(-50%)", z: 5, op: 1, br: 1 },
  "1": { tf: "perspective(1400px) translateX(-21%) rotateY(-16deg) scale(0.9)", z: 4, op: 1, br: 0.72 },
  "-1": { tf: "perspective(1400px) translateX(-79%) rotateY(16deg) scale(0.9)", z: 4, op: 1, br: 0.72 },
  "2": { tf: "perspective(1400px) translateX(14%) rotateY(-26deg) scale(0.8)", z: 3, op: 1, br: 0.5 },
  "-2": { tf: "perspective(1400px) translateX(-114%) rotateY(26deg) scale(0.8)", z: 3, op: 1, br: 0.5 },
  r: { tf: "perspective(1400px) translateX(40%) rotateY(-32deg) scale(0.7)", z: 1, op: 0, br: 0.4 },
  l: { tf: "perspective(1400px) translateX(-140%) rotateY(32deg) scale(0.7)", z: 1, op: 0, br: 0.4 },
};

function statusLine(g: Game, emulator: string | undefined): string {
  const how = g.kind === "store" ? "Lancé par Steam" : g.kind === "native" ? "Port PC natif" : emulator ? `Émulé avec ${emulator}` : "Aucun émulateur";
  if (!g.lastPlayed) return `${how} · ajouté ${ago(g.addedAt).toLowerCase()} · jamais lancé`;
  return `${how} · dernière session ${ago(g.lastPlayed).toLowerCase()}`;
}

/** Vrai si le focus vient de la manette ou du clavier (pas d'un simple clic de souris). */
function focusIsPadOrKeyboard(target: EventTarget): boolean {
  if (document.documentElement.dataset.input === "pad") return true;
  try {
    return target instanceof Element && target.matches(":focus-visible");
  } catch {
    return true;
  }
}

export default function Home({ onAdd }: { onAdd: () => void }) {
  const { library, launch, toggleFavorite, running } = useStore();
  const [active, setActive] = useState(0);
  const [hover, setHover] = useState(false);
  const [focused, setFocused] = useState(false);
  const navigate = useNavigate();
  const sectionRef = useRef<HTMLElement>(null);
  const refocus = useRef(false);

  const games = library?.games ?? [];
  const systems = library?.systems ?? [];

  // Carrousel : les jeux joués récemment, complétés par les derniers ajoutés.
  const hero = useMemo(() => {
    const played = [...games].filter((g) => g.lastPlayed).sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0));
    const rest = [...games].filter((g) => !g.lastPlayed).sort((a, b) => b.addedAt - a.addedAt);
    return [...played, ...rest].slice(0, 6);
  }, [games]);
  const recentlyAdded = useMemo(() => [...games].sort((a, b) => b.addedAt - a.addedAt).slice(0, 12), [games]);

  useEffect(() => {
    if (!autoplayOn(hover, focused, hero.length)) return;
    const t = setInterval(() => setActive((a) => (a + 1) % hero.length), 5000);
    return () => clearInterval(t);
  }, [hover, focused, hero.length]);

  const emulatorOf = (g: Game) => {
    const sys = systems.find((s) => s.id === g.systemId);
    return library?.emulators.find((e) => e.id === (g.emulatorOverride ?? sys?.emulatorId))?.name;
  };

  const N = hero.length;

  // Si le focus était dans le carrousel, il suit le nouveau jeu central.
  useEffect(() => {
    if (!refocus.current) return;
    refocus.current = false;
    sectionRef.current?.querySelector<HTMLElement>(".cta")?.focus();
  }, [active]);

  function onCarouselKey(e: KeyboardEvent<HTMLElement>) {
    if (N < 2 || (e.key !== "ArrowLeft" && e.key !== "ArrowRight")) return;
    e.preventDefault();
    refocus.current = !!sectionRef.current?.contains(document.activeElement);
    setActive((a) => (a + (e.key === "ArrowRight" ? 1 : -1) + N) % N);
  }

  return (
    <>
      <Topbar onAdd={onAdd} />

      {library && library.games.length === 0 && (
        <section className="welcome" aria-labelledby="welcome-title">
          <h1 id="welcome-title" className="h2">Bienvenue dans allone</h1>
          <p>Ta bibliothèque est vide pour l'instant. Commence par indiquer où se trouvent tes émulateurs et tes jeux.</p>
          <div className="welcome-actions">
            <button type="button" className="btn btn-acc" onClick={() => navigate("/settings")}>Configurer mes émulateurs</button>
            <button type="button" className="btn btn-ghost" onClick={onAdd}>Ajouter un jeu</button>
            <button type="button" className="btn btn-ghost" onClick={() => navigate("/settings")}>Ajouter une console</button>
          </div>
          <p className="welcome-note">Les jeux Steam installés apparaissent automatiquement.</p>
        </section>
      )}

      {N > 0 && (
        <section ref={sectionRef} aria-label="À reprendre" aria-roledescription="carrousel" onKeyDown={onCarouselKey}
          onFocus={(e) => setFocused(focusIsPadOrKeyboard(e.target))}
          onBlur={(e) => focusLeaves(e.currentTarget, e.relatedTarget) && setFocused(false)} style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          <div className="carousel" onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}>
            {hero.map((g, i) => {
              let d = (i - active + N) % N;
              if (d > N / 2) d -= N;
              const p = POS[String(d)] ?? (d > 0 ? POS.r : POS.l);
              const center = d === 0;
              const sys = systems.find((s) => s.id === g.systemId);
              return (
                <div key={g.id} className="slide" aria-hidden={!center}
                  style={{ transform: p.tf, zIndex: p.z, opacity: p.op, filter: `brightness(${p.br})`, visibility: Math.abs(d) > 2 ? "hidden" : "visible" }}>
                  <div className="card">
                    <Cover game={g} variant="hero" showTitle={false} style={{ position: "absolute", inset: 0 }} />
                    {center ? (
                      <>
                        <span className="grad" />
                        <button type="button" className={`fav ${g.favorite ? "on" : ""}`} onClick={() => toggleFavorite(g)}
                          aria-label={g.favorite ? "Retirer des favoris" : "Ajouter aux favoris"} aria-pressed={g.favorite}>
                          <Heart filled={g.favorite} />
                        </button>
                        <span className="sys tag-lg" style={{ background: "rgb(var(--ground-rgb) / 0.6)" }}>
                          {g.kind === "native" ? `${sys?.shortName} · Port PC` : g.kind === "store" ? "Steam" : sys?.shortName}
                        </span>
                        <div className="body">
                          <LogoTitle title={g.title} logo={g.logo} />
                          <p>{statusLine(g, emulatorOf(g))}</p>
                          <div className="actions">
                            <button type="button" data-pad-keys className="btn btn-acc cta" onClick={() => launch(g)} disabled={running.has(g.id)}>
                              {running.has(g.id) ? "En cours" : g.lastPlayed ? "Continuer" : "Jouer"}
                              {!running.has(g.id) && <PadHint action="confirm" />}
                            </button>
                            <Link to={`/game/${g.id}`} className="more" aria-label={`Voir la fiche de ${g.title}`}>
                              <Play size={14} />
                            </Link>
                            <span className="time"><Clock />{playtime(g.playtimeSecs)}</span>
                          </div>
                        </div>
                      </>
                    ) : (
                      <button type="button" data-pad-keys className="pick" onClick={() => setActive(i)} aria-label={`Afficher ${g.title}`} tabIndex={Math.abs(d) > 2 ? -1 : 0} />
                    )}
                  </div>
                </div>
              );
            })}
          </div>
          <div className="dots">
            {hero.map((g, i) => (
              <button key={g.id} type="button" onClick={() => setActive(i)} aria-label={`Aller à ${g.title}`} aria-current={i === active}>
                <span style={{ width: i === active ? 22 : 6, background: i === active ? "var(--acc)" : "rgba(255,255,255,0.22)" }} />
              </button>
            ))}
          </div>
        </section>
      )}

      <section className="section">
        <div className="section-head">
          <h2 className="h2">Systèmes</h2>
          <Link to="/settings" className="link-acc">Configurer</Link>
        </div>
        <div className="grid-sys">
          {systems.map((s) => (
            <Link key={s.id} to={`/system/${s.id}`} className="syscard">
              <span className="bar" style={{ background: s.color }} />
              <span style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                <SystemLogo system={s} />
                <span className="c">{games.filter((g) => g.systemId === s.id).length} jeux</span>
              </span>
            </Link>
          ))}
        </div>
      </section>

      <section className="section">
        <div className="section-head">
          <h2 className="h2">Ajoutés récemment</h2>
          <Link to="/library" className="link-acc">Tout voir</Link>
        </div>
        {recentlyAdded.length ? (
          <div className="grid-games">
            {recentlyAdded.map((g) => <GameCard key={g.id} game={g} />)}
          </div>
        ) : (
          <div className="empty">
            <span>Ta bibliothèque est vide.</span>
            <button type="button" className="btn btn-acc" onClick={onAdd}>Ajouter un jeu</button>
          </div>
        )}
      </section>
    </>
  );
}
