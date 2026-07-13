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
