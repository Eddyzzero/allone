export function playtime(secs: number): string {
  if (secs < 60) return secs > 0 ? "< 1 min" : "0 h";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h === 0) return `${m} min`;
  return `${h} h ${String(m).padStart(2, "0")}`;
}

export function ago(unix?: number | null): string {
  if (!unix) return "Jamais";
  const d = Math.floor(Date.now() / 1000) - unix;
  if (d < 60) return "À l'instant";
  if (d < 3600) return `Il y a ${Math.floor(d / 60)} min`;
  if (d < 86400) return `Il y a ${Math.floor(d / 3600)} h`;
  const days = Math.floor(d / 86400);
  if (days === 1) return "Hier";
  if (days < 7) return `Il y a ${days} j`;
  if (days < 30) return `Il y a ${Math.floor(days / 7)} sem.`;
  return new Date(unix * 1000).toLocaleDateString("fr-FR", { day: "numeric", month: "short", year: "numeric" });
}

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export function bytes(n: number): string {
  if (n < 1024 * 1024) return `${Math.round(n / 1024)} Ko`;
  if (n < 1024 ** 3) return `${(n / 1024 ** 2).toFixed(1).replace(".", ",")} Mo`;
  return `${(n / 1024 ** 3).toFixed(2).replace(".", ",")} Go`;
}

/** Couleurs de repli (formes abstraites du design) quand un jeu n'a pas d'image. */
const PALETTE: [string, string, string][] = [
  ["#1E3A5F", "#F2B33D", "#13283F"],
  ["#2A1446", "#FF4F8B", "#1C0D30"],
  ["#3B1F14", "#E8673C", "#26130C"],
  ["#13322A", "#7BE0A8", "#0B211B"],
  ["#0F2F4A", "#8FD3FF", "#0A1F33"],
  ["#2B2B3F", "#C9B6FF", "#1B1B2A"],
  ["#3A0F1A", "#FFD23F", "#260910"],
];

export function paletteFor(id: string): [string, string, string] {
  let h = 0;
  for (const c of id) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return PALETTE[h % PALETTE.length];
}
