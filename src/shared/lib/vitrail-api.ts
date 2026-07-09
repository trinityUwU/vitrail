import { invoke } from "@tauri-apps/api/core";
import { logger } from "./logger";
import type {
  AlertRule,
  DashboardSummary,
  DestinationInfo,
  Exclusion,
  Flow,
  LogEntry,
  ProcessInfo,
  Session,
  Settings,
  SystemStatus,
  TeardownReport,
} from "./types";

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    logger.error({ command, args, error }, "Échec de la commande IPC Tauri");
    throw error;
  }
}

export const vitrailApi = {
  getDashboardSummary: (): Promise<DashboardSummary> => call("get_dashboard_summary"),

  listFlows: (): Promise<Flow[]> => call("list_flows"),
  getFlowDetail: (id: string): Promise<Flow | null> => call("get_flow_detail", { id }),
  searchFlows: (query: string): Promise<Flow[]> => call("search_flows", { query }),

  listProcesses: (): Promise<ProcessInfo[]> => call("list_processes"),
  getProcessDetail: (name: string): Promise<ProcessInfo | null> =>
    call("get_process_detail", { name }),

  listDestinations: (): Promise<DestinationInfo[]> => call("list_destinations"),
  getDestinationDetail: (domain: string): Promise<DestinationInfo | null> =>
    call("get_destination_detail", { domain }),

  activateVitrail: (): Promise<SystemStatus> => call("activate_vitrail"),
  deactivateVitrail: (): Promise<SystemStatus> => call("deactivate_vitrail"),
  emergencyStop: (): Promise<SystemStatus> => call("emergency_stop"),
  getSystemStatus: (): Promise<SystemStatus> => call("get_system_status"),
  verifyTeardown: (): Promise<TeardownReport> => call("verify_teardown"),

  getSettings: (): Promise<Settings> => call("get_settings"),
  updateSettings: (settings: Settings): Promise<Settings> => call("update_settings", { settings }),
  addExclusion: (name: string, kind: string): Promise<Exclusion> =>
    call("add_exclusion", { name, kind }),
  removeExclusion: (name: string): Promise<boolean> => call("remove_exclusion", { name }),
  rotateCa: (): Promise<Settings> => call("rotate_ca"),
  exportConfig: (): Promise<string> => call("export_config"),
  importConfig: (payload: string): Promise<Settings> => call("import_config", { payload }),
  listAlertRules: (): Promise<AlertRule[]> => call("list_alert_rules"),
  toggleAlertRule: (id: string): Promise<boolean> => call("toggle_alert_rule", { id }),
  listSessions: (): Promise<Session[]> => call("list_sessions"),
  getLogEntries: (): Promise<LogEntry[]> => call("get_log_entries"),
};
