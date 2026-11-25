import type { MouseEvent } from "react";
import { Link } from "react-router-dom";
import * as api from "../api";
import { useStore } from "../store";
import { isNotInstalled, steamAppId } from "../steamState";
import type { Game } from "../types";
import Cover from "./Cover";
import { Download, Heart } from "./Icons";

const WEEK = 7 * 24 * 3600;

export default function GameCard({ game, showSystem = true }: { game: Game; showSystem?: boolean }) {
  const { library, fileOk, notify } = useStore();
  const system = library?.systems.find((s) => s.id === game.systemId);
  const isNew = !game.lastPlayed && Date.now() / 1000 - game.addedAt < WEEK;
  const notInstalled = isNotInstalled(game, fileOk);
  const missing = !notInstalled && fileOk[game.id] === false;
  const appId = steamAppId(game);

  // Le bouton est dans le lien de la carte : on n'ouvre pas la fiche.
  const install = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (appId == null) return;
    api.installSteamGame(appId).then(
      () => notify(`Steam ouvre l'installation de ${game.title}.`),
      (err) => notify(String(err), "error"),
    );
  };

  return (
    <Link className={notInstalled ? "gcard off" : "gcard"} to={`/game/${game.id}`}>
      {/* Le titre est déjà écrit sous la carte : pas de doublon sur la jaquette. */}
      <Cover game={game} showTitle={false}>
        <span className="badge-tl">
          {notInstalled ? (
            <span className="badge-port">Non installé</span>
          ) : missing ? (
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
        {/* Souris uniquement : à la manette, A ouvre la fiche, où « Installer » a le focus. */}
        {notInstalled && appId != null && (
          <button type="button" className="card-install" tabIndex={-1} onClick={install}>
            <Download size={16} />Installer
          </button>
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
