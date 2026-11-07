import { useMemo, useState } from "react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router-dom";
import Cover from "../components/Cover";
import GameCard from "../components/GameCard";
import SystemLogo from "../components/SystemLogo";
import { ChevronLeft, Dice, Grid, Heart, List, Play, Search, Sliders, Sort } from "../components/Icons";
import { playtime } from "../format";
import { useStore } from "../store";
import type { Game } from "../types";

type Mode = "system" | "library" | "favorites";
type Filter = "all" | "fav" | "never";
type SortKey = "az" | "recent" | "playtime";

const SORT_LABEL: Record<SortKey, string> = { az: "A → Z", recent: "Récents", playtime: "Temps de jeu" };
const NEXT_SORT: Record<SortKey, SortKey> = { az: "recent", recent: "playtime", playtime: "az" };
const LETTERS = "#ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("");

function firstLetter(title: string) {
  const c = title.trim()[0]?.toUpperCase() ?? "#";
  return /[A-Z]/.test(c) ? c : "#";
}

export default function Games({ mode, onAdd }: { mode: Mode; onAdd: () => void }) {
  const { library, launch } = useStore();
  const { id } = useParams();
  const [params] = useSearchParams();
  const navigate = useNavigate();
  const [filter, setFilter] = useState<Filter>(mode === "favorites" ? "fav" : "all");
  const [sort, setSort] = useState<SortKey>("az");
  const [view, setView] = useState<"grid" | "list">("grid");
  const [q, setQ] = useState(params.get("q") ?? "");
  const [letter, setLetter] = useState<string | null>(null);

  const system = mode === "system" ? library?.systems.find((s) => s.id === id) : undefined;
  const emulator = library?.emulators.find((e) => e.id === system?.emulatorId);

  const pool = useMemo(() => {
    const all = library?.games ?? [];
    return system ? all.filter((g) => g.systemId === system.id) : all;
  }, [library, system]);

  const shown = useMemo(() => {
    const needle = q.trim().toLowerCase();
    let list = pool.filter((g) => {
      if (filter === "fav" && !g.favorite) return false;
      if (filter === "never" && g.lastPlayed) return false;
      if (letter && firstLetter(g.title) !== letter) return false;
      if (needle) {
        const sys = library?.systems.find((s) => s.id === g.systemId);
        const hay = `${g.title} ${g.genre ?? ""} ${sys?.name ?? ""} ${sys?.shortName ?? ""}`.toLowerCase();
        if (!hay.includes(needle)) return false;
      }
      return true;
    });
    list = [...list].sort((a, b) =>
      sort === "az" ? a.title.localeCompare(b.title, "fr")
        : sort === "recent" ? (b.lastPlayed ?? b.addedAt) - (a.lastPlayed ?? a.addedAt)
          : b.playtimeSecs - a.playtimeSecs,
    );
    return list;
  }, [pool, filter, letter, q, sort, library]);

  const lettersPresent = useMemo(() => new Set(pool.map((g) => firstLetter(g.title))), [pool]);

  if (mode === "system" && library && !system) {
    return <div className="empty">Ce système n'existe pas. <Link className="link-acc" to="/">Retour à l'accueil</Link></div>;
  }

  const lastPlayed = [...pool].filter((g) => g.lastPlayed).sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0))[0];
  const heroGame: Game | undefined = lastPlayed ?? pool.find((g) => g.hero || g.cover);
  const totalTime = pool.reduce((s, g) => s + g.playtimeSecs, 0);
  const random = () => {
    if (!pool.length) return;
    navigate(`/game/${pool[Math.floor(Math.random() * pool.length)].id}`);
  };

  const title = system?.name ?? (mode === "favorites" ? "Favoris" : "Bibliothèque");

  return (
    <>
      <nav className="crumbs" aria-label="Fil d'Ariane">
        <button type="button" className="crumb-back" onClick={() => navigate(-1)}><ChevronLeft size={16} />Retour</button>
        <span>/</span>
        <span style={{ color: "var(--text)" }}>{title}</span>
      </nav>

      {system ? (
        <section className="sys-hero" aria-label={system.name} style={{ background: "var(--hero-bg)" }}>
          {heroGame && <Cover game={heroGame} variant="hero" showTitle={false} style={{ position: "absolute", inset: 0 }} />}
          <span className="veil" />
          <div className="content">
            <span className="eyebrow" style={{ color: system.color }}>Système{system.year ? ` · ${system.year}` : ""}</span>
            <h1>
              <SystemLogo system={system} />
            </h1>
            <p>
              {pool.length} jeux · {emulator ? `émulé avec ${emulator.name}` : "aucun émulateur associé"} · {playtime(totalTime)} de jeu au total
            </p>
            <div className="row-actions">
              {lastPlayed && (
                <button type="button" className="btn btn-acc" onClick={() => launch(lastPlayed)}>
                  <Play size={15} />Reprendre {lastPlayed.title}
                </button>
              )}
              <button type="button" className="btn btn-soft" onClick={random}><Dice />Jeu au hasard</button>
              <Link to="/settings" className="btn btn-soft btn-round" aria-label="Réglages du système"><Sliders /></Link>
            </div>
          </div>
        </section>
      ) : (
        <header style={{ display: "flex", alignItems: "flex-end", gap: 16, flexWrap: "wrap" }}>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <span className="eyebrow" style={{ color: "var(--acc)" }}>{mode === "favorites" ? "Mes coups de cœur" : "Tous les systèmes"}</span>
            <h1 style={{ fontFamily: "var(--display)", fontSize: 48, letterSpacing: "-0.03em", lineHeight: 1 }}>{title}</h1>
          </div>
          <button type="button" className="btn btn-acc" style={{ marginLeft: "auto" }} onClick={onAdd}>Ajouter un jeu</button>
        </header>
      )}

      <div className="toolbar" role="toolbar" aria-label="Filtres">
        <h2 className="h2" style={{ marginRight: 8 }}>Tous les jeux <span className="count">{shown.length} / {pool.length}</span></h2>
        {([["all", "Tous"], ["fav", "Favoris"], ["never", "Jamais joués"]] as [Filter, string][]).map(([k, label]) => (
          <button key={k} type="button" className="pill" aria-pressed={filter === k} onClick={() => setFilter(k)}>{label}</button>
        ))}
        <div className="right">
          <label className="field-search">
            <Search size={16} />
            <span className="sr-only">Chercher</span>
            <input type="search" placeholder={system ? `Chercher dans ${system.shortName}` : "Chercher"} value={q} onChange={(e) => setQ(e.target.value)} />
          </label>
          <button type="button" className="soft-btn" onClick={() => setSort(NEXT_SORT[sort])}><Sort />{SORT_LABEL[sort]}</button>
          <div className="seg">
            <button type="button" aria-label="Grille" aria-pressed={view === "grid"} onClick={() => setView("grid")}><Grid /></button>
            <button type="button" aria-label="Liste" aria-pressed={view === "list"} onClick={() => setView("list")}><List /></button>
          </div>
        </div>
      </div>

      <nav className="letters" aria-label="Aller à la lettre">
        {LETTERS.map((ch) => (
          <button key={ch} type="button" disabled={!lettersPresent.has(ch)} aria-pressed={letter === ch}
            onClick={() => setLetter(letter === ch ? null : ch)}>{ch}</button>
        ))}
      </nav>

      {shown.length === 0 ? (
        <div className="empty">
          <span>{pool.length ? "Aucun jeu ne correspond à ces filtres." : "Aucun jeu pour l'instant."}</span>
          {pool.length === 0 && <button type="button" className="btn btn-acc" onClick={onAdd}>Ajouter un jeu</button>}
        </div>
      ) : view === "grid" ? (
        <div className="grid-games">
          {shown.map((g) => <GameCard key={g.id} game={g} showSystem={!system} />)}
        </div>
      ) : (
        <ul className="list">
          {shown.map((g) => (
            <li key={g.id}>
              <Link to={`/game/${g.id}`}>
                <Cover game={g} showTitle={false} />
                <span className="t">{g.title}</span>
                {!system && <span className="tag">{library?.systems.find((s) => s.id === g.systemId)?.shortName}</span>}
                <span className="c" style={{ flex: "0 1 140px" }}>{g.kind === "native" ? "Port PC" : g.kind === "store" ? "Steam" : g.genre}</span>
                <span className="c" style={{ flex: "0 0 60px" }}>{g.year}</span>
                <span className="c" style={{ flex: "0 0 90px", textAlign: "right" }}>{playtime(g.playtimeSecs)}</span>
                <span style={{ flex: "0 0 36px", display: "flex", justifyContent: "center", color: g.favorite ? "var(--acc)" : "rgba(255,255,255,0.15)" }}>
                  <Heart size={16} />
                </span>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}
