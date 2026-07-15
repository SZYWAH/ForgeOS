import type { Project } from "./project";
import type { DevelopmentSession } from "./session";
import type { ProjectSnapshot } from "./snapshot";

export type ProjectMemorySummary = {
  projectId: string;
  name: string;
  description: string;
  status: string;
  lifecycleStage: string;
  currentGoal: string;
  nextSteps: string[];
  blockerCount: number;
  recentActivityAt: string | null;
  pulseStatus: string | null;
  pulseFreshness: "fresh" | "stale" | null;
  hasRepository: boolean;
  dirtyFiles: number;
  aheadCount: number | null;
  behindCount: number | null;
  pendingAchievementCount: number;
};

export type MemoryPulse = {
  id: string;
  status: string;
  currentGoal: string;
  statusSummary: string;
  achievements: string[];
  blockers: string[];
  nextSteps: string[];
  generatedAt: string;
};

export type GitCommitSummary = {
  sha: string;
  message: string;
  authorName: string;
  authoredAt: string;
};

export type DevelopmentHistoryItem = {
  id: string;
  kind: "codex_activity" | "git_activity" | "manual_session";
  trustStatus: "observed" | "confirmed";
  title: string;
  goal: string;
  rationale: string;
  activity: string;
  outcome: string;
  validation: string;
  handoff: string;
  startedAt: string;
  endedAt: string | null;
  codexTurnCount: number;
  gitCommits: GitCommitSummary[];
  sessionId: string | null;
  pulseId: string | null;
};

export type DevelopmentHistoryPage = {
  items: DevelopmentHistoryItem[];
  nextCursor: string | null;
};

export type ProjectMemoryOverview = {
  project: Project;
  pulse: MemoryPulse | null;
  currentSnapshot: ProjectSnapshot | null;
  currentSession: DevelopmentSession | null;
  recentHistory: DevelopmentHistoryItem[];
};

export type ProjectAiContext = {
  projectId: string;
  contentMarkdown: string;
  contentHash: string;
};
