import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import * as api from "../api";
import { useStore } from "../store";
import type { Game } from "../types";
import { Close } from "./Icons";
import PadHint from "./PadHint";

/** Confirmation avant de retirer un jeu de la bibliothèque (le fichier reste sur le disque). */
export default function RemoveGameDialog({ game, onClose }: { game: Game; onClose: () => void }) {
  const { reload, notify, running } = useStore();
  const navigate = useNavigate();
  const [busy, setBusy] = useState(false);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const isStore = game.kind === "store";

  // Échap pour fermer ; le focus part sur « Annuler », le choix sans risque.
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    cancelRef.current?.focus();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && closeRef.current();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  async function confirm() {
    setBusy(true);
    try {
      await api.removeGame(game.id);
      await reload();
      notify(`« ${game.title} » a été retiré de la bibliothèque.`, "ok");
      navigate("/library", { replace: true });
    } catch (e) {
      notify(String(e), "error");
      setBusy(false);
    }
  }

  return (
    <div className="overlay" onClick={(e) => e.target === e.currentTarget && onClose()}>
      <div className="dialog dialog-sm" role="dialog" aria-modal="true" aria-labelledby="remove-title">
        <header>
          <h2 id="remove-title">Retirer « {game.title} » ?</h2>
          <PadHint action="back" />
          <button type="button" className="x" onClick={onClose} aria-label="Fermer"><Close /></button>
        </header>
        <div className="content remove-body">
          <p>Le jeu disparaît d'allone, avec ses images et son temps de jeu.</p>
          {isStore ? (
            <p>Il reste installé dans Steam et ne sera plus importé automatiquement.</p>
          ) : (
            <>
              <p>Le fichier reste sur ton disque, rien n'est effacé :</p>
              <code className="remove-path">{game.path}</code>
              <p>L'analyse des dossiers ne le proposera plus. Tu peux toujours le rajouter avec « Ajouter un jeu ».</p>
            </>
          )}
        </div>
        <footer>
          <span className="note">Réglages › Bibliothèques permet de rétablir les jeux retirés.</span>
          <button ref={cancelRef} type="button" className="btn btn-ghost" onClick={onClose}>Annuler</button>
          <button type="button" className="btn btn-danger remove-confirm" onClick={confirm} disabled={busy || running.has(game.id)}>
            {busy ? "Retrait…" : "Retirer"}
          </button>
        </footer>
      </div>
    </div>
  );
}
