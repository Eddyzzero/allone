import { Link } from "react-router-dom";
import { useStore } from "../store";
import type { Game } from "../types";
import Cover from "./Cover";
import { Heart } from "./Icons";

const WEEK = 7 * 24 * 3600;

export default function GameCard({ game, showSystem = true }: { game: Game; showSystem?: boolean }) {
  const { library, fileOk } = useStore();
  const system = library?.systems.find((s) => s.id === game.systemId);
  const isNew = !game.lastPlayed && Date.now() / 1000 - game.addedAt < WEEK;
  const missing = fileOk[game.id] === false;

  return (
    <Link className="gcard" to={`/game/${game.id}`}>
      {/* Le titre est déjà écrit sous la carte : pas de doublon sur la jaquette. */}
      <Cover game={game} showTitle={false}>
        <span className="badge-tl">
          {missing ? (
            <span className="badge-missing">Introuvable</span>
          ) : game.kind === "native" ? (
            <span className="badge-port">Port PC</span>
          ) : game.kind === "store" ? (
            <span className="badge-port">Steam</span>
          ) : isNew ? (
            <span className="badge-new">Nouveau</span>
          ) : null}
        </span>
        {game.favorite && (
          <span className="badge-tr badge-fav">
            <Heart size={14} />
            <span className="sr-only">Favori</span>
          </span>
        )}
      </Cover>
      {showSystem && (
        <span className="row">
          <span className="tag">{system?.shortName ?? game.systemId}</span>
          {game.year && <span className="s">{game.year}</span>}
        </span>
      )}
      <span style={{ display: "flex", flexDirection: "column", gap: 2 }}>
        <span className="t">{game.title}</span>
        <span className="s">
          {showSystem ? game.genre ?? "—" : [game.year, game.genre].filter(Boolean).join(" · ")}
        </span>
      </span>
    </Link>
  );
}
