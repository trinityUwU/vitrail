import type { ReactElement } from "react";
import { useState } from "react";
import { Plus, X } from "lucide-react";
import { Button } from "../../shared/components/Button";

const SEED_KEYLOG_APPS = ["/usr/bin/google-chrome-stable", "/usr/lib/firefox/firefox", "/usr/share/code/code"];

export function KeylogTab(): ReactElement {
  const [apps, setApps] = useState(SEED_KEYLOG_APPS);
  const [path, setPath] = useState("");

  return (
    <div className="settings-section active">
      <div style={{ fontSize: ".82rem", color: "var(--t3)", marginBottom: 16, lineHeight: 1.6 }}>
        Les applications listées ici exportent leurs clés TLS via SSLKEYLOGFILE, permettant le déchiffrement
        même quand le MITM est contourné par du pinning.
      </div>
      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <input
          className="input"
          style={{ width: 280 }}
          placeholder="Chemin de l'exécutable..."
          value={path}
          onChange={(e) => setPath(e.target.value)}
        />
        <Button
          variant="primary"
          size="sm"
          onClick={() => {
            if (!path.trim()) return;
            setApps((prev) => [...prev, path.trim()]);
            setPath("");
          }}
        >
          <Plus /> Ajouter
        </Button>
      </div>
      <ul className="excl-list">
        {apps.map((app) => (
          <li className="excl-item" key={app}>
            <span className="excl-item-name mono">{app}</span>
            <span className="excl-item-type">keylog</span>
            <Button variant="ghost" size="sm" onClick={() => setApps((prev) => prev.filter((a) => a !== app))}>
              <X />
            </Button>
          </li>
        ))}
      </ul>
    </div>
  );
}
