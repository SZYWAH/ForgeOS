export type CodexMcpRegistrationState =
  | "codex_unavailable"
  | "sidecar_missing"
  | "not_configured"
  | "configured"
  | "conflict"
  | "invalid";

export interface CodexMcpRegistrationStatus {
  state: CodexMcpRegistrationState;
  codexVersion: string | null;
  expectedCommand: string | null;
  configuredCommand: string | null;
  enabled: boolean;
  canConfigure: boolean;
}

export interface ConfigureCodexMcpResult {
  changed: boolean;
  restartRequired: boolean;
  backupCreated: boolean;
}
