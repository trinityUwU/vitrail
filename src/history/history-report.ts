import { logger } from "../shared/lib/logger";
import { fmtDate, fmtVol } from "../shared/lib/format-utils";
import type { SessionDetail } from "../shared/lib/types";

function buildReportText(detail: SessionDetail): string {
  const { session, flows } = detail;
  const lines = [
    `Session ${session.id}`,
    `Du ${fmtDate(session.startedAt)} au ${fmtDate(session.endedAt)}`,
    `Volume : ${fmtVol(session.volumeMb)}`,
    `Processus : ${session.processCount}`,
    `Alertes : ${session.alertCount}`,
    "",
    "Flux :",
    ...flows.map((f) => `- ${f.timestamp} ${f.process} → ${f.destination} (${f.visibility})`),
  ];
  return lines.join("\n");
}

export function downloadSessionReport(detail: SessionDetail): void {
  try {
    const blob = new Blob([buildReportText(detail)], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `vitrail-rapport-${detail.session.id}.txt`;
    link.click();
    URL.revokeObjectURL(url);
  } catch (error) {
    logger.error({ error, sessionId: detail.session.id }, "Échec de génération du rapport de session");
    throw error;
  }
}
