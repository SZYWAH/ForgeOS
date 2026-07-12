import { invoke } from "@tauri-apps/api/core";
import type { Evidence } from "../types/evidence";
import type { GitInspection, RepositoryRecord } from "../types/repository";

export const repositoryApi = {
  inspect: (path: string) =>
    invoke<GitInspection>("inspect_repository", { input: { path } }),

  register: (projectId: string, path: string) =>
    invoke<RepositoryRecord>("register_repository", {
      projectId,
      input: { path },
    }),

  list: (projectId: string) =>
    invoke<RepositoryRecord[]>("list_repositories", { projectId }),

  refresh: (projectId: string, repositoryId: string, expectedVersion: number) =>
    invoke<RepositoryRecord>("refresh_repository", {
      projectId,
      repositoryId,
      input: { expectedVersion },
    }),

  createCommitEvidence: (projectId: string, repositoryId: string, commitSha: string) =>
    invoke<Evidence>("create_commit_evidence", {
      projectId,
      repositoryId,
      input: { commitSha },
    }),

  createStatusEvidence: (projectId: string, repositoryId: string) =>
    invoke<Evidence>("create_status_evidence", { projectId, repositoryId }),

  listEvidence: (projectId: string, repositoryId: string) =>
    invoke<Evidence[]>("list_repository_evidence", { projectId, repositoryId }),
};
