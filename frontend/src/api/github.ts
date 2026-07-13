import { invoke } from "@tauri-apps/api/core";
import type { GitHubConnector, GitHubImportDraft, GitHubImportResult } from "../types/github";

export const githubApi = {
  getConnector: () => invoke<GitHubConnector>("get_github_connector"),
  setToken: (token: string) => invoke<GitHubConnector>("set_github_token", { input: { token } }),
  clearToken: () => invoke<GitHubConnector>("clear_github_token"),
  preview: (repositoryUrl: string) => invoke<GitHubImportDraft>("preview_github_repository", { input: { repositoryUrl } }),
  listDrafts: () => invoke<GitHubImportDraft[]>("list_github_import_drafts"),
  review: (draftId: string, expectedVersion: number) => invoke<GitHubImportDraft>("review_github_import", { draftId, input: { expectedVersion } }),
  reject: (draftId: string, expectedVersion: number) => invoke<GitHubImportDraft>("reject_github_import", { draftId, input: { expectedVersion } }),
  importProject: (draftId: string, expectedVersion: number) => invoke<GitHubImportResult>("import_github_project", { draftId, input: { expectedVersion } }),
  getProjectImport: (projectId: string) => invoke<GitHubImportDraft | null>("get_project_github_import", { projectId }),
};
