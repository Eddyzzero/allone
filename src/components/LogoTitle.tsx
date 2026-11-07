import { useEffect, useState } from "react";
import { imageSrc } from "../api";
import { useStore } from "../store";

/**
 * Titre d'un jeu : logo officiel s'il charge (avec le titre en `sr-only`),
 * sinon le titre en texte.
 */
export default function LogoTitle({ title, logo }: { title: string; logo?: string | null }) {
  const { imageVersion } = useStore();
  const src = imageSrc(logo, imageVersion);
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [src]);

  if (!src || failed) return <h1>{title}</h1>;
  return (
    <>
      <h1 className="sr-only">{title}</h1>
      <img className="logo-title" src={src} alt={title} onError={() => setFailed(true)} />
    </>
  );
}
