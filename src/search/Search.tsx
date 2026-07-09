import type { ReactElement } from "react";
import { useState } from "react";
import { Bell, BellPlus, Bookmark, Check, Search as SearchIcon, Trash2, Upload } from "lucide-react";
import { Button } from "../shared/components/Button";
import { TableWrap } from "../shared/components/Table";
import { VisibilityBadge } from "../shared/components/VisibilityBadge";
import { EmptyState } from "../shared/components/EmptyState";
import { VISIBILITY_OPTIONS } from "../shared/lib/visibility";
import { useToast } from "../shared/hooks/useToast";
import { logger } from "../shared/lib/logger";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { EMPTY_QUERY, useSearch } from "./useSearch";
import { useSavedQueries } from "./useSavedQueries";
import { criteriaToQuery, queryToCriteria } from "./search-utils";

export function Search({ onSelectFlow }: { onSelectFlow: (id: string) => void }): ReactElement {
  const [query, setQuery] = useState(EMPTY_QUERY);
  const { results, run } = useSearch();
  const { savedQueries, save, remove } = useSavedQueries();
  const { showToast } = useToast();
  const [namingSave, setNamingSave] = useState(false);
  const [namingAlert, setNamingAlert] = useState(false);
  const [nameInput, setNameInput] = useState("");

  const handleSave = async (): Promise<void> => {
    if (!nameInput.trim()) return;
    await save(nameInput.trim(), queryToCriteria(query));
    showToast("Requête sauvegardée");
    setNameInput("");
    setNamingSave(false);
  };

  const handleConvertToAlert = async (): Promise<void> => {
    if (!nameInput.trim()) return;
    try {
      const saved = await save(`Recherche : ${nameInput.trim()}`, queryToCriteria(query));
      if (!saved) return;
      await vitrailApi.convertQueryToAlert(saved.id, nameInput.trim());
      showToast("Règle d'alerte créée depuis la recherche");
    } catch (error) {
      logger.error({ error }, "Échec de conversion de la requête en alerte");
    }
    setNameInput("");
    setNamingAlert(false);
  };

  return (
    <div>
      <div className="screen-title">Recherche avancée</div>
      <div className="screen-subtitle">Requêtage libre sur l'historique stocké</div>
      <div className="card" style={{ marginBottom: 20 }}>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 14 }}>
          <div>
            <label className="field-label">Processus</label>
            <input className="input" placeholder="Nom ou chemin..." value={query.process}
              onChange={(e) => setQuery({ ...query, process: e.target.value })} />
          </div>
          <div>
            <label className="field-label">Domaine / IP</label>
            <input className="input" placeholder="Ex: google.com" value={query.destination}
              onChange={(e) => setQuery({ ...query, destination: e.target.value })} />
          </div>
          <div>
            <label className="field-label">Port</label>
            <input className="input" placeholder="443" value={query.port}
              onChange={(e) => setQuery({ ...query, port: e.target.value })} />
          </div>
          <div>
            <label className="field-label">Niveau de visibilité</label>
            <select className="input select" value={query.visibility}
              onChange={(e) => setQuery({ ...query, visibility: e.target.value })}>
              <option value="">Tous</option>
              {VISIBILITY_OPTIONS.map((v) => <option key={v.key} value={v.key}>{v.label}</option>)}
            </select>
          </div>
        </div>
        <div style={{ marginTop: 14 }}>
          <label className="field-label">Recherche plein texte (contenu déchiffré)</label>
          <input className="input" placeholder="Ex: password, token, api_key..." value={query.text}
            onChange={(e) => setQuery({ ...query, text: e.target.value })} />
        </div>
        <div style={{ marginTop: 16, display: "flex", gap: 8 }}>
          <Button variant="primary" onClick={() => void run(query)}>
            <SearchIcon /> Rechercher
          </Button>
          <Button onClick={() => { setNamingSave((v) => !v); setNamingAlert(false); }}>
            <Bookmark /> Sauvegarder la requête
          </Button>
          <Button onClick={() => { setNamingAlert((v) => !v); setNamingSave(false); }}>
            <BellPlus /> Transformer en alerte
          </Button>
        </div>
        {(namingSave || namingAlert) && (
          <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
            <input className="input" style={{ width: 240 }} placeholder="Nom..." value={nameInput}
              onChange={(e) => setNameInput(e.target.value)} autoFocus />
            <Button variant="primary" size="sm" onClick={() => void (namingSave ? handleSave() : handleConvertToAlert())}>
              <Check /> Confirmer
            </Button>
          </div>
        )}
      </div>

      {savedQueries.length > 0 && (
        <div className="card" style={{ marginBottom: 20 }}>
          <div className="section-title">Requêtes sauvegardées</div>
          {savedQueries.map((q) => (
            <div key={q.id} style={{ display: "flex", alignItems: "center", gap: 10, padding: "6px 0" }}>
              <span style={{ flex: 1, fontSize: ".85rem" }}>{q.name}</span>
              <Button size="sm" onClick={() => setQuery(criteriaToQuery(q.criteria))}>
                <Upload /> Charger
              </Button>
              <Button variant="ghost" size="sm" onClick={() => void remove(q.id)}>
                <Trash2 />
              </Button>
            </div>
          ))}
        </div>
      )}

      {results === null ? (
        <EmptyState icon={SearchIcon} message="Ajoutez au moins un critère pour lancer la recherche" />
      ) : results.length === 0 ? (
        <EmptyState icon={Bell} message="Aucun résultat pour ces critères" />
      ) : (
        <TableWrap>
          <table>
            <thead><tr><th>Heure</th><th>Processus</th><th>Destination</th><th>Visibilité</th><th></th></tr></thead>
            <tbody>
              {results.map((f) => (
                <tr key={f.id} style={{ cursor: "pointer" }} onClick={() => onSelectFlow(f.id)}>
                  <td className="mono" style={{ whiteSpace: "nowrap" }}>{f.timestamp}</td>
                  <td style={{ fontWeight: 500 }}>{f.process}</td>
                  <td className="mono">{f.destination}</td>
                  <td><VisibilityBadge visibility={f.visibility} /></td>
                  <td><Button variant="ghost" size="sm">Inspecter</Button></td>
                </tr>
              ))}
            </tbody>
          </table>
        </TableWrap>
      )}
    </div>
  );
}
