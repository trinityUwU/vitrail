import { logger } from "../shared/lib/logger";
import type { Flow, HttpHeader } from "../shared/lib/types";

function formatHeaders(headers: HttpHeader[]): string {
  return headers.map((h) => `${h.name}: ${h.value}`).join("\n");
}

export async function copyFlowHeaders(flow: Flow): Promise<void> {
  const text = [
    "# Requête",
    formatHeaders(flow.requestHeaders),
    "",
    "# Réponse",
    formatHeaders(flow.responseHeaders),
  ].join("\n");
  try {
    await navigator.clipboard.writeText(text);
  } catch (error) {
    logger.error({ error, flowId: flow.id }, "Échec de copie des headers dans le presse-papiers");
    throw error;
  }
}

export function downloadFlowJson(flow: Flow): void {
  try {
    const blob = new Blob([JSON.stringify(flow, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `vitrail-flow-${flow.id}.json`;
    link.click();
    URL.revokeObjectURL(url);
  } catch (error) {
    logger.error({ error, flowId: flow.id }, "Échec d'export du flux en JSON");
    throw error;
  }
}
