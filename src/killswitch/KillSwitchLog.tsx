import type { ReactElement } from "react";
import { useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { Button } from "../shared/components/Button";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { LogEntry } from "../shared/lib/types";

export function KillSwitchLog({ onVerify }: { onVerify: () => void }): ReactElement {
  const [entries, setEntries] = useState<LogEntry[]>([]);

  useEffect(() => {
    let cancelled = false;
    vitrailApi
      .getLogEntries()
      .then((all) => {
        if (!cancelled) setEntries(all.filter((e) => e.subsystem === "killswitch" || e.level !== "info"));
      })
      .catch((error) => logger.error({ error }, "Échec de chargement du journal d'audit"));
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="card" style={{ marginTop: 20 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <div className="section-title" style={{ marginBottom: 0 }}>Journal d'audit</div>
        <Button size="sm" onClick={onVerify}>
          <RefreshCw /> Vérifier l'état
        </Button>
      </div>
      <div className="ks-log">
        {entries.map((e, i) => (
          <div className="ks-log-entry" key={`${e.time}-${i}`}>
            <span className="ks-log-time">{e.time}</span>
            <span className="ks-log-msg">{e.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
