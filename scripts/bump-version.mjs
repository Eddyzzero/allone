// Usage : node scripts/bump-version.mjs <x.y.z>
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error("Usage: node scripts/bump-version.mjs <x.y.z>");
  process.exit(1);
}

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const edit = (rel, fn) => {
  const path = join(root, rel);
  if (!existsSync(path)) return;
  const before = readFileSync(path, "utf8");
  const after = fn(before);
  if (after === before) console.log(`${rel}: deja a ${version}`);
  else {
    writeFileSync(path, after);
    console.log(`${rel} -> ${version}`);
  }
};

const jsonVersion = (s) => s.replace(/("version"\s*:\s*")[^"]*(")/, `$1${version}$2`);
edit("package.json", jsonVersion);
edit("src-tauri/tauri.conf.json", jsonVersion);
// Cargo.toml : uniquement la ligne `version` de la section [package].
edit("src-tauri/Cargo.toml", (s) =>
  s.replace(/(\[package\][^\[]*?^version\s*=\s*")[^"]*(")/m, `$1${version}$2`));
// Cargo.lock : uniquement l'entree du paquet `allone`.
edit("src-tauri/Cargo.lock", (s) =>
  s.replace(/(\[\[package\]\]\r?\nname = "allone"\r?\nversion = ")[^"]*(")/, `$1${version}$2`));
// package-lock.json : version racine (haut du fichier et packages[""]).
edit("package-lock.json", (s) => {
  const lock = JSON.parse(s);
  lock.version = version;
  if (lock.packages?.[""]) lock.packages[""].version = version;
  const eol = s.includes("\r\n") ? "\r\n" : "\n";
  return JSON.stringify(lock, null, 2).replace(/\n/g, eol) + (s.endsWith("\n") ? eol : "");
});
