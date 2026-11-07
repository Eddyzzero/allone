import { useEffect, useState } from "react";
import * as api from "../api";
import AppearanceSection from "../components/AppearanceSection";
import { Folder, Plus } from "../components/Icons";
import AddSystemDialog from "../components/AddSystemDialog";
import SystemLogo from "../components/SystemLogo";
import { useStore } from "../store";
import { useUpdater } from "../updater";
import type { Emulator, Library, MetadataStatus } from "../types";

/** Champ texte qui n'enregistre qu'en quittant le champ (évite une écriture par touche). */
function LazyInput({ value, onCommit, mono, placeholder, id }: { value: string; onCommit: (v: string) => void; mono?: boolean; placeholder?: string; id?: string }) {
  const [v, setV] = useState(value);
  const [prev, setPrev] = useState(value);
  if (value !== prev) { setPrev(value); setV(value); }
  return (
    <input id={id} className="input" style={mono ? { fontFamily: "ui-monospace, Consolas, monospace", fontSize: 13 } : undefined}
      value={v} placeholder={placeholder} onChange={(e) => setV(e.target.value)}
      onBlur={() => v !== value && onCommit(v)} onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()} />
  );
}

export default function Settings() {
  const { library, update, notify, progress, bumpMedia } = useStore();
  const [status, setStatus] = useState<MetadataStatus | null>(null);
  const [confirmDel, setConfirmDel] = useState<string | null>(null);
  const [addingSystem, setAddingSystem] = useState(false);
  const { ready, checking, downloading, check } = useUpdater();
  const [appVersion, setAppVersion] = useState<string | null>(null);
  useEffect(() => {
    if (!api.inTauri) return;
    import("@tauri-apps/api/app").then((m) => m.getVersion()).then(setAppVersion).catch((e) => console.warn("Version indisponible :", e));
  }, []);
  const checkNow = async () => {
    try {
      const v = await check();
      notify(v ? `Version ${v.version} disponible` : "allone est à jour", "info");
    } catch (e) {
      console.warn("Recherche de mise à jour impossible :", e);
      notify("Impossible de contacter le serveur des mises à jour", "error");
    }
  };
  const phase = progress?.phase;
  const working = !!phase && phase !== "idle";

  // État des bibliothèques : lu à l'ouverture, puis relu quand un travail en arrière-plan commence ou finit.
  useEffect(() => {
    let cancelled = false;
    api.metadataStatus().then((s) => !cancelled && setStatus(s), () => !cancelled && setStatus(null));
    return () => { cancelled = true; };
  }, [working]);

  if (!library) return null;

  const setEmu = (id: string, patch: Partial<Emulator>) =>
    update((lib: Library) => { const e = lib.emulators.find((x) => x.id === id); if (e) Object.assign(e, patch); });

  const browseExe = async (id: string) => {
    const p = await api.pickFile({ title: "Choisir l'exécutable de l'émulateur", extensions: ["exe", "AppImage", "sh", "*"] });
    if (p) setEmu(id, { exePath: p });
  };

  /** Supprime un émulateur ; les systèmes et jeux qui l'utilisaient n'en ont plus. */
  const removeEmulator = (id: string) => {
    setConfirmDel(null);
    update((lib) => {
      lib.emulators = lib.emulators.filter((e) => e.id !== id);
      for (const s of lib.systems) if (s.emulatorId === id) s.emulatorId = null;
      for (const g of lib.games) if (g.emulatorOverride === id) g.emulatorOverride = null;
    }).then(() => notify("Émulateur supprimé"));
  };

  /** Systèmes qui utilisent cet émulateur par défaut (pour prévenir avant suppression). */
  const usedBy = (id: string) => library.systems.filter((s) => s.emulatorId === id).map((s) => s.shortName);

  /** Choisit une image du PC comme logo du système, puis l'enregistre dans la bibliothèque. */
  const changeLogo = async (systemId: string) => {
    try {
      const src = await api.pickImage("Choisir le logo de la console");
      if (!src) return;
      const logo = await api.saveLogo(systemId, src);
      if (!logo) return; // mode navigateur : pas d'enregistrement possible
      const ok = await update((lib) => { const x = lib.systems.find((y) => y.id === systemId); if (x) x.logo = logo; });
      if (!ok) return; // l'erreur est déjà signalée par update
      bumpMedia();
      notify("Logo mis à jour");
    } catch (e) {
      notify(String(e), "error");
    }
  };

  /** Supprime un système vide (sans jeu). */
  const removeSystem = (id: string) => {
    setConfirmDel(null);
    update((lib) => { lib.systems = lib.systems.filter((s) => s.id !== id); }).then(async (ok) => {
      if (!ok) return;
      await api.deleteLogo(id).catch((e) => console.warn("Suppression du logo impossible :", e));
      bumpMedia();
      notify("Console supprimée");
    });
  };

  const gameCount = (id: string) => library.games.filter((g) => g.systemId === id).length;

  const addEmulator = () =>
    update((lib) => {
      lib.emulators.push({ id: `emu-${Date.now().toString(16)}`, name: "Nouvel émulateur", exePath: "", args: ["{rom}"] });
    });

  return (
    <div className="settings">
      <header style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <span className="eyebrow" style={{ color: "var(--acc)" }}>Apparence, bibliothèques, émulateurs et systèmes</span>
        <h1 style={{ fontFamily: "var(--display)", fontSize: 48, letterSpacing: "-0.03em", lineHeight: 1 }}>Réglages</h1>
        <p style={{ margin: 0, color: "var(--muted)", maxWidth: "70ch", lineHeight: 1.55 }}>
          Connecte tes bibliothèques (Steam, images et infos des jeux), puis règle l'émulateur de chaque système.
          Dans les arguments, <code>{"{rom}"}</code> est remplacé par le chemin du jeu ; les ports PC se lancent directement depuis leur dossier.
        </p>
      </header>

      <AppearanceSection />

      <section className="section">
        <h2 className="h2">Bibliothèques</h2>
        <div className="lib-card">
          <div className="info">
            <span className="name">Steam</span>
            <span className="s">{status ? (status.steamRoot ?? "Steam non détecté") : "—"}</span>
            <span className="s">{status ? `${status.steamGames} jeux installés` : "—"}</span>
          </div>
          <div className="actions">
            <button type="button" className="btn btn-ghost" disabled={working} onClick={() => api.syncSteam().then(() => notify("Synchronisation lancée"), (e) => notify(String(e), "error"))}>Synchroniser</button>
          </div>
        </div>
        <div className="lib-card">
          <div className="info">
            <span className="name">Images et infos</span>
            <span className="s">
              {!status
                ? "—"
                : status.launchboxDate
                ? `Base LaunchBox du ${new Date(status.launchboxDate * 1000).toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" })}`
                : "Base LaunchBox pas encore téléchargée"}
            </span>
          </div>
          <div className="actions">
            <button type="button" className="btn btn-ghost" disabled={working} onClick={() => api.updateLaunchboxDb().then(() => notify("Mise à jour lancée"), (e) => notify(String(e), "error"))}>Mettre à jour la base</button>
            <button type="button" className="btn btn-ghost" disabled={working} onClick={() => api.refreshMetadata(undefined, true).then(() => notify("Mise à jour lancée"), (e) => notify(String(e), "error"))}>Mettre à jour toutes les images</button>
          </div>
        </div>
      </section>

      <section className="section">
        <div className="section-head">
          <h2 className="h2">Émulateurs</h2>
          <button type="button" className="btn btn-ghost" style={{ marginLeft: "auto" }} onClick={addEmulator}><Plus size={16} />Ajouter</button>
        </div>
        {library.emulators.map((e) => (
          <div key={e.id} className="emu-card">
            <div className="top">
              <LazyInput value={e.name} onCommit={(name) => setEmu(e.id, { name })} />
              {confirmDel === e.id ? (
                <span className="confirm-del">
                  <span className="s">
                    {usedBy(e.id).length ? `Utilisé par ${usedBy(e.id).join(", ")} : ces systèmes n'auront plus d'émulateur.` : "Supprimer cet émulateur ?"}
                  </span>
                  <button type="button" className="btn btn-ghost" onClick={() => setConfirmDel(null)}>Annuler</button>
                  <button type="button" className="btn btn-danger" onClick={() => removeEmulator(e.id)}>Supprimer</button>
                </span>
              ) : (
                <button type="button" className="btn btn-ghost" style={{ marginLeft: "auto" }} onClick={() => setConfirmDel(e.id)}>Supprimer</button>
              )}
            </div>
            <div className="two">
              <div>
                <label className="label" htmlFor={`exe-${e.id}`}>Exécutable</label>
                <div className="path-field">
                  <LazyInput id={`exe-${e.id}`} mono value={e.exePath} placeholder="C:\…\emulateur.exe" onCommit={(exePath) => setEmu(e.id, { exePath })} />
                  {api.inTauri && <button type="button" className="btn btn-ghost" style={{ height: 44 }} onClick={() => browseExe(e.id)} aria-label="Parcourir"><Folder size={16} /></button>}
                </div>
              </div>
              <div>
                <label className="label" htmlFor={`args-${e.id}`}>Arguments</label>
                <LazyInput id={`args-${e.id}`} mono value={e.args.join(" ")} onCommit={(v) => setEmu(e.id, { args: v.split(" ").filter(Boolean) })} />
              </div>
            </div>
          </div>
        ))}
      </section>

      <section className="section">
        <div className="section-head">
          <h2 className="h2">Systèmes</h2>
          <button type="button" className="btn btn-ghost" style={{ marginLeft: "auto" }} onClick={() => setAddingSystem(true)}><Plus size={16} />Ajouter une console</button>
        </div>
        {library.systems.filter((s) => s.id !== "steam").map((s) => {
          const count = gameCount(s.id);
          return (
          <div key={s.id} className="emu-card">
            <div className="top sys-top">
              <span style={{ width: 12, height: 12, borderRadius: 3, background: s.color }} />
              <SystemLogo system={s} className="sys-card-logo" />
              <span className="name">{s.name}</span>
              <span className="tag">{s.extensions.map((x) => `.${x}`).join(" ")}</span>
              <span className="tag">{s.lbPlatform ?? "Sans images automatiques"}</span>
              <span className="sys-actions">
                {api.inTauri && <button type="button" className="btn btn-ghost" onClick={() => changeLogo(s.id)}>Changer le logo</button>}
                {count > 0 ? (
                  <>
                    <span className="s">Contient {count} {count > 1 ? "jeux" : "jeu"}</span>
                    <button type="button" className="btn btn-ghost" disabled>Supprimer</button>
                  </>
                ) : confirmDel === `sys:${s.id}` ? (
                  <span className="confirm-del">
                    <span className="s">Supprimer cette console ?</span>
                    <button type="button" className="btn btn-ghost" onClick={() => setConfirmDel(null)}>Annuler</button>
                    <button type="button" className="btn btn-danger" onClick={() => removeSystem(s.id)}>Supprimer</button>
                  </span>
                ) : (
                  <button type="button" className="btn btn-ghost" onClick={() => setConfirmDel(`sys:${s.id}`)}>Supprimer</button>
                )}
              </span>
            </div>
            <div className="two">
              <div>
                <label className="label" htmlFor={`dir-${s.id}`}>Dossier des jeux</label>
                <LazyInput id={`dir-${s.id}`} mono value={s.romDirs[0] ?? ""} onCommit={(v) => update((lib) => { const x = lib.systems.find((y) => y.id === s.id); if (x) x.romDirs = v ? [v] : []; })} />
              </div>
              <div>
                <label className="label" htmlFor={`emu-${s.id}`}>Émulateur par défaut</label>
                <select id={`emu-${s.id}`} className="input" value={s.emulatorId ?? ""}
                  onChange={(ev) => update((lib) => { const x = lib.systems.find((y) => y.id === s.id); if (x) x.emulatorId = ev.target.value || null; })}>
                  <option value="">Aucun</option>
                  {library.emulators.map((e) => <option key={e.id} value={e.id}>{e.name}</option>)}
                </select>
              </div>
            </div>
          </div>
          );
        })}
      </section>
      <section className="section">
        <h2 className="h2">À propos</h2>
        <div style={{ display: "flex", alignItems: "center", gap: 16, flexWrap: "wrap" }}>
          <span>{api.inTauri ? `allone ${appVersion ?? "dev"}` : "allone · mode navigateur"}</span>
          {api.inTauri && (
            <button type="button" className="btn btn-ghost" disabled={checking || !!downloading || ready} onClick={checkNow}>
              {checking ? "Recherche…" : "Rechercher une mise à jour"}
            </button>
          )}
        </div>
      </section>
      {addingSystem && <AddSystemDialog onClose={() => setAddingSystem(false)} />}
    </div>
  );
}
