import { useState } from "react";
import { useTheme, type Theme } from "../theme";
import { Close, Plus } from "./Icons";

type ColorKey = "ground" | "acc" | "onAcc" | "acc2" | "text" | "muted";
const COLOR_FIELDS: { key: ColorKey; label: string }[] = [
  { key: "ground", label: "Fond" },
  { key: "acc", label: "Accent" },
  { key: "onAcc", label: "Texte sur l'accent" },
  { key: "acc2", label: "Accent 2" },
  { key: "text", label: "Texte" },
  { key: "muted", label: "Texte secondaire" },
];

function preview(t: Theme) {
  return `linear-gradient(135deg, ${t.gradient.join(", ")})`;
}

/** Réglages → Apparence : choix du thème, dégradé animé, thèmes perso. */
export default function AppearanceSection() {
  const { themes, theme, animate, hidden, setTheme, setAnimate, saveCustom, removeTheme, restoreBuiltins } = useTheme();
  const [editingId, setEditingId] = useState<string | null>(null);
  const editing = themes.find((t) => t.id === editingId && !t.builtin) ?? null;

  const createTheme = () => {
    const t: Theme = { ...theme, id: `perso-${Date.now().toString(16)}`, name: `${theme.name} perso`, builtin: false, gradient: [...theme.gradient] };
    saveCustom(t);
    setTheme(t.id);
    setEditingId(t.id);
  };

  const patch = (p: Partial<Theme>) => editing && saveCustom({ ...editing, ...p });
  const setStop = (i: number, c: string) => editing && patch({ gradient: editing.gradient.map((x, j) => (j === i ? c : x)) });

  return (
    <section className="section">
      <div className="section-head">
        <h2 className="h2">Apparence</h2>
        <button type="button" className="btn btn-ghost" style={{ marginLeft: "auto" }} onClick={createTheme}>
          <Plus size={16} />Créer un thème
        </button>
      </div>

      <div className="theme-grid" role="radiogroup" aria-label="Thème">
        {themes.map((t) => (
          <div key={t.id} className="theme-item">
            <button type="button" role="radio" aria-checked={t.id === theme.id} className="theme-card"
              onClick={() => { setTheme(t.id); setEditingId(t.builtin ? null : t.id); }}>
              <span className="swatch" style={{ background: preview(t) }}>
                <span className="dot" style={{ background: t.acc }} />
                <span className="dot" style={{ background: t.acc2 }} />
              </span>
              <span className="n">{t.name}</span>
            </button>
            {themes.length > 1 && (
              <button type="button" className="theme-del" aria-label={`Supprimer le thème ${t.name}`} title="Supprimer"
                onClick={() => { removeTheme(t.id); if (editingId === t.id) setEditingId(null); }}>
                <Close size={14} />
              </button>
            )}
          </div>
        ))}
      </div>
      {hidden.length > 0 && (
        <button type="button" className="link-acc" style={{ marginLeft: 0, alignSelf: "flex-start" }} onClick={restoreBuiltins}>
          Restaurer les thèmes d'origine
        </button>
      )}

      <label className="check">
        <input type="checkbox" checked={animate} onChange={(e) => setAnimate(e.target.checked)} />
        Animer le dégradé du fond
      </label>

      {editing && (
        <div className="emu-card theme-editor">
          <div className="top">
            <input className="input" aria-label="Nom du thème" value={editing.name} onChange={(e) => patch({ name: e.target.value })} />
            {themes.length > 1 && (
              <button type="button" className="btn btn-ghost" onClick={() => { removeTheme(editing.id); setEditingId(null); }}>Supprimer</button>
            )}
          </div>
          <div className="color-grid">
            {COLOR_FIELDS.map((f) => (
              <label key={f.key} className="color-field">
                <input type="color" value={editing[f.key]} onChange={(e) => patch({ [f.key]: e.target.value } as Partial<Theme>)} />
                <span>{f.label}</span>
              </label>
            ))}
          </div>
          <div>
            <span className="label">Dégradé du fond</span>
            <div className="color-grid">
              {editing.gradient.map((c, i) => (
                <span key={i} className="color-field">
                  <input type="color" aria-label={`Couleur ${i + 1} du dégradé`} value={c} onChange={(e) => setStop(i, e.target.value)} />
                  {editing.gradient.length > 2 && (
                    <button type="button" className="btn btn-ghost btn-sm" onClick={() => patch({ gradient: editing.gradient.filter((_, j) => j !== i) })}>Retirer</button>
                  )}
                </span>
              ))}
              {editing.gradient.length < 6 && (
                <button type="button" className="btn btn-ghost btn-sm" onClick={() => patch({ gradient: [...editing.gradient, editing.acc] })}>
                  <Plus size={14} />Couleur
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
