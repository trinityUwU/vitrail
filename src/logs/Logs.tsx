import type { ReactElement } from "react";
import { useMemo, useState } from "react";
import { Clipboard, Download, Trash2 } from "lucide-react";
import { Button } from "../shared/components/Button";
import { useToast } from "../shared/hooks/useToast";
import { logger } from "../shared/lib/logger";
import { copyLogEntries, downloadLogEntries } from "./log-actions";
import { useLogs } from "./useLogs";

const SUBSYSTEMS = ["attribution", "capture", "decryption", "keylog", "killswitch"];
const LEVELS = ["info", "warn", "error"];

const LEVEL_STYLE: Record<string, string> = {
  error: "color:var(--danger);font-weight:600",
  warn: "color:var(--warn);font-weight:500",
  info: "color:var(--t2)",
};

export function Logs(): ReactElement {
  const { entries, purge } = useLogs();
  const [subFilter, setSubFilter] = useState("");
  const [levelFilter, setLevelFilter] = useState("");
  const { showToast } = useToast();

  const filtered = useMemo(
    () =>
      entries.filter(
        (e) => (!subFilter || e.subsystem === subFilter) && (!levelFilter || e.level === levelFilter),
      ),
    [entries, subFilter, levelFilter],
  );

  const handleCopy = async (): Promise<void> => {
    try {
      await copyLogEntries(filtered);
      showToast("Copié dans le presse-papiers");
    } catch (error) {
      logger.error({ error }, "Échec de copie du journal");
    }
  };

  const handleExport = (): void => {
    try {
      downloadLogEntries(filtered);
      showToast("Export lancé");
    } catch (error) {
      logger.error({ error }, "Échec d'export du journal");
    }
  };

  const handlePurge = async (): Promise<void> => {
    await purge();
    showToast("Action de purge effectuée");
  };

  return (
    <div>
      <div className="screen-title">Journal système</div>
      <div className="screen-subtitle">Logs structurés de chaque sous-système</div>
      <div style={{ display: "flex", gap: 8, marginBottom: 16, alignItems: "center" }}>
        <select className="input select" style={{ width: 160 }} value={subFilter} onChange={(e) => setSubFilter(e.target.value)}>
          <option value="">Tous les sous-systèmes</option>
          {SUBSYSTEMS.map((s) => <option key={s}>{s}</option>)}
        </select>
        <select className="input select" style={{ width: 130 }} value={levelFilter} onChange={(e) => setLevelFilter(e.target.value)}>
          <option value="">Tous niveaux</option>
          {LEVELS.map((l) => <option key={l}>{l}</option>)}
        </select>
        <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
          <Button size="sm" onClick={() => void handleCopy()}><Clipboard /> Copier</Button>
          <Button size="sm" onClick={handleExport}><Download /> Exporter</Button>
          <Button variant="danger" size="sm" onClick={() => void handlePurge()}><Trash2 /> Purger</Button>
        </div>
      </div>
      <div className="ks-log" style={{ maxHeight: "calc(100vh - 260px)" }}>
        {filtered.map((e, i) => (
          <div className="ks-log-entry" key={`${e.time}-${i}`}>
            <span className="ks-log-time">{e.time}</span>
            <span style={{ fontSize: ".7rem", color: "var(--t3)", width: 90, flexShrink: 0 }}>{e.subsystem}</span>
            <span style={{ fontSize: ".7rem", width: 40, flexShrink: 0, ...styleFromString(LEVEL_STYLE[e.level]) }}>
              {e.level.toUpperCase()}
            </span>
            <span className="ks-log-msg">{e.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function styleFromString(css: string): Record<string, string> {
  const entries = css.split(";").filter(Boolean).map((rule) => {
    const [key, value] = rule.split(":");
    const camel = key.replace(/-([a-z])/g, (_, c: string) => c.toUpperCase());
    return [camel, value] as const;
  });
  return Object.fromEntries(entries);
}
