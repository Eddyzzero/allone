import { useState } from "react";
import { HashRouter, Route, Routes, useLocation, useParams } from "react-router-dom";
import AddGameDialog from "./components/AddGameDialog";
import AppBackground from "./components/AppBackground";
import Sidebar from "./components/Sidebar";
import Detail from "./pages/Detail";
import Games from "./pages/Games";
import Home from "./pages/Home";
import Settings from "./pages/Settings";
import { LibraryProvider, useStore } from "./store";
import { ThemeProvider } from "./theme";
import { UpdaterProvider } from "./updater";

function Toasts() {
  const { toasts } = useStore();
  return (
    <div className="toasts" role="status" aria-live="polite">
      {toasts.map((t) => (
        <div key={t.id} className={`toast ${t.tone}`}><span className="led" />{t.text}</div>
      ))}
    </div>
  );
}

function SystemRoute({ onAdd }: { onAdd: () => void }) {
  const { id } = useParams();
  return <Games key={`system-${id}`} mode="system" onAdd={onAdd} />;
}

function Shell() {
  const [adding, setAdding] = useState(false);
  const { pathname, search } = useLocation();
  const openAdd = () => setAdding(true);

  return (
    <div className="shell">
      <Sidebar />
      {/* key : on remonte en haut de page à chaque changement d'écran */}
      <main key={pathname + search}>
        <Routes>
          <Route path="/" element={<Home onAdd={openAdd} />} />
          <Route path="/library" element={<Games key="library" mode="library" onAdd={openAdd} />} />
          <Route path="/favorites" element={<Games key="favorites" mode="favorites" onAdd={openAdd} />} />
          <Route path="/system/:id" element={<SystemRoute onAdd={openAdd} />} />
          <Route path="/game/:id" element={<Detail />} />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </main>
      {adding && <AddGameDialog onClose={() => setAdding(false)} />}
      <Toasts />
    </div>
  );
}

export default function App() {
  return (
    <ThemeProvider>
      <AppBackground />
      <LibraryProvider>
        <UpdaterProvider>
          <HashRouter>
            <Shell />
          </HashRouter>
        </UpdaterProvider>
      </LibraryProvider>
    </ThemeProvider>
  );
}
