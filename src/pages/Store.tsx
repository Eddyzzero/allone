import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import { metadataStatus, openShopPage, shopFeatured, shopSearch } from "../api";
import { Check, Close, External, Logo, Play, Search } from "../components/Icons";
import InterfaceSwitch from "../components/InterfaceSwitch";
import PadHint from "../components/PadHint";
import { paletteFor } from "../format";
import {
  controllerText,
  deckText,
  offerEnds,
  priceOf,
  priceText,
  releaseText,
  reviewText,
  reviewTone,
  screenshotThumb,
} from "../shopFormat";
import { useStore } from "../store";
import { isNotInstalled } from "../steamState";
import type { Game, ShopItem, ShopShelf } from "../types";

/** Image de la boutique, remplacée par les formes de couleur d'allone si elle manque ou ne charge pas. */
function ShopImage({ src, fallback, appId, className, alt = "" }: {
  src?: string | null;
  fallback?: string | null;
  appId: number;
  className?: string;
  alt?: string;
}) {
  const [url, setUrl] = useState(src ?? fallback ?? null);
  useEffect(() => setUrl(src ?? fallback ?? null), [src, fallback]);
  if (!url) {
    const [c1, c2] = paletteFor(String(appId));
    return (
      <span className={`shop-img shop-img--empty ${className ?? ""}`} style={{ background: c1 }} role={alt ? "img" : undefined} aria-label={alt || undefined}>
        <span className="blob" style={{ background: c2 }} />
      </span>
    );
  }
  return (
    <img className={`shop-img ${className ?? ""}`} src={url} alt={alt} loading="lazy"
      onError={() => setUrl(url !== fallback && fallback ? fallback : null)} />
  );
}

function Price({ item, size = "sm" }: { item: ShopItem; size?: "sm" | "lg" }) {
  const price = priceOf(item);
  return (
    <span className={`shop-price shop-price--${size}`}>
      {item.discountPct > 0 && <span className="disc">-{item.discountPct}&nbsp;%</span>}
      <span className="amounts">
        {item.discountPct > 0 && item.originalPrice && <s>{priceText(item.originalPrice)}</s>}
        <span className="now">{price ?? (item.comingSoon ? "Bientôt" : "—")}</span>
      </span>
    </span>
  );
}

export default function Store() {
  const { library, launch, running, notify, fileOk } = useStore();
  const [shelves, setShelves] = useState<ShopShelf[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState("specials");
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<{ query: string; items: ShopItem[] } | null>(null);
  const [searching, setSearching] = useState(false);
  const [selId, setSelId] = useState<number | null>(null);
  const [shot, setShot] = useState(0);
  const [steamInstalled, setSteamInstalled] = useState(true);

  function load(force = false) {
    setError(null);
    shopFeatured(force)
      .then(setShelves)
      .catch((e) => setError(String(e)));
  }

  useEffect(() => load(), []);
  useEffect(() => {
    metadataStatus().then((s) => setSteamInstalled(!!s.steamRoot)).catch(() => {});
  }, []);

  // Jeux Steam déjà présents dans la bibliothèque, par identifiant d'application.
  const owned = useMemo(() => {
    const map = new Map<number, Game>();
    for (const g of library?.games ?? []) {
      if (g.store?.store === "steam") map.set(Number(g.store.id), g);
    }
    return map;
  }, [library]);

  const list = results ? results.items : (shelves?.find((s) => s.id === tab)?.items ?? []);
  const sel = list.find((i) => i.appId === selId) ?? list[0] ?? null;
  const selGame = sel ? owned.get(sel.appId) : undefined;

  useEffect(() => setShot(0), [sel?.appId]);

  function pickTab(id: string) {
    setTab(id);
    setResults(null);
    setSelId(null);
  }

  function submit(e: FormEvent) {
    e.preventDefault();
    const q = query.trim();
    if (!q) {
      setResults(null);
      return;
    }
    setSearching(true);
    shopSearch(q)
      .then((items) => {
        setResults({ query: q, items });
        setSelId(null);
      })
      .catch((err) => notify(String(err), "error"))
      .finally(() => setSearching(false));
  }

  async function buy(item: ShopItem) {
    try {
      await openShopPage(item.appId);
    } catch (e) {
      notify(String(e), "error");
    }
  }

  const buyLabel = (item: ShopItem) =>
    item.comingSoon ? (item.price ? "Précommander sur Steam" : "Voir la page Steam") : item.isFree ? "Installer avec Steam" : "Acheter sur Steam";
  const shots = sel?.screenshots ?? [];
  const mainShot = shots[shot] ?? null;

  return (
    <div className="store">
      <header className="store-top">
        <Link to="/" className="brand" aria-label="allone, accueil">
          <span className="brand-logo"><Logo /></span>
          <span className="brand-name">allone</span>
        </Link>
        <div className="store-switch"><InterfaceSwitch current="store" /></div>
        <form role="search" className="search store-search" onSubmit={submit}>
          <label htmlFor="shop-q" style={{ display: "flex" }}>
            <Search />
            <span className="sr-only">Rechercher sur Steam</span>
          </label>
          <input id="shop-q" type="search" placeholder="Rechercher sur Steam" value={query} onChange={(e) => setQuery(e.target.value)} />
          {searching && <span className="store-busy" aria-live="polite">Recherche…</span>}
        </form>
      </header>

      <nav className="store-tabs" aria-label="Rayons" data-pad-tabs>
        {results ? (
          <>
            <button type="button" aria-pressed="true">
              Résultats pour « {results.query} » <span className="n">{results.items.length}</span>
            </button>
            <button type="button" className="store-clear" onClick={() => { setResults(null); setQuery(""); setSelId(null); }} aria-label="Effacer la recherche">
              <Close size={16} />
            </button>
          </>
        ) : (
          (shelves ?? []).map((s) => (
            <button key={s.id} type="button" aria-pressed={s.id === tab} onClick={() => pickTab(s.id)}>
              {s.label} <span className="n">{s.items.length}</span>
            </button>
          ))
        )}
      </nav>

      {error ? (
        <div className="empty store-error">
          <span>La boutique Steam ne répond pas pour l'instant.</span>
          <span className="store-error-detail">{error}</span>
          <button type="button" className="btn btn-acc" onClick={() => load(true)}>Réessayer</button>
        </div>
      ) : !shelves && !results ? (
        <div className="store-body" aria-busy="true">
          <ul className="store-list" aria-label="Chargement">
            {Array.from({ length: 8 }, (_, i) => <li key={i} className="store-row store-row--ghost" />)}
          </ul>
        </div>
      ) : list.length === 0 ? (
        <div className="empty">{results ? `Aucun jeu trouvé pour « ${results.query} ».` : "Ce rayon est vide pour l'instant."}</div>
      ) : (
        <div className="store-body">
          <ul className="store-list" aria-label="Jeux">
            {list.map((item) => {
              const on = item.appId === sel?.appId;
              const mine = owned.has(item.appId);
              return (
                <li key={item.appId}>
                  <button type="button" className="store-row" aria-pressed={on} onClick={() => setSelId(item.appId)}>
                    <ShopImage src={item.thumb} fallback={item.capsule} appId={item.appId} className="thumb" />
                    <span className="meta">
                      <span className="t">{item.name}</span>
                      <span className={`s ${mine ? "mine" : ""}`}>{mine ? "Dans ta bibliothèque" : reviewText(item)}</span>
                    </span>
                    <Price item={item} />
                  </button>
                </li>
              );
            })}
          </ul>

          {sel && (
            <article className="store-detail" aria-labelledby="shop-title">
              <div className="store-media">
                <ShopImage key={mainShot ?? sel.capsule ?? sel.appId} src={mainShot} fallback={sel.capsule} appId={sel.appId}
                  className="main" alt={mainShot ? `Capture ${shot + 1} de ${sel.name}` : sel.name} />
                {shots.length > 1 && (
                  <div className="store-shots">
                    {shots.map((s, i) => (
                      <button key={s} type="button" aria-label={`Capture ${i + 1}`} aria-pressed={i === shot} onClick={() => setShot(i)}>
                        <ShopImage src={screenshotThumb(s)} fallback={s} appId={sel.appId} />
                      </button>
                    ))}
                  </div>
                )}
                <div className="store-info">
                  <h1 id="shop-title">{sel.name}</h1>
                  {sel.developers.length > 0 && <p className="by">{sel.developers.join(", ")}</p>}
                  {sel.description && <p className="prose">{sel.description}</p>}
                </div>
              </div>

              <aside className="store-side">
                <section className="panel store-buy" aria-label="Achat">
                  {selGame ? (
                    <>
                      <span className="owned"><Check size={18} />Déjà dans ta bibliothèque</span>
                      <button type="button" className="btn btn-acc" data-pad-autofocus onClick={() => launch(selGame)} disabled={running.has(selGame.id)}>
                        <Play size={14} />{running.has(selGame.id) ? "En cours" : isNotInstalled(selGame, fileOk) ? "Installer" : "Jouer"}
                        <PadHint action="confirm" />
                      </button>
                      <Link to={`/game/${selGame.id}`} className="btn btn-ghost">Voir la fiche</Link>
                    </>
                  ) : (
                    <>
                      <Price item={sel} size="lg" />
                      {offerEnds(sel) && <span className="ends">{offerEnds(sel)}</span>}
                      <button type="button" className="btn btn-light" data-pad-autofocus onClick={() => buy(sel)}>
                        {buyLabel(sel)}<External />
                        <PadHint action="confirm" />
                      </button>
                      <span className="note">
                        {steamInstalled
                          ? "La page s'ouvre dans Steam : le paiement se fait là-bas. Une fois installé, le jeu apparaît dans allone."
                          : "Steam n'est pas installé sur ce PC : la page s'ouvre dans ton navigateur."}
                      </span>
                    </>
                  )}
                </section>

                <section className="panel" aria-label="Évaluations et infos">
                  <div className="store-reviews">
                    <strong className={`tone-${reviewTone(sel)}`}>{reviewText(sel)}</strong>
                    {sel.reviewCount > 0 && <span>{sel.reviewCount.toLocaleString("fr-BE")} évaluations</span>}
                  </div>
                  {sel.reviewPct != null && sel.reviewCount > 0 && (
                    <div className="store-bar" role="img" aria-label={`${sel.reviewPct} % d'évaluations positives`}>
                      <span className={`tone-${reviewTone(sel)}`} style={{ width: `${sel.reviewPct}%` }} />
                    </div>
                  )}
                  <div className="kv"><span className="k">Sortie</span><span className="v">{releaseText(sel)}</span></div>
                  <div className="kv"><span className="k">Français</span><span className="v">{sel.french ?? "Non"}</span></div>
                  <div className="kv"><span className="k">Manette</span><span className="v">{controllerText(sel)}</span></div>
                  {deckText(sel.deck) && <div className="kv"><span className="k">Steam Deck</span><span className="v">{deckText(sel.deck)}</span></div>}
                </section>
              </aside>
            </article>
          )}
        </div>
      )}
    </div>
  );
}
