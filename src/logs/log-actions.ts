import { logger } from "../shared/lib/logger";
import type { LogEntry } from "../shared/lib/types";

function formatEntries(entries: LogEntry[]): string {
  return entries.map((e) => `[${e.time}] ${e.level.toUpperCase()} ${e.subsystem}: ${e.message}`).join("\n");
}

export async function copyLogEntries(entries: LogEntry[]): Promise<void> {
  try {
    await navigator.clipboard.writeText(formatEntries(entries));
  } catch (error) {
    logger.error({ error }, "Échec de copie du journal dans le presse-papiers");
    throw error;
  }
}

export function downloadLogEntries(entries: LogEntry[]): void {
  try {
    const blob = new Blob([JSON.stringify(entries, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = "vitrail-logs.json";
    link.click();
    URL.revokeObjectURL(url);
  } catch (error) {
    logger.error({ error }, "Échec d'export du journal");
    throw error;
  }
}
