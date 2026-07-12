import type { Evidence } from "./evidence";

export type DiffSummary = {
  changedFiles: number;
  stagedFiles: number;
  unstagedFiles: number;
  untrackedFiles: number;
  insertions: number;
  deletions: number;
};

export type Repository = {
  id: string;
  projectId: string;
  path: string;
  remoteUrl: string | null;
  defaultBranch: string;
  lastCheckedAt: string;
  version: number;
};

export type WorkingCopy = {
  id: string;
  repositoryId: string;
  branch: string;
  headCommit: string;
  isDirty: boolean;
  diffSummary: DiffSummary;
  checkedAt: string;
};

export type GitCommit = {
  sha: string;
  message: string;
  authorName: string;
  authorEmail: string;
  authoredAt: string;
};

export type GitInspection = {
  rootPath: string;
  remoteUrl: string | null;
  defaultBranch: string;
  branch: string;
  headCommit: string;
  isDirty: boolean;
  diffSummary: DiffSummary;
  checkedAt: string;
  recentCommits: GitCommit[];
};

export type RepositoryRecord = {
  repository: Repository;
  workingCopy: WorkingCopy;
  recentCommits: GitCommit[];
};

export type RepositoryEvidence = Evidence;
