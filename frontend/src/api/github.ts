import { invoke } from "@tauri-apps/api/core";
import type { GitHubAnalysisFocus, GitHubAnalysisImportProjectFields, GitHubAnalysisImportView, GitHubAnalysisList, GitHubAnalysisReportView, GitHubAnalysisStatusView, GitHubConnector, GitHubImportDraft, GitHubImportResult, StartGitHubAnalysisResult } from "../types/github";

export const githubApi = {
  getConnector: () => invoke<GitHubConnector>("get_github_connector"),
  setToken: (token: string) => invoke<GitHubConnector>("set_github_token", { input: { token } }),
  clearToken: () => invoke<GitHubConnector>("clear_github_token"),
  preview: (repositoryUrl: string) => invoke<GitHubImportDraft>("preview_github_repository", { input: { repositoryUrl } }),
  listDrafts: () => invoke<GitHubImportDraft[]>("list_github_import_drafts"),
  review: (draftId: string, expectedVersion: number) => invoke<GitHubImportDraft>("review_github_import", { draftId, input: { expectedVersion } }),
  reject: (draftId: string, expectedVersion: number) => invoke<GitHubImportDraft>("reject_github_import", { draftId, input: { expectedVersion } }),
  importProject: (draftId: string, expectedVersion: number, workspaceId?: string, proposalExpectedVersion?: number) => invoke<GitHubImportResult>("import_github_project", { draftId, input: { expectedVersion, workspaceId, proposalExpectedVersion } }),
  getProjectImport: (projectId: string) => invoke<GitHubImportDraft | null>("get_project_github_import", { projectId }),
};

export const githubAnalysisImportApi = {
  prepare: (workspaceId: string, analysisRunId: string) => invoke<GitHubAnalysisImportView>("import_github_analysis", { input: { workspaceId, analysisRunId } }),
  get: (workspaceId: string, analysisRunId: string) => invoke<GitHubAnalysisImportView>("get_github_analysis_import", { input: { workspaceId, analysisRunId } }),
  update: (input: {
    workspaceId: string;
    draftId: string;
    expectedDraftVersion: number;
    expectedProposalVersion: number;
    project: GitHubAnalysisImportProjectFields;
    partialAcknowledged: boolean;
    staleAcknowledged: boolean;
  }) => invoke<GitHubAnalysisImportView>("update_github_analysis_import", { input: {
    workspaceId: input.workspaceId,
    draftId: input.draftId,
    expectedDraftVersion: input.expectedDraftVersion,
    expectedProposalVersion: input.expectedProposalVersion,
    projectName: input.project.name,
    projectSlug: input.project.slug,
    description: input.project.description,
    vision: input.project.vision,
    targetUser: input.project.targetUser,
    scope: input.project.scope,
    nonGoals: input.project.nonGoals,
    techStack: input.project.techStack,
    projectStatus: input.project.status,
    lifecycleStage: input.project.lifecycleStage,
    priority: input.project.priority,
    partialAcknowledged: input.partialAcknowledged,
    staleAcknowledged: input.staleAcknowledged,
  } }),
  review: (input: {
    workspaceId: string;
    draftId: string;
    expectedDraftVersion: number;
    expectedProposalVersion: number;
    partialAcknowledged: boolean;
    staleAcknowledged: boolean;
  }) => invoke<GitHubAnalysisImportView>("review_github_analysis_import", { input }),
  refresh: (workspaceId: string, analysisRunId: string, expectedDraftVersion: number, expectedProposalVersion: number) => invoke<GitHubAnalysisImportView>("refresh_github_analysis_import_from_report", { input: { workspaceId, analysisRunId, expectedDraftVersion, expectedProposalVersion } }),
};

export const githubAnalysisApi = {
  start: (input: { workspaceId: string; repositoryUrl: string; focusType: GitHubAnalysisFocus; customPrompt?: string; analysisMode: "rules" | "ai"; aiProviderId?: string }) => invoke<StartGitHubAnalysisResult>("start_github_analysis", { input }),
  getStatus: (workspaceId: string, analysisRunId: string) => invoke<GitHubAnalysisStatusView>("get_github_analysis_status", { input: { workspaceId, analysisRunId } }),
  getReport: (workspaceId: string, analysisRunId: string) => invoke<GitHubAnalysisReportView>("get_github_analysis_report", { input: { workspaceId, analysisRunId } }),
  list: (workspaceId: string, cursor?: string, limit = 20) => invoke<GitHubAnalysisList>("list_github_analyses", { input: { workspaceId, cursor, limit } }),
  reanalyze: (workspaceId: string, analysisRunId: string) => invoke<StartGitHubAnalysisResult>("reanalyze_github_project", { input: { workspaceId, analysisRunId } }),
  archive: (workspaceId: string, analysisRunId: string, expectedVersion: number) => invoke<GitHubAnalysisStatusView>("archive_github_analysis", { input: { workspaceId, analysisRunId, expectedVersion } }),
};
