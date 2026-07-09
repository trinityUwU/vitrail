// Types miroir des structs Rust sérialisées par src-tauri/src/commands (EPIC 8.5).
// Génération automatique non encore branchée — à synchroniser manuellement avec
// src-tauri/src/commands/types.rs tant que ts-rs/specta n'est pas intégré.

export type FlowVisibility = "fully" | "meta" | "attrib" | "unknown";

export interface ProcessInfo {
  name: string;
  path: string;
  pids: number[];
  volumeMb: number;
  destinations: number;
  visibility: FlowVisibility;
  keylogCovered: boolean;
}

export interface DestinationInfo {
  domain: string;
  ip: string;
  volumeMb: number;
  processCount: number;
  visibility: FlowVisibility;
  tls: boolean;
  pinning: boolean;
  firstSeen: string;
  lastSeen: string;
  tag: string | null;
}

export interface HttpHeader {
  name: string;
  value: string;
}

export interface CertificateInfo {
  issuer: string;
  subject: string;
  validFrom: string;
  validTo: string;
  fingerprintSha256: string;
}

export type CorrelationStatus = "ok" | "warn" | "off";

export interface CorrelationSource {
  name: string;
  status: CorrelationStatus;
  detail: string;
}

export interface Flow {
  id: string;
  timestamp: string;
  process: string;
  destination: string;
  ip: string;
  port: number;
  protocol: string;
  sizeBytes: number;
  durationMs: number;
  visibility: FlowVisibility;
  method: string | null;
  path: string | null;
  status: number | null;
  sourceIp: string;
  sourcePort: number;
  requestHeaders: HttpHeader[];
  responseHeaders: HttpHeader[];
  bodyPreview: string | null;
  contentType: string | null;
  certificate: CertificateInfo | null;
  sources: CorrelationSource[];
}

export interface DashboardSummary {
  killSwitchActive: boolean;
  activeSince: string | null;
  activeConnections: number;
  totalInMb: number;
  totalOutMb: number;
  metaOnlyCount: number;
  topProcesses: ProcessInfo[];
  topDestinations: DestinationInfo[];
  degraded: boolean;
  degradedReason: string | null;
}

export interface SubsystemStatus {
  id: string;
  name: string;
  detail: string;
  status: "ok" | "err" | "wait" | "off";
}

export interface SystemStatus {
  killSwitchState: "active" | "inactive" | "transitioning" | "degraded";
  subsystems: SubsystemStatus[];
  lastVerification: string | null;
  lastVerificationClean: boolean;
}

export interface TeardownReport {
  clean: boolean;
  divergences: string[];
  checkedAt: string;
}

export interface Exclusion {
  name: string;
  type: string;
}

export interface AlertRule {
  id: string;
  name: string;
  description: string;
  criteria: string;
  active: boolean;
  triggerCount: number;
}

export interface Session {
  id: string;
  startedAt: string;
  endedAt: string;
  volumeMb: number;
  processCount: number;
  alertCount: number;
}

export interface LogEntry {
  time: string;
  level: "info" | "warn" | "error";
  subsystem: string;
  message: string;
}

export interface Settings {
  caFingerprint: string;
  caTrustStoreInstalled: boolean;
  nftablesChain: string;
  monitoredInterfaces: string[];
  retentionDays: number | null;
  databaseSizeMb: number;
  notificationsEnabled: boolean;
  notificationSound: boolean;
}

export interface AlertEvent {
  id: string;
  ruleId: string;
  flowId: string;
  time: string;
  summary: string;
  visibility: FlowVisibility;
}

export interface SearchCriteria {
  process: string | null;
  destination: string | null;
  port: string | null;
  visibility: FlowVisibility | null;
  from: string | null;
  to: string | null;
  text: string | null;
}

export interface SavedQuery {
  id: string;
  name: string;
  criteria: SearchCriteria;
}

export interface PurgeResult {
  deletedFlows: number;
  freedMb: number;
}

export interface SessionDetail {
  session: Session;
  flows: Flow[];
}

export type ScreenId =
  | "onboarding"
  | "dashboard"
  | "timeline"
  | "processes"
  | "destinations"
  | "inspector"
  | "search"
  | "alerts"
  | "killswitch"
  | "settings"
  | "privacy"
  | "logs"
  | "history";
