import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import * as api from "../api";
import { useStore } from "../store";
import { EXTENSIONS, SYSTEM_COLORS, shortNameFor, uniqueSystemId } from "../systemPresets";
import type { Emulator, LbPlatform, LogoCandidate, System } from "../types";
import { Check, Close, Folder } from "./Icons";

type Step = "choose" | "configure" | "logo" | "done";

const NEW_EMU = "__new__";

const parseExts = (v: string) =>
  [...new Set(v.split(/[\s,;]+/).map((x) => x.replace(/^[*.]+/, "").toLowerCase()).filter(Boolean))];

export default function AddSystemDialog({ onClose }: { onClose: () => void }) {
  const { library, update, notify, bumpMedia, progress } = useStore();
  const navigate = useNavigate();
  const [step, setStep] = useState<Step>("choose");
  const [platforms, setPlatforms] = useState<LbPlatform[] | null>(null);
  const [query, setQuery] = useState("");
  const [lbPlatform, setLbPlatform] = useState<string | null>(null);
  const [year, setYear] = useState<number | null>(null);
  const [name, setName] = useState("");
  const [shortName, setShortName] = useState("");
  const [color, setColor] = useState(SYSTEM_COLORS[0]);
  const [emuChoice, setEmuChoice] = useState("");
  const [emuName, setEmuName] = useState("");
  const [emuExe, setEmuExe] = useState("");
  const [emuArgs, setEmuArgs] = useState("{rom}");
  const [romDir, setRomDir] = useState("");
  const [exts, setExts] = useState("");
  const [logos, setLogos] = useState<LogoCandidate[] | null>(null);
  const [logo, setLogo] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [createdId, setCreatedId] = useState("");
  const [createdShort, setCreatedShort] = useState("");
  const dialogRef = useRef<HTMLDivElement>(null);
  const logoReq = useRef(0);
  const dbWait = useRef<"no" | "waiting" | "running">("no");
  const phase = progress?.phase;

  // Échap pour fermer, focus dans la fenêtre à l'ouverture.
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    dialogRef.current?.focus();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && closeRef.current();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    let off = false;
    api.listLbPlatforms().then((l) => !off && setPlatforms(l), () => !off && setPlatforms([]));
    return () => { off = true; };
  }, []);

  // Après « Télécharger la base » : recharge le catalogue quand le travail de fond se termine.
  useEffect(() => {
    const idle = !phase || phase === "idle";
    // On n'arme le rechargement qu'une fois la phase de la base LaunchBox vue (pas une synchro Steam).
    if (dbWait.current === "waiting" && (phase === "launchbox-download" || phase === "launchbox-index")) dbWait.current = "running";
    else if (dbWait.current === "running" && idle) {
      dbWait.current = "no";
      api.listLbPlatforms().then(setPlatforms, () => {});
    }
  }, [phase]);

  const emulators = library?.emulators ?? [];
  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (platforms ?? []).filter((p) => !q || p.name.toLowerCase().includes(q) || (p.manufacturer ?? "").toLowerCase().includes(q));
  }, [platforms, query]);

  function resetLogos() {
    logoReq.current++;
    setLogos(null);
    setLogo(null);
  }

  function pick(p: LbPlatform | null) {
    const n = p?.name ?? "";
    setLbPlatform(p?.name ?? null);
    setYear(p?.year ?? null);
    setName(n);
    setShortName(n ? shortNameFor(n) : "");
    setExts((EXTENSIONS[n] ?? []).join(" "));
    resetLogos();
    setStep("configure");
  }

  async function downloadDb() {
    setBusy(true);
    try {
      dbWait.current = "waiting";
      await api.updateLaunchboxDb();
      notify("Téléchargement de la base lancé");
    } catch (e) {
      dbWait.current = "no";
      notify(String(e), "error");
    } finally {
      setBusy(false);
    }
  }

  async function browseExe() {
    const p = await api.pickFile({ title: "Choisir l'exécutable de l'émulateur", extensions: ["exe", "AppImage", "sh", "*"] });
    if (p) setEmuExe(p);
  }

  async function browseDir() {
    const p = await api.pickFolder("Dossier des jeux");
    if (p) setRomDir(p);
  }

  function toLogo() {
    setStep("logo");
    if (logos === null) {
      const req = ++logoReq.current;
      api.searchLogos(name.trim()).then(
        (l) => req === logoReq.current && setLogos(l),
        () => req === logoReq.current && setLogos([]),
      );
    }
  }

  async function pickOwnLogo() {
    const p = await api.pickImage("Choisir un logo");
    if (p) setLogo(p);
  }

  async function create(withLogo: string | null) {
    if (!library) return;
    setBusy(true);
    try {
      let emulatorId: string | null = emuChoice || null;
      let emu: Emulator | null = null;
      if (emuChoice === NEW_EMU) {
        emu = { id: `emu-${Date.now().toString(16)}`, name: emuName.trim() || "Nouvel émulateur", exePath: emuExe.trim(), args: emuArgs.split(" ").filter(Boolean) };
        emulatorId = emu.id;
      }
      const shortOut = shortName.trim() || name.trim();
      // Identifiant calculé d'avance : le logo est enregistré sous cet id avant la bibliothèque,
      // puis tout part en une seule sauvegarde.
      const plannedId = uniqueSystemId(shortOut, library.systems.map((s) => s.id));
      let logoPath: string | null = null;
      if (withLogo) {
        try {
          logoPath = await api.saveLogo(plannedId, withLogo);
          if (!logoPath) notify("Le logo n'a pas été enregistré ; la console sera ajoutée sans logo.", "error");
        } catch (e) {
          notify(`Le logo n'a pas pu être enregistré (${String(e)}) ; la console sera ajoutée sans logo.`, "error");
        }
      }
      let id = plannedId;
      let logoLost = false;
      const saved = await update((l) => {
        // Bibliothèque fraîche : l'identifiant a pu être pris entre-temps.
        id = uniqueSystemId(shortOut, l.systems.map((s) => s.id));
        logoLost = !!logoPath && id !== plannedId;
        const sys: System = {
          id, name: name.trim(), shortName: shortOut, color, year,
          emulatorId, romDirs: romDir.trim() ? [romDir.trim()] : [], extensions: parseExts(exts),
          lbPlatform, logo: logoLost ? null : logoPath,
        };
        if (emu) l.emulators.push(emu);
        // Avant Steam, pour garder Steam en dernier dans l'accueil et la barre latérale.
        const at = l.systems.findIndex((x) => x.id === "steam");
        if (at >= 0) l.systems.splice(at, 0, sys);
        else l.systems.push(sys);
      });
      if (!saved) {
        if (logoPath) api.deleteLogo(plannedId).catch(() => {});
        return; // l'erreur est déjà signalée ; on reste sur l'étape du logo
      }
      if (logoPath) {
        if (logoLost) notify("Console ajoutée, mais son identifiant a changé : logo à rechoisir dans les réglages.", "error");
        else bumpMedia();
      }
      setCreatedId(id);
      setCreatedShort(shortOut);
      setStep("done");
    } catch (e) {
      notify(String(e), "error");
    } finally {
      setBusy(false);
    }
  }

  const canConfigure = name.trim().length > 0 && shortName.trim().length > 0 && (emuChoice !== NEW_EMU || (emuName.trim().length > 0 && emuExe.trim().length > 0)) && parseExts(exts).length > 0;

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="dialog" role="dialog" aria-modal="true" aria-labelledby="addsys-title" tabIndex={-1} ref={dialogRef}>
        <header>
          <h2 id="addsys-title">{step === "done" ? "Console ajoutée" : "Ajouter une console"}</h2>
          <button type="button" className="x" aria-label="Fermer" onClick={onClose}><Close /></button>
        </header>

        {step === "choose" && (
          <div className="content">
            <input className="input" placeholder="Rechercher une console…" aria-label="Rechercher une console" value={query} onChange={(e) => setQuery(e.target.value)} />
            {platforms !== null && platforms.length === 0 ? (
              <div className="empty" style={{ padding: 24 }}>
                <p style={{ margin: "0 0 12px" }}>La base LaunchBox n'est pas encore téléchargée.</p>
                {api.inTauri && <button type="button" className="btn btn-ghost" disabled={busy} onClick={downloadDb}>Télécharger la base</button>}
              </div>
            ) : (
              <div className="found">
                {platforms === null && <p className="s" style={{ margin: 0 }}>Chargement…</p>}
                {filtered.map((p) => (
                  <button key={p.name} type="button" className="plat-row" onClick={() => pick(p)}>
                    <span className="t">{p.name}</span>
                    <span className="s">{[p.year, p.manufacturer, p.category].filter(Boolean).join(" · ")}</span>
                  </button>
                ))}
                {platforms !== null && filtered.length === 0 && <p className="s" style={{ margin: 0 }}>Aucune console ne correspond.</p>}
              </div>
            )}
            <button type="button" className="btn btn-ghost" style={{ alignSelf: "flex-start" }} onClick={() => pick(null)}>
              Autre console (sans images automatiques)
            </button>
          </div>
        )}

        {step === "configure" && (
          <>
            <div className="content">
              <div className="two">
                <div>
                  <label className="label" htmlFor="s-name">Nom</label>
                  <input id="s-name" className="input" value={name} onChange={(e) => { setName(e.target.value); resetLogos(); }} />
                </div>
                <div>
                  <label className="label" htmlFor="s-short">Nom court</label>
                  <input id="s-short" className="input" value={shortName} onChange={(e) => setShortName(e.target.value)} />
                </div>
              </div>

              <div>
                <span className="label">Couleur</span>
                <div className="swatches" role="radiogroup" aria-label="Couleur">
                  {SYSTEM_COLORS.map((c) => (
                    <button key={c} type="button" role="radio" aria-checked={color === c} aria-label={c} className={`swatch${color === c ? " on" : ""}`} style={{ background: c }} onClick={() => setColor(c)} />
                  ))}
                  <input type="color" aria-label="Autre couleur" value={/^#[0-9a-f]{6}$/i.test(color) ? color : "#888888"} onChange={(e) => setColor(e.target.value)} />
                </div>
              </div>

              <div>
                <label className="label" htmlFor="s-emu">Émulateur</label>
                <select id="s-emu" className="input" value={emuChoice} onChange={(e) => setEmuChoice(e.target.value)}>
                  <option value="">Aucun</option>
                  {emulators.map((e) => <option key={e.id} value={e.id}>{e.name}</option>)}
                  <option value={NEW_EMU}>Nouvel émulateur…</option>
                </select>
              </div>

              {emuChoice === NEW_EMU && (
                <div className="emu-card" style={{ margin: 0 }}>
                  <div className="two">
                    <div>
                      <label className="label" htmlFor="s-emu-name">Nom de l'émulateur</label>
                      <input id="s-emu-name" className="input" value={emuName} onChange={(e) => setEmuName(e.target.value)} />
                    </div>
                    <div>
                      <label className="label" htmlFor="s-emu-args">Arguments</label>
                      <input id="s-emu-args" className="input" value={emuArgs} onChange={(e) => setEmuArgs(e.target.value)} />
                    </div>
                  </div>
                  <label className="label" htmlFor="s-emu-exe">Exécutable</label>
                  <div className="path-field">
                    <input id="s-emu-exe" className="input" placeholder="C:\…\emulateur.exe" value={emuExe} onChange={(e) => setEmuExe(e.target.value)} />
                    {api.inTauri && <button type="button" className="btn btn-ghost" style={{ height: 44 }} onClick={browseExe} aria-label="Parcourir"><Folder size={16} /></button>}
                  </div>
                </div>
              )}

              <div>
                <label className="label" htmlFor="s-dir">Dossier des jeux</label>
                <div className="path-field">
                  <input id="s-dir" className="input" placeholder="D:\Jeux\…" value={romDir} onChange={(e) => setRomDir(e.target.value)} />
                  {api.inTauri && <button type="button" className="btn btn-ghost" style={{ height: 44 }} onClick={browseDir} aria-label="Parcourir"><Folder size={16} /></button>}
                </div>
              </div>

              <div>
                <label className="label" htmlFor="s-exts">Extensions</label>
                <input id="s-exts" className="input" placeholder="iso rvz gcm" value={exts} onChange={(e) => setExts(e.target.value)} />
                <p className="s" style={{ margin: "6px 0 0", fontSize: 12, color: "var(--faint)" }}>Au moins une extension, ex. iso</p>
              </div>
            </div>
            <footer>
              <button type="button" className="btn btn-ghost" style={{ marginRight: "auto" }} onClick={() => setStep("choose")}>Retour</button>
              <button type="button" className="btn btn-acc" style={{ height: 40 }} disabled={!canConfigure} onClick={toLogo}>Suivant</button>
            </footer>
          </>
        )}

        {step === "logo" && (
          <>
            <div className="content">
              {logos === null ? (
                <p className="s" style={{ margin: 0 }}>Recherche de logos…</p>
              ) : logos.length === 0 ? (
                <div className="empty" style={{ padding: 24 }}>Aucun logo trouvé</div>
              ) : (
                <div className="logo-grid">
                  {logos.map((c) => (
                    <button key={c.url} type="button" className={`logo-cell${logo === c.url ? " on" : ""}`} aria-pressed={logo === c.url} title={c.title} onClick={() => setLogo(c.url)}>
                      <img src={c.thumbUrl} alt={c.title} loading="lazy" />
                    </button>
                  ))}
                </div>
              )}
              {logo && <p className="s" style={{ margin: 0, wordBreak: "break-all" }}>Logo choisi : {logo}</p>}
              {api.inTauri && <button type="button" className="btn btn-ghost" style={{ alignSelf: "flex-start" }} onClick={pickOwnLogo}>Choisir une image sur mon PC</button>}
            </div>
            <footer>
              <button type="button" className="btn btn-ghost" style={{ marginRight: "auto" }} onClick={() => setStep("configure")}>Retour</button>
              <button type="button" className="btn btn-ghost" disabled={busy} onClick={() => create(null)}>Sans logo</button>
              <button type="button" className="btn btn-acc" style={{ height: 40 }} disabled={busy || !logo} onClick={() => create(logo)}>Ajouter la console</button>
            </footer>
          </>
        )}

        {step === "done" && (
          <div className="content">
            <div className="done">
              <span className="ico"><Check size={28} /></span>
              <p className="big">{name.trim()} est prête</p>
              <p className="small">Ajoutée aux systèmes · {createdShort}</p>
              <div style={{ display: "flex", gap: 10, marginTop: 14 }}>
                <button type="button" className="btn btn-ghost" onClick={() => { onClose(); navigate(`/system/${createdId}`); notify(`Utilise « + » puis « Analyser ${createdShort} » pour ajouter ses jeux.`); }}>Analyser ses jeux</button>
                <button type="button" className="btn btn-acc" style={{ height: 40 }} onClick={() => { onClose(); navigate(`/system/${createdId}`); }}>Voir la console</button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
