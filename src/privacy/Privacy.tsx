import type { ReactElement } from "react";
import { ShieldCheck } from "lucide-react";
import { Button } from "../shared/components/Button";
import { useToast } from "../shared/hooks/useToast";
import { logger } from "../shared/lib/logger";
import { vitrailApi } from "../shared/lib/vitrail-api";

const FILE_LOCATIONS: Array<{ label: string; path: string }> = [
  { label: "Certificat CA (clé privée)", path: "~/.vitrail/ca/key.pem" },
  { label: "Certificat CA (certificat public)", path: "~/.vitrail/ca/cert.pem" },
  { label: "Base de données (trafic)", path: "~/.vitrail/data/vitrail.db" },
  { label: "Fichier keylog SSL", path: "~/.vitrail/keylog/sslkeys.log" },
  { label: "Logs système", path: "~/.vitrail/logs/" },
];

export function Privacy(): ReactElement {
  const { showToast } = useToast();

  const handlePurgeAll = async (): Promise<void> => {
    try {
      const result = await vitrailApi.purgeData(null);
      showToast(`${result.deletedFlows} flux supprimés, ${result.freedMb.toFixed(1)} Mo libérés`);
    } catch (error) {
      logger.error({ error }, "Échec de la purge totale");
    }
  };

  return (
    <div>
      <div className="screen-title">Confidentialité & gouvernance des données</div>
      <div className="screen-subtitle">Transparence sur le traitement de vos données</div>

      <div className="card" style={{ marginBottom: 16, borderColor: "rgba(45,90,61,.2)", background: "var(--ok-l)" }}>
        <div style={{ display: "flex", alignItems: "flex-start", gap: 12 }}>
          <ShieldCheck style={{ width: 24, height: 24, color: "var(--ok)", flexShrink: 0, marginTop: 2 }} />
          <div>
            <div style={{ fontWeight: 600, color: "var(--ok)", marginBottom: 6 }}>Tout reste local, toujours</div>
            <p style={{ fontSize: ".85rem", color: "var(--t2)", lineHeight: 1.7 }}>
              Aucune donnée — ni métadonnée, ni contenu déchiffré, ni statistique d'usage — n'est jamais
              envoyée, transmise ou partagée en dehors de cette machine. Vitrail ne contacte aucun serveur
              distant. Il n'y a pas de télémétrie, pas de rapport d'erreur automatique, pas de vérification
              de mise à jour en arrière-plan.
            </p>
          </div>
        </div>
      </div>

      <div className="card" style={{ marginBottom: 16 }}>
        <div className="section-title">Emplacement des fichiers</div>
        <div style={{ display: "grid", gap: 12, fontSize: ".83rem" }}>
          {FILE_LOCATIONS.map((f) => (
            <div key={f.path} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "8px 0", borderBottom: "1px solid var(--border-s)" }}>
              <span>{f.label}</span>
              <span className="mono" style={{ fontSize: ".78rem", color: "var(--t2)" }}>{f.path}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="card" style={{ marginBottom: 16 }}>
        <div className="section-title">Ce qui est stocké en clair sur disque</div>
        <div style={{ fontSize: ".83rem", lineHeight: 1.8, color: "var(--t2)" }}>
          <p style={{ marginBottom: 8 }}>
            <strong style={{ color: "var(--t1)" }}>En clair :</strong> le contenu des requêtes/réponses HTTP
            déchiffrées (headers, corps), les métadonnées de tous les flux (5-tuple, timestamps, tailles), les
            clés TLS exportées via SSLKEYLOGFILE.
          </p>
          <p>
            <strong style={{ color: "var(--t1)" }}>Jamais stocké :</strong> les clés privées de la CA ne
            quittent pas le répertoire CA. Les flux non déchiffrés (pinning) ne stockent que des métadonnées
            réseau. Aucun payload binaire supérieur à 1 Mo n'est stocké inline (référence uniquement).
          </p>
        </div>
      </div>

      <div className="card" style={{ borderColor: "rgba(184,59,52,.2)", background: "var(--danger-l)" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <div style={{ fontWeight: 600, color: "var(--danger)", marginBottom: 2 }}>Purge totale et immédiate</div>
            <div style={{ fontSize: ".8rem", color: "var(--danger)", opacity: 0.8 }}>
              Supprime définitivement toutes les données collectées : base de données, logs, clés TLS
              exportées. Cette action est irréversible.
            </div>
          </div>
          <Button variant="danger" onClick={() => void handlePurgeAll()}>Tout purger</Button>
        </div>
      </div>
    </div>
  );
}
