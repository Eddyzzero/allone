import { useGamepad } from "../gamepad/GamepadProvider";
import { buttonGlyphs } from "../gamepad/logic";

/** Glyphe du bouton de la manette (A/B ou ✕/○), visible seulement quand la manette est en cours d'usage. */
export default function PadHint({ action }: { action: "confirm" | "back" }) {
  const { pad, active } = useGamepad();
  if (!pad || !active) return null;
  return <span className="pad-hint" aria-hidden="true">{buttonGlyphs(pad.family)[action]}</span>;
}
