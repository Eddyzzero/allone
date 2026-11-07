import { useEffect, useState, type CSSProperties } from "react";
import { imageSrc } from "../api";
import { paletteFor } from "../format";
import { useStore } from "../store";
import type { Game } from "../types";

interface Props {
  game: Game;
  /** "hero" utilise l'image large ; sinon la jaquette. */
  variant?: "cover" | "hero";
  showTitle?: boolean;
  className?: string;
  style?: CSSProperties;
  children?: React.ReactNode;
}

/**
 * Jaquette d'un jeu. Sans image (ou si elle ne charge pas), on dessine les
 * formes abstraites du design avec une palette dérivée de l'identifiant.
 */
export default function Cover({ game, variant = "cover", showTitle = true, className = "", style, children }: Props) {
  const { imageVersion } = useStore();
  const src = imageSrc(variant === "hero" ? game.hero ?? game.cover : game.cover ?? game.hero, imageVersion);
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [src]);
  const [c1, c2, c3] = paletteFor(game.id);
  const hasImage = !!src && !failed;

  return (
    <span className={`cover ${className}`} style={{ background: c1, ...style }} role="img" aria-label={`Jaquette de ${game.title}`}>
      {hasImage && variant === "cover" ? (
        <>
          {/* Jaquette entière (pas de rognage), bords remplis par la même image floutée. */}
          <img className="fill" src={src!} alt="" aria-hidden="true" loading="lazy" />
          <img className="fit" src={src!} alt="" loading="lazy" onError={() => setFailed(true)} />
        </>
      ) : hasImage ? (
        <img src={src!} alt="" loading="lazy" onError={() => setFailed(true)} />
      ) : variant === "hero" ? (
        <>
          <span className="blob" style={{ width: "95%", aspectRatio: "1", right: "-22%", top: "-45%", background: c2 }} />
          <span className="ring" style={{ width: "56%", aspectRatio: "1", right: "10%", top: "-12%" }} />
          <span className="arch" style={{ width: "32%", height: "70%", right: "34%", bottom: "-12%", background: c3 }} />
        </>
      ) : (
        <>
          <span className="blob" style={{ width: "120%", aspectRatio: "1", right: "-55%", top: "-30%", background: c2 }} />
          <span className="ring" style={{ width: "40%", aspectRatio: "1", left: "12%", bottom: "26%" }} />
        </>
      )}
      {showTitle && (
        <>
          {hasImage && <span className="shade" />}
          <span className="ctitle">{game.title}</span>
        </>
      )}
      {children}
    </span>
  );
}
