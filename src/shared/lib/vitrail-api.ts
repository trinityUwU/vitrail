import { invoke } from "@tauri-apps/api/core";
import { logger } from "./logger";
import type {
  AlertEvent,
  AlertRule,
  DashboardSummary,
  DestinationInfo,
  Exclusion,
  Flow,
  LogEntry,
  ProcessInfo,
  PurgeResult,
  SavedQuery,
  SearchCriteria,
  Session,
  SessionDetail,
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
  tagDestination: (domain: string, tag: string): Promise<DestinationInfo> =>
    call("tag_destination", { domain, tag }),

  activateVitrail: (): Promise<SystemStatus> => call("activate_vitrail"),
  deactivateVitrail: (): Promise<SystemStatus> => call("deactivate_vitrail"),
  emergencyStop: (): Promise<SystemStatus> => call("emergency_stop"),
  getSystemStatus: (): Promise<SystemStatus> => call("get_system_status"),
  verifyTeardown: (): Promise<TeardownReport> => call("verify_teardown"),

  getSettings: (): Promise<Settings> => call("get_settings"),
  updateSettings: (settings: Settings): Promise<Settings> => call("update_settings", { settings }),
  addExclusion: (name: string, kind: string): Promise<Exclusion> =>
    call("add_exclusion", { name, kind }),
  removeExclusion: (name: string): Promise<void> => call("remove_exclusion", { name }),
  rotateCa: (): Promise<Settings> => call("rotate_ca"),
  exportConfig: (): Promise<string> => call("export_config"),
  importConfig: (payload: string): Promise<Settings> => call("import_config", { payload }),
  listAlertRules: (): Promise<AlertRule[]> => call("list_alert_rules"),
  toggleAlertRule: (id: string): Promise<boolean> => call("toggle_alert_rule", { id }),
  createAlertRule: (name: string, description: string, criteria: string): Promise<AlertRule> =>
    call("create_alert_rule", { name, description, criteria }),
  updateAlertRule: (
    id: string,
    name: string,
    description: string,
    criteria: string,
  ): Promise<AlertRule> => call("update_alert_rule", { id, name, description, criteria }),
  deleteAlertRule: (id: string): Promise<void> => call("delete_alert_rule", { id }),
  listAlertEvents: (ruleId: string | null): Promise<AlertEvent[]> =>
    call("list_alert_events", { ruleId }),

  saveSearchQuery: (name: string, criteria: SearchCriteria): Promise<SavedQuery> =>
    call("save_search_query", { name, criteria }),
  listSavedQueries: (): Promise<SavedQuery[]> => call("list_saved_queries"),
  deleteSavedQuery: (id: string): Promise<void> => call("delete_saved_query", { id }),
  convertQueryToAlert: (queryId: string, alertName: string): Promise<AlertRule> =>
    call("convert_query_to_alert", { queryId, alertName }),

  listSessions: (): Promise<Session[]> => call("list_sessions"),
  getSessionDetail: (id: string): Promise<SessionDetail | null> =>
    call("get_session_detail", { id }),
  deleteSession: (id: string): Promise<void> => call("delete_session", { id }),

  getLogEntries: (): Promise<LogEntry[]> => call("get_log_entries"),
  purgeLogs: (): Promise<number> => call("purge_logs"),
  purgeData: (before: string | null): Promise<PurgeResult> => call("purge_data", { before }),

  listKeylogApps: (): Promise<string[]> => call("list_keylog_apps"),
  addKeylogApp: (path: string): Promise<string[]> => call("add_keylog_app", { path }),
  removeKeylogApp: (path: string): Promise<string[]> => call("remove_keylog_app", { path }),
};
