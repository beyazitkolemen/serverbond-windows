export interface PackageStatus {
  id: string;
  name: string;
  description: string;
  version: string;
  installed: boolean;
  running: boolean;
  pid: number | null;
  repairable: boolean;
  issue: string | null;
  license: string;
  source: string;
}
export interface QueueWorker {
  id: string;
  name: string;
  connection: string;
  queue: string;
  processes: number;
  timeout: number;
  sleep: number;
  maxTries: number;
  memory: number;
  backoff: number;
  enabled: boolean;
  autoStart: boolean;
}
export interface ProjectSchedule {
  enabled: boolean;
  autoStart: boolean;
}
export interface WorkerState {
  id: string;
  running: number;
  pids: number[];
  issue: string | null;
}
export interface Project {
  id: string;
  name: string;
  path: string;
  host: string;
  phpVersion: string;
  running: boolean;
  phpPort: number | null;
  issue: string | null;
  workers: QueueWorker[];
  schedule: ProjectSchedule;
  workerStates: WorkerState[];
  scheduleRunning: boolean;
  schedulePid: number | null;
  scheduleIssue: string | null;
}
export interface PhpSettings {
  timezone: string;
  memoryMb: number;
  uploadMb: number;
  postMb: number;
  executionSeconds: number;
  inputSeconds: number;
  inputVars: number;
  displayErrors: boolean;
  logErrors: boolean;
  opcache: boolean;
  opcacheMb: number;
  extensions: string[];
  extraIni: string;
}
export interface Settings {
  webPort: number;
  mysqlPort: number;
  phpPort: number;
  php: PhpSettings;
  phpVersions: Record<string, PhpSettings>;
  mysql: {
    bufferPoolMb: number;
    maxConnections: number;
    maxPacketMb: number;
    waitSeconds: number;
    collation: string;
    sqlMode: string;
    slowQueryLog: boolean;
    longQuerySeconds: number;
  };
  web: {
    hostPattern: string;
    compression: boolean;
    accessLog: boolean;
    readSeconds: number;
    connectSeconds: number;
  };
  phpmyadmin: {
    enabled: boolean;
    language: string;
    rows: number;
    loginSeconds: number;
  };
  tunnel: {
    autoStart: boolean;
  };
  mail: MailSettings;
  projectsDir: string;
  backupsDir: string;
  startOnLaunch: boolean;
}
export interface MailSettings {
  smtpPort: number;
  webPort: number;
  autoStart: boolean;
  relayPhpMail: boolean;
  maxMessages: number;
}
export interface MailState {
  version: string;
  installed: boolean;
  running: boolean;
  pid: number | null;
  smtpPort: number;
  webPort: number;
  autoStart: boolean;
  relayPhpMail: boolean;
  issue: string | null;
}
export interface NodeState {
  version: string;
  installed: boolean;
  directory: string | null;
}
export interface TunnelState {
  version: string;
  installed: boolean;
  running: boolean;
  pid: number | null;
  tokenSaved: boolean;
  autoStart: boolean;
  issue: string | null;
}
export interface PermissionState {
  granted: boolean;
  appliedAt: string | null;
  applied: string[];
  failed: string[];
  programs: string[];
  defenderExclusion: boolean;
  pending: string[];
}
export interface Snapshot {
  packages: PackageStatus[];
  phpVersions: PackageStatus[];
  projects: Project[];
  tunnel: TunnelState;
  mail: MailState;
  node: NodeState;
  permissions: PermissionState;
  logs: string[];
  home: string;
  settings: Settings;
  busy: boolean;
  anyRunning: boolean;
  recoveryIssue: string | null;
  restartRequired: boolean;
}
export interface Requirement {
  id: string;
  label: string;
  status: "ok" | "warning" | "error";
  detail: string;
  helpUrl: string | null;
}
export type Page = "overview" | "packages" | "projects" | "logs" | "settings";
export type Run = (
  label: string,
  action: () => Promise<unknown>,
) => Promise<boolean>;
