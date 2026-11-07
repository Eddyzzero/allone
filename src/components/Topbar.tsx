import { useState, type FormEvent } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { Plus, Search, Sliders, User } from "./Icons";

export default function Topbar({ onAdd }: { onAdd: () => void }) {
  const [params] = useSearchParams();
  const [q, setQ] = useState(params.get("q") ?? "");
  const navigate = useNavigate();

  function submit(e: FormEvent) {
    e.preventDefault();
    navigate(q.trim() ? `/library?q=${encodeURIComponent(q.trim())}` : "/library");
  }

  return (
    <header className="topbar">
      <form role="search" className="search" onSubmit={submit}>
        <label htmlFor="q" style={{ display: "flex" }}>
          <Search />
          <span className="sr-only">Rechercher</span>
        </label>
        <input id="q" type="search" placeholder="Rechercher un jeu, un système…" value={q} onChange={(e) => setQ(e.target.value)} />
        <button type="button" className="icon-btn" style={{ width: 38, height: 38, border: 0, background: "rgba(255,255,255,0.07)", borderRadius: 10 }}
          aria-label="Filtres" onClick={() => navigate("/library")}>
          <Sliders size={18} />
        </button>
      </form>
      <div className="top-right">
        <button type="button" className="icon-btn" aria-label="Ajouter un jeu" onClick={onAdd}><Plus /></button>
        <span className="profile">
          <span className="avatar"><User /></span>
          <span className="n">Profil local</span>
        </span>
      </div>
    </header>
  );
}
