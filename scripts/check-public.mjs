// Usage : node scripts/check-public.mjs [dossier]
// Sans argument : fichiers suivis par git. Avec un dossier : tous ses fichiers.
import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";

const FORBIDDEN_PREFIXES = ["games Library/", ".claude/", "public/demo/", "docs/superpowers/", ".superpowers/"];
// Construites par morceaux pour que ce script ne se signale pas lui-meme.
const FORBIDDEN_STRINGS = ["Jh" + "on", "c227" + "732"];
// Identifiant legacy, tolere uniquement dans le code de migration.
const LEGACY_ID = "com.jh" + "on.allone";
const LEGACY_ALLOWED = ["src-tauri/src/legacy.rs", "src-tauri/src/lib.rs"];
// Chemins personnels (le nom d'utilisateur fictif X est tolere).
const PERSONAL_PATH = /[A-Za-z]:[\\/]+Users[\\/]+(?!X[\\/])[^\\/\s]+|\/home\/(?!user\b)[a-z0-9_.-]+\/|\/Users\/(?!X\/)[A-Za-z0-9_.-]+\//i;
const STRING_EXEMPT_PREFIXES = ["docs/superpowers/"];
const BINARY_EXT = /\.(png|jpe?g|gif|webp|ico|icns|woff2?|ttf|zip|gz|exe|dll|pdf)$/i;

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    if (name === ".git" || name === "node_modules") continue;
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p, out);
    else out.push(p);
  }
  return out;
}

const dirArg = process.argv[2];
let base;
let files;
if (dirArg) {
  base = dirArg;
  files = walk(dirArg).map((p) => relative(dirArg, p).split(sep).join("/"));
} else {
  base = ".";
  files = execFileSync("git", ["ls-files", "-z"], { encoding: "utf8", maxBuffer: 1 << 28 })
    .split("\0")
    .filter(Boolean);
}

const problems = [];
for (const f of files) {
  if (FORBIDDEN_PREFIXES.some((p) => f.startsWith(p)) || /\.key$/i.test(f)) {
    problems.push(`fichier interdit : ${f}`);
  }
  if (BINARY_EXT.test(f) || STRING_EXEMPT_PREFIXES.some((p) => f.startsWith(p))) continue;
  let text;
  try {
    text = readFileSync(join(base, f), "utf8");
  } catch (e) {
    console.error(`illisible, ignore : ${f} (${e.code ?? e.message})`);
    continue;
  }
  let lower = text.toLowerCase();
  if (LEGACY_ALLOWED.includes(f)) lower = lower.split(LEGACY_ID).join("");
  for (const needle of FORBIDDEN_STRINGS) {
    if (lower.includes(needle.toLowerCase())) problems.push(`chaine interdite "${needle}" dans ${f}`);
  }
  const m = PERSONAL_PATH.exec(text);
  if (m) problems.push(`chemin personnel "${m[0]}" dans ${f}`);
}

if (problems.length) {
  console.error(problems.join("\n"));
  console.error(`\n${problems.length} probleme(s).`);
  process.exit(1);
}
console.log(`OK : ${files.length} fichiers verifies, rien d'interdit.`);
