import { useStore } from "../store";
import { parseNotes } from "../updateNotes";
import { useUpdater } from "../updater";
import { Check, Close, Upload } from "./Icons";

/** Nombre de nouveautés montrées dans le bandeau ; le reste est résumé. */
const MAX_NOTES = 3;

function Notes({ notes }: { notes: string[] }) {
  if (notes.length === 0) return null;
  const shown = notes.slice(0, MAX_NOTES);
  const more = notes.length - shown.length;
  return (
    <ul className="upd-notes">
      {shown.map((n) => <li key={n}>{n}</li>)}
      {more > 0 && <li className="upd-more">… et {more} autre{more > 1 ? "s" : ""} changement{more > 1 ? "s" : ""}</li>}
    </ul>
  );
}

/**
 * En haut de chaque page : le bandeau « Nouvelle version disponible » (visible sans fenêtre
 * surgissante), puis, après une mise à jour, l'encart « Nouveautés de la version X ».
 */
export default function UpdateBanner() {
  const { running } = useStore();
  const { available, dismissed, dismiss, ready, downloading, install, whatsNew, closeWhatsNew } = useUpdater();
  const gameRunning = running.size > 0;
  const pct = downloading && downloading.total > 0 ? Math.min(100, Math.round((downloading.done / downloading.total) * 100)) : null;

  return (
    <>
      {available && !dismissed && (
        <section className="upd-banner" role="status" aria-live="polite">
          <span className="upd-icon" aria-hidden="true"><Upload size={20} /></span>
          <div className="upd-body">
            <strong>{ready ? `La version ${available.version} est prête à être installée` : `Nouvelle version ${available.version} disponible`}</strong>
            <Notes notes={parseNotes(available.notes)} />
            {gameRunning && <span className="upd-hint">{ready ? "Ferme le jeu en cours pour l'installer." : "Ferme le jeu en cours pour mettre à jour."}</span>}
          </div>
          <div className="upd-actions">
            {downloading ? (
              <span className="upd-progress">Téléchargement…{pct !== null ? ` ${pct} %` : ""}</span>
            ) : (
              <>
                <button type="button" className="btn btn-acc" disabled={gameRunning} onClick={install}>{ready ? "Installer" : "Mettre à jour"}</button>
                <button type="button" className="btn btn-ghost" onClick={dismiss}>Plus tard</button>
              </>
            )}
          </div>
        </section>
      )}
      {whatsNew && (
        <section className="upd-banner upd-done" role="status" aria-live="polite">
          <span className="upd-icon" aria-hidden="true"><Check size={20} /></span>
          <div className="upd-body">
            <strong>allone a été mis à jour en version {whatsNew.version}</strong>
            {whatsNew.notes.length > 0 ? (
              <ul className="upd-notes">{whatsNew.notes.map((n) => <li key={n}>{n}</li>)}</ul>
            ) : (
              <span className="upd-hint">La liste des changements est sur la page des versions d'allone sur GitHub.</span>
            )}
          </div>
          <div className="upd-actions">
            <button type="button" className="icon-btn" aria-label="Fermer les nouveautés" onClick={closeWhatsNew}><Close /></button>
          </div>
        </section>
      )}
    </>
  );
}
