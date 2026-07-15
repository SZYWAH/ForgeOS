export type SyncMode = "open_manual" | "interval_30m";
export type SyncTrigger = "project_open" | "manual" | "interval";
export type SyncStatus = "queued" | "running" | "succeeded" | "partial" | "failed";
export type SyncStage = "connecting" | "reading_codex" | "reading_git" | "analyzing" | "persisting";
export type PulseStatus = "pending_review" | "confirmed" | "dismissed" | "superseded";
export type PulseActivityType = "implementation" | "debugging" | "release" | "review" | "support_question" | "unknown";
export type PulseAnalysisRevision = "legacy-v1" | "rules-v2" | "ai-v2";

export type CodexConnectorSettings = {
  workspaceId: string;
  executablePath: string | null;
  lastDetectedVersion: string | null;
  lastHealthCheckAt: string | null;
  version: number;
  available: boolean;
};

export type ProjectActivitySettings = {
  projectId: string;
  syncMode: SyncMode;
  aiEnabled: boolean;
  aiProviderId: string | null;
  initialBackfillCompleted: boolean;
  version: number;
  updatedAt: string;
};

export type ActivitySyncRun = {
  id: string;
  projectId: string;
  trigger: SyncTrigger;
  status: SyncStatus;
  stage: SyncStage;
  threadsScanned: number;
  turnsImported: number;
  observationsCreated: number;
  aiInvoked: boolean;
  startedAt: string;
  completedAt: string | null;
  errorCode: string | null;
  errorSummary: string | null;
};

export type ProjectPulse = {
  id: string;
  projectId: string;
  status: PulseStatus;
  factStatus: "observed_unconfirmed" | "confirmed";
  analysisMode: "rules" | "ai";
  activityType: PulseActivityType;
  analysisRevision: PulseAnalysisRevision;
  aiProviderId: string | null;
  aiModel: string | null;
  currentGoal: string;
  statusSummary: string;
  achievements: string[];
  blockers: string[];
  nextSteps: string[];
  validationSummary: string;
  handoff: string;
  gitSummary: string;
  generatedAt: string;
  updatedAt: string;
  sourceWindowStartedAt: string | null;
  sourceWindowEndedAt: string | null;
  selectedObservationCount: number;
  availableObservationCount: number;
  qualityWarnings: string[];
  lastSuccessfulSyncAt: string | null;
  freshness: "fresh" | "stale";
  contentHash: string;
  sourceCount: number;
  version: number;
  confirmedSnapshotId: string | null;
  confirmedSessionId: string | null;
};

export type ProjectPulseSource = {
  id: string;
  threadId: string;
  turnId: string;
  sourceKind: string;
  turnStatus: string;
  goalHint: string;
  activitySummary: string;
  outcomeSummary: string;
  validationSummary: string;
  handoffSummary: string;
  blockers: string[];
  sourceUpdatedAt: string;
};

export type PulseDraftInput = Pick<ProjectPulse, "currentGoal" | "statusSummary" | "achievements" | "blockers" | "nextSteps" | "validationSummary" | "handoff" | "gitSummary"> & { expectedVersion: number };
