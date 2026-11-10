/**
 * Logique pure des mises à jour : quand revérifier, et quoi montrer après une mise à jour.
 * Testée sans Tauri (voir updateNotes.test.ts).
 */

/** Revérification régulière tant qu'allone reste ouvert. */
export const CHECK_EVERY_MS = 6 * 60 * 60 * 1000;
/** Au retour sur la fenêtre, on revérifie si la dernière vérification date d'au moins une heure. */
export const FOCUS_RECHECK_MS = 60 * 60 * 1000;

export function shouldCheck(now: number, lastCheck: number | null, minGapMs: number): boolean {
  return lastCheck === null || now - lastCheck >= minGapMs;
}

/**
 * Transforme le texte d'une version (section du CHANGELOG) en liste de points.
 * Garde les lignes « - … » ou « * … » ; à défaut, les lignes non vides. Retire le gras Markdown.
 */
export function parseNotes(body: string): string[] {
  const lines = body.split(/\r?\n/).map((l) => l.trim()).filter((l) => l && !l.startsWith("#"));
  const bullets = lines.filter((l) => /^[-*]\s+/.test(l)).map((l) => l.replace(/^[-*]\s+/, ""));
  return (bullets.length > 0 ? bullets : lines).map((l) => l.replace(/\*\*(.+?)\*\*/g, "$1"));
}

export interface PendingUpdate {
  version: string;
  notes: string;
}

export interface WhatsNew {
  version: string;
  notes: string[];
}

/**
 * Au démarrage : faut-il afficher « Nouveautés de la version X » ?
 * - `lastSeen` : version vue au précédent démarrage (null au tout premier lancement) ;
 * - `pending` : mise à jour installée depuis allone, avec son texte, enregistrée avant l'installation.
 */
export function whatsNewOnStart(current: string, lastSeen: string | null, pending: PendingUpdate | null): WhatsNew | null {
  if (lastSeen === current) return null;
  if (pending && pending.version === current) return { version: current, notes: parseNotes(pending.notes) };
  // Version changée sans texte connu (installée à la main) : on le signale quand même.
  if (lastSeen !== null) return { version: current, notes: [] };
  return null; // Premier lancement : rien de « nouveau » à annoncer.
}
