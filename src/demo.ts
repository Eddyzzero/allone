// Bibliothèque de démonstration utilisée quand l'interface tourne dans un
// navigateur (npm run dev) au lieu de l'appli Tauri. Elle est
// indépendante de la bibliothèque de départ du Rust (src-tauri/src/library.rs).

import type { Game, Library } from "./types";

const G = "C:\\Jeux";
const HOUR = 3600;
const now = Math.floor(Date.now() / 1000);

let n = 0;
function game(
  title: string,
  systemId: string,
  kind: Game["kind"],
  path: string,
  year: number,
  genre: string,
  extra: Partial<Game> = {},
): Game {
  n += 1;
  return {
    id: `seed-${n}`,
    title,
    systemId,
    kind,
    path,
    args: [],
    cover: null,
    hero: null,
    year,
    genre,
    description: null,
    favorite: false,
    playtimeSecs: 0,
    lastPlayed: null,
    addedAt: now - n * HOUR,
    ...extra,
  };
}

export function demoLibrary(): Library {
  n = 0;
  return {
    version: 2,
    revision: 0,
    emulators: [
      { id: "pcsx2", name: "PCSX2", exePath: `${G}\\Emulateurs\\Emulateur PS2\\PCSX2\\pcsx2-qt.exe`, args: ["-nogui", "-fullscreen", "--", "{rom}"] },
      { id: "rpcs3", name: "RPCS3", exePath: `${G}\\Emulateurs\\Emulateur PS3\\RPCS3\\rpcs3.exe`, args: ["--no-gui", "{rom}"] },
      { id: "shadps4", name: "shadPS4", exePath: `${G}\\Emulateurs\\Emulateur PS4\\shadps4\\shadPS4.exe`, args: ["-g", "{rom}"] },
      { id: "ppsspp", name: "PPSSPP", exePath: `${G}\\Emulateurs\\Emulateur PSP\\PPSSPPWindows64.exe`, args: ["--fullscreen", "{rom}"] },
    ],
    systems: [
      { id: "ps2", name: "PlayStation 2", shortName: "PS2", color: "#8FD3FF", year: 2000, emulatorId: "pcsx2", romDirs: [`${G}\\2- PS2\\Games`], extensions: ["iso", "chd", "bin", "cso"], lbPlatform: "Sony Playstation 2", logo: "/systems/ps2.svg" },
      { id: "ps3", name: "PlayStation 3", shortName: "PS3", color: "#C9B6FF", year: 2006, emulatorId: "rpcs3", romDirs: [`${G}\\3 - PS3\\Games`], extensions: ["bin", "iso"], lbPlatform: "Sony Playstation 3", logo: "/systems/ps3.svg" },
      { id: "ps4", name: "PlayStation 4", shortName: "PS4", color: "#7BE0A8", year: 2013, emulatorId: "shadps4", romDirs: [`${G}\\4 - PS4\\Games`], extensions: ["bin"], lbPlatform: "Sony Playstation 4", logo: "/systems/ps4.svg" },
      { id: "psp", name: "PlayStation Portable", shortName: "PSP", color: "#F2B33D", year: 2004, emulatorId: "ppsspp", romDirs: [`${G}\\1 - PSP\\Games`], extensions: ["iso", "cso", "chd"], lbPlatform: "Sony PSP", logo: "/systems/psp.svg" },
    ],
    games: [
      game("God of War III Remastered", "ps4", "native", `${G}\\4 - PS4\\Games\\God of war III Remastered_spartanPort\\Play SpartanPort.exe`, 2015, "Action", {
        favorite: true,
        playtimeSecs: 6 * HOUR + 25 * 60,
        lastPlayed: now - 2 * HOUR,
        description:
          "Port PC natif (SpartanPort) : lancé directement, sans émulateur. Les réglages du port restent dans son propre lanceur.",
      }),
      game("God of War", "ps2", "emulated", `${G}\\2- PS2\\Games\\God of War (Europe, Australia) (En,Fr,De,Es,It).iso`, 2005, "Action", {
        playtimeSecs: 9 * HOUR + 10 * 60,
        lastPlayed: now - 26 * HOUR,
      }),
      game("God of War II", "ps2", "emulated", `${G}\\2- PS2\\Games\\God of War II (Europe, Australia) (En,Fr,De,Es,It,Ru).iso`, 2007, "Action", {
        playtimeSecs: 3 * HOUR + 5 * 60,
        lastPlayed: now - 3 * 24 * HOUR,
      }),
      game("God of War: Ascension", "ps3", "emulated", `${G}\\3 - PS3\\Games\\God of War - Ascension\\PS3_GAME\\USRDIR\\EBOOT.BIN`, 2013, "Action"),
      game("God of War: Chains of Olympus", "psp", "emulated", `${G}\\1 - PSP\\Games\\God of War - Chains of Olympus (Europe Australia) (EnFrDeEsIt).chd`, 2008, "Action", {
        playtimeSecs: 1 * HOUR + 50 * 60,
        lastPlayed: now - 8 * 24 * HOUR,
      }),
      game("God of War: Ghost of Sparta", "psp", "emulated", `${G}\\1 - PSP\\Games\\God of War - Ghost of Sparta (Europe Australia) (EnFrDeEsIt).chd`, 2010, "Action"),
      game("Devil May Cry 3", "ps2", "emulated", `${G}\\2- PS2\\Games\\Devil May Cry 3 - Dante's Awakening (Europe) (En,Fr,De,Es,It).iso`, 2005, "Action"),
      game("Need for Speed: Most Wanted", "ps2", "emulated", `${G}\\2- PS2\\Games\\Need for Speed - Most Wanted (Europe) (En,Fr,De,Es,It,Nl,Sv).iso`, 2005, "Course"),
      game("Def Jam: Fight for NY", "ps2", "emulated", `${G}\\2- PS2\\Games\\Def Jam - Fight for NY (Europe, Australia) (En,Fr).iso`, 2004, "Combat"),
    ],
  };
}
