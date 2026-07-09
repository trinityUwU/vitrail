import { logger } from "../shared/lib/logger";

export function downloadConfigJson(json: string): void {
  try {
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = "vitrail-config.json";
    link.click();
    URL.revokeObjectURL(url);
  } catch (error) {
    logger.error({ error }, "Échec de téléchargement de la configuration");
    throw error;
  }
}

export function readConfigFile(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.onerror = () => reject(reader.error ?? new Error("Échec de lecture du fichier"));
    reader.readAsText(file);
  });
}
