import type { Evidence } from "./evidence";
import type { Project } from "./project";

export type GitHubConnector = {
  id: string;
  provider: "github";
  authType: "none" | "token_reference";
  tokenConfigured: boolean;
  createdAt: string;
  updatedAt: string;
  version: number;
};

export type GitHubCommitFact = {
  sha: string;
  message: string;
  author: string;
  authoredAt: string;
  url: string;
};

export type GitHubReleaseFact = {
  id: string;
  tagName: string;
  name: string;
  publishedAt?: string;
  url: string;
};

export type GitHubImportStatus = "draft" | "reviewed" | "imported" | "rejected";

export type GitHubImportDraft = {
  id: string;
  githubRepositoryId: string;
  owner: string;
  name: string;
  url: string;
  description: string;
  homepage?: string;
  defaultBranch: string;
  language?: string;
  languages: [string, number][];
  topics: string[];
  readmeSummary: string;
  recentCommits: GitHubCommitFact[];
  latestRelease?: GitHubReleaseFact;
  status: GitHubImportStatus;
  importedProjectId?: string;
  createdAt: string;
  updatedAt: string;
  version: number;
};

export type GitHubImportResult = {
  draft: GitHubImportDraft;
  project: Project;
  evidences: Evidence[];
};

export type GitHubAnalysisFocus = "overview" | "architecture" | "learning" | "reproduction" | "secondary_development" | "custom";
export type GitHubAnalysisStatus = "queued" | "collecting" | "analyzing" | "ready" | "partial" | "failed" | "archived";
export type GitHubAnalysisReadiness = "ready" | "warning" | "incomplete" | "blocked";
export type GitHubAnalysisStage = "resolving" | "collecting_tree" | "reading_sources" | "rules_analysis" | "ai_analysis" | "persisting";
export type GitHubAnalysisConfidence = "high" | "medium" | "low";

export type SourcedConclusion<T> = { value: T; confidence: GitHubAnalysisConfidence; sourceIds: string[] };

export type GitHubAnalysisStatusView = {
  analysisRunId: string; owner: string; name: string; repositoryUrl: string;
  focusType: GitHubAnalysisFocus; status: GitHubAnalysisStatus;
  readiness: GitHubAnalysisReadiness; stage?: GitHubAnalysisStage;
  defaultBranch: string; sourceCommitSha: string; stale: boolean;
  treeEntriesSeen: number; filesRead: number; filesSkipped: number; sourceBytes: number;
  apiRequestsUsed: number; apiRequestsRemaining?: number; apiRateLimitResetAt?: string;
  aiInvoked: boolean; startedAt?: string; completedAt?: string; createdAt: string;
  errorCode?: string; errorSummary?: string; version: number;
};

export type GitHubAnalysisSource = { id: string; sourceType: string; sourcePath: string; sourceSha?: string; excerpt: string; sourceOrder: number };

export type GitHubAnalysisReport = {
  repository: { owner: string; name: string; url: string; defaultBranch: string; headCommitSha: string };
  overview?: SourcedConclusion<string>;
  capabilities: SourcedConclusion<string>[];
  techStack: SourcedConclusion<string>[];
  architecture: SourcedConclusion<{ name: string; responsibility: string; keyPaths: string[] }>[];
  entrypoints: SourcedConclusion<{ path: string; purpose: string }>[];
  buildAndRun: SourcedConclusion<string>[];
  testStrategy: SourcedConclusion<string>[];
  dependencies?: SourcedConclusion<{ packageManagers: string[]; directDependencyCount?: number; sbomAvailable: boolean; summary: string }>;
  license?: SourcedConclusion<{ identifier?: string; reuseGuidance: string }>;
  risks: SourcedConclusion<string>[];
  recommendedReadingOrder: SourcedConclusion<string>[];
  reproductionAssessment: SourcedConclusion<{ difficulty: "low" | "medium" | "high" | "unknown"; prerequisites: string[]; caveats: string[] }>;
  extensionPoints: SourcedConclusion<string>[];
  userQuestionAnswer?: SourcedConclusion<string>;
  completeness: { readiness: GitHubAnalysisReadiness; filesSelected: number; filesSkipped: number; treeEntriesSeen: number; sourceBytes: number };
  qualityWarnings: string[];
};

export type GitHubAnalysisReportView = { status: GitHubAnalysisStatusView; report?: GitHubAnalysisReport; contentMarkdown?: string; contentHash?: string; sources: GitHubAnalysisSource[] };
export type GitHubAnalysisList = { items: GitHubAnalysisStatusView[]; nextCursor?: string };
export type StartGitHubAnalysisResult = { analysisRunId: string; reusedExistingRun: boolean; warnings: string[] };
