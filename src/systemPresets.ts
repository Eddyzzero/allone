/** Extensions de ROM usuelles, par plateforme LaunchBox. */
export const EXTENSIONS: Record<string, string[]> = {
  "Nintendo GameCube": ["iso", "rvz", "gcm", "gcz", "ciso"],
  "Nintendo Wii": ["iso", "rvz", "wbfs", "wia"],
  "Nintendo Wii U": ["wua", "wud", "wux", "rpx"],
  "Nintendo Switch": ["nsp", "xci", "nsz"],
  "Nintendo 64": ["z64", "n64", "v64"],
  "Nintendo Entertainment System": ["nes"],
  "Super Nintendo Entertainment System": ["sfc", "smc"],
  "Nintendo Game Boy": ["gb"],
  "Nintendo Game Boy Color": ["gbc"],
  "Nintendo Game Boy Advance": ["gba"],
  "Nintendo DS": ["nds"],
  "Nintendo 3DS": ["3ds", "cci", "cxi"],
  "Sony Playstation": ["bin", "cue", "chd", "iso", "pbp"],
  "Sony Playstation Vita": ["vpk"],
  "Sega Dreamcast": ["gdi", "cdi", "chd"],
  "Sega Genesis": ["md", "gen", "bin"],
  "Sega Saturn": ["cue", "chd", "iso"],
  "Microsoft Xbox": ["iso", "xiso"],
  "Microsoft Xbox 360": ["iso", "xex"],
};

/** Les 7 couleurs proposées pour une console. */
export const SYSTEM_COLORS = ["#F2B33D", "#FF4F8B", "#E8673C", "#7BE0A8", "#8FD3FF", "#C9B6FF", "#FFD23F"];

const SHORT: Record<string, string> = {
  "Nintendo GameCube": "GC",
  "Nintendo Wii": "Wii",
  "Nintendo Wii U": "WiiU",
  "Nintendo Switch": "NSW",
  "Nintendo 64": "N64",
  "Nintendo Entertainment System": "NES",
  "Super Nintendo Entertainment System": "SNES",
  "Nintendo Game Boy": "GB",
  "Nintendo Game Boy Color": "GBC",
  "Nintendo Game Boy Advance": "GBA",
  "Nintendo DS": "NDS",
  "Nintendo 3DS": "3DS",
  "Sony Playstation": "PS1",
  "Sony Playstation 2": "PS2",
  "Sony Playstation 3": "PS3",
  "Sony Playstation Vita": "Vita",
  "Sony PSP": "PSP",
  "Sega Dreamcast": "DC",
  "Sega Genesis": "MD",
  "Sega Saturn": "SAT",
  "Microsoft Xbox": "Xbox",
  "Microsoft Xbox 360": "X360",
};

/** Nom court proposé : table connue, sinon initiales (≥ 2 mots) ou premier mot, 6 caractères au plus. */
export function shortNameFor(name: string): string {
  const n = name.trim();
  if (SHORT[n]) return SHORT[n];
  const words = n.split(/\s+/).filter(Boolean);
  if (words.length === 0) return "";
  const s = words.length > 1 ? words.map((w) => (/^\d/.test(w) ? w : w[0])).join("") : words[0];
  return s.slice(0, 6);
}

/** Identifiant unique (a-z0-9-) dérivé du nom court ; suffixe -2, -3… si déjà pris. */
export function uniqueSystemId(shortName: string, existingIds: string[]): string {
  const base = shortName.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "console";
  const taken = new Set([...existingIds, "steam"]); // « steam » est réservé
  if (!taken.has(base)) return base;
  let i = 2;
  while (taken.has(`${base}-${i}`)) i++;
  return `${base}-${i}`;
}
