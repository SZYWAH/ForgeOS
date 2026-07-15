import { invoke } from "@tauri-apps/api/core";
import type {
  ActivitySyncRun,
  CodexConnectorSettings,
  ProjectActivitySettings,
  ProjectPulse,
  ProjectPulseSource,
  PulseDraftInput,
  SyncMode,
  SyncTrigger,
} from "../types/project-pulse";

export const projectPulseApi = {
  connectorStatus: () => invoke<CodexConnectorSettings>("get_codex_connector_status"),
  updateConnector: (executablePath: string | null, expectedVersion: number | null) => invoke<CodexConnectorSettings>("update_codex_connector_settings", { input: { executablePath, expectedVersion } }),
  settings: (projectId: string) => invoke<ProjectActivitySettings>("get_project_activity_settings", { projectId }),
  updateSettings: (projectId: string, input: { syncMode: SyncMode; aiEnabled: boolean; aiProviderId: string | null; expectedVersion: number | null }) => invoke<ProjectActivitySettings>("update_project_activity_settings", { projectId, input }),
  sync: (projectId: string, trigger: SyncTrigger) => invoke<ActivitySyncRun>("sync_project_activity", { projectId, input: { trigger } }),
  syncRun: (syncRunId: string) => invoke<ActivitySyncRun>("get_activity_sync_run", { syncRunId }),
  syncRuns: (projectId: string) => invoke<ActivitySyncRun[]>("list_activity_sync_runs", { projectId }),
  latest: (projectId: string) => invoke<ProjectPulse | null>("get_latest_project_pulse", { projectId }),
  list: (projectId: string) => invoke<ProjectPulse[]>("list_project_pulses", { projectId }),
  sources: (projectId: string, pulseId: string) => invoke<ProjectPulseSource[]>("get_project_pulse_sources", { projectId, pulseId }),
  updateDraft: (projectId: string, pulseId: string, input: PulseDraftInput) => invoke<ProjectPulse>("update_project_pulse_draft", { projectId, pulseId, input }),
  confirm: (projectId: string, pulseId: string, input: { expectedVersion: number; updateSnapshot: boolean; applySession: boolean; completeSession: boolean }) => invoke<ProjectPulse>("confirm_project_pulse", { projectId, pulseId, input }),
  dismiss: (projectId: string, pulseId: string, expectedVersion: number) => invoke<ProjectPulse>("dismiss_project_pulse", { projectId, pulseId, expectedVersion }),
};
