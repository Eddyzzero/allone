import { useEffect, useState } from "react";
import { imageSrc } from "../api";
import { useStore } from "../store";
import type { System } from "../types";

/**
 * Logo d'un système : les logos livrés (/systems/…) passent en blanc pour le thème sombre,
 * ceux choisis par l'utilisateur gardent leurs couleurs.
 * Sans logo (ou s'il ne charge pas), on affiche simplement son nom.
 */
export default function SystemLogo({ system, className = "" }: { system: System; className?: string }) {
  const { imageVersion } = useStore();
  const src = imageSrc(system.logo, imageVersion);
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [src]);

  if (!src || failed) return <span className={`sys-name ${className}`}>{system.name}</span>;
  return <img className={`sys-logo${system.logo?.startsWith("/systems/") ? " sys-logo--mono" : ""} ${className}`} src={src} alt={system.name} onError={() => setFailed(true)} />;
}
