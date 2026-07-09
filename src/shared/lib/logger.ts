import pino from "pino";

// pino détecte l'environnement navigateur automatiquement (frontend Tauri = webview).
export const logger = pino({ level: "info", browser: { asObject: true } });
