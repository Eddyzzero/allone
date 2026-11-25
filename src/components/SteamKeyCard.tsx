import { useEffect, useState } from "react";
import * as api from "../api";
import { useStore } from "../store";
import type { SteamAccountStatus } from "../types";

/** Paramètres → Steam : clé d'API pour lister aussi les jeux du compte qui ne sont pas installés. */
export default function SteamKeyCard({ steamInstalled }: { steamInstalled: boolean }) {
  const { notify } = useStore();
  const [account, setAccount] = useState<SteamAccountStatus | null>(null);
  const [key, setKey] = useState("");
  const [checking, setChecking] = useState(false);

  const reload = () => api.steamAccountStatus().then(setAccount, () => setAccount(null));
  useEffect(() => {
    reload();
  }, []);

  const save = async () => {
    setChecking(true);
    try {
      const count = await api.setSteamApiKey(key);
      setKey("");
      notify(`Clé enregistrée : ${count} jeux trouvés.`);
      await reload();
    } catch (e) {
      notify(String(e), "error");
    } finally {
      setChecking(false);
    }
  };

  const clear = async () => {
    try {
      await api.clearSteamApiKey();
      notify("Clé retirée. Les jeux non installés restent dans la bibliothèque.");
      await reload();
    } catch (e) {
      notify(String(e), "error");
    }
  };

  return (
    <div className="lib-card">
      <div className="info">
        <span className="name">Jeux non installés</span>
        {!steamInstalled ? (
          <span className="s">Steam n'est pas installé sur ce PC.</span>
        ) : (
          <>
            <span className="s">
              {account?.accountName ? `Compte Steam : ${account.accountName}` : "Aucun compte Steam trouvé sur ce PC."}
            </span>
            <span className="s">
              {account?.hasKey
                ? "Clé d'API enregistrée : tous les jeux de ton compte apparaissent, installés ou non."
                : "Ajoute ta clé d'API Steam pour voir aussi les jeux de ton compte qui ne sont pas installés."}
            </span>
          </>
        )}
      </div>
      {steamInstalled && (
        <div className="actions">
          {account?.hasKey ? (
            <button type="button" className="btn btn-ghost" onClick={clear}>Retirer la clé</button>
          ) : (
            <>
              <input
                className="input"
                type="password"
                autoComplete="off"
                spellCheck={false}
                placeholder="Clé d'API Steam"
                aria-label="Clé d'API Steam"
                value={key}
                onChange={(e) => setKey(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && !checking && key.trim() && save()}
                style={{ width: 260 }}
              />
              <button type="button" className="btn btn-ghost" disabled={checking || !key.trim()} onClick={save}>
                {checking ? "Vérification…" : "Enregistrer"}
              </button>
              <button type="button" className="btn btn-ghost" onClick={() => api.openSteamKeyPage().catch((e) => notify(String(e), "error"))}>
                Créer une clé
              </button>
            </>
          )}
        </div>
      )}
    </div>
  );
}
