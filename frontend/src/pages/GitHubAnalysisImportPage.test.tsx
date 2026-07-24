import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { GitHubAnalysisImportPage } from "./GitHubAnalysisImportPage";

const importApi = vi.hoisted(() => ({
  prepare: vi.fn(),
  get: vi.fn(),
  update: vi.fn(),
  review: vi.fn(),
  refresh: vi.fn(),
}));
const githubApi = vi.hoisted(() => ({ importProject: vi.fn() }));
const projectApi = vi.hoisted(() => ({ getCurrentWorkspace: vi.fn() }));

vi.mock("../api/github", () => ({
  githubAnalysisImportApi: importApi,
  githubApi,
}));
vi.mock("../api/project-kernel", () => ({ projectKernelApi: projectApi }));

const analysis = {
  analysisRunId: "analysis-1",
  owner: "SZYWAH",
  name: "sample",
  repositoryUrl: "https://github.com/SZYWAH/sample",
  focusType: "overview",
  analysisMode: "rules",
  analysisMethod: "local_rules",
  aiFallbackUsed: false,
  status: "partial",
  readiness: "warning",
  defaultBranch: "main",
  sourceCommitSha: "a".repeat(40),
  analyzerRevision: "github-rules-v2",
  stale: true,
  treeEntriesSeen: 10,
  filesRead: 2,
  filesSkipped: 1,
  sourceBytes: 100,
  apiRequestsUsed: 5,
  aiInvoked: false,
  createdAt: "2026-07-22T00:00:00Z",
  version: 1,
} as const;

const draft = {
  id: "draft-1",
  githubRepositoryId: "repo-1",
  owner: "SZYWAH",
  name: "sample",
  url: analysis.repositoryUrl,
  description: "Repository description",
  defaultBranch: "main",
  languages: [],
  topics: [],
  readmeSummary: "",
  recentCommits: [],
  status: "draft",
  createdAt: "2026-07-22T00:00:00Z",
  updatedAt: "2026-07-22T00:00:00Z",
  version: 1,
} as const;

const proposal = {
  draftId: draft.id,
  workspaceId: "workspace-1",
  analysisRunId: analysis.analysisRunId,
  sourceCommitSha: analysis.sourceCommitSha,
  sourceContentHash: "b".repeat(64),
  projectName: "sample",
  projectSlug: "sample",
  description: "Repository description",
  vision: "",
  targetUser: "",
  scope: "Repository type: source_application",
  nonGoals: "",
  techStack: "Rust, TypeScript",
  projectStatus: "active",
  lifecycleStage: "definition",
  priority: 50,
  partialAcknowledged: false,
  staleAcknowledged: false,
  createdAt: "2026-07-22T00:00:00Z",
  updatedAt: "2026-07-22T00:00:00Z",
  version: 1,
} as const;

const view = {
  analysis,
  draft,
  proposal,
  stale: true,
  sourceChanged: false,
};

describe("GitHub analysis human import", () => {
  beforeEach(() => {
    Object.values(importApi).forEach((mock) => mock.mockReset());
    githubApi.importProject.mockReset();
    projectApi.getCurrentWorkspace.mockReset();
    projectApi.getCurrentWorkspace.mockResolvedValue({ id: "workspace-1" });
    importApi.prepare.mockResolvedValue(view);
  });

  it("shows the editable project draft and explicit import boundaries", async () => {
    render(
      <MemoryRouter initialEntries={["/github-import/analyses/analysis-1/import"]}>
        <Routes>
          <Route path="/github-import/analyses/:analysisId/import" element={<GitHubAnalysisImportPage />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByRole("heading", { name: "审核项目导入草稿" })).toBeInTheDocument();
    expect(screen.getByDisplayValue("Rust, TypeScript")).toBeInTheDocument();
    expect(screen.getByText("一个 ForgeOS 项目")).toBeInTheDocument();
    expect(screen.getByText("当前项目状态")).toBeInTheDocument();
    expect(screen.getByText("本地代码仓库")).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "我已了解分析内容不完整" })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: "我已了解这不是仓库最新 HEAD" })).not.toBeChecked();
  });

  it("saves user edits before allowing review", async () => {
    importApi.update.mockResolvedValue({
      ...view,
      draft: { ...draft, version: 2 },
      proposal: {
        ...proposal,
        projectName: "Edited project",
        partialAcknowledged: true,
        staleAcknowledged: true,
        version: 2,
      },
    });
    render(
      <MemoryRouter initialEntries={["/github-import/analyses/analysis-1/import"]}>
        <Routes>
          <Route path="/github-import/analyses/:analysisId/import" element={<GitHubAnalysisImportPage />} />
        </Routes>
      </MemoryRouter>,
    );
    const name = await screen.findByRole("textbox", { name: "项目名称" });
    fireEvent.change(name, { target: { value: "Edited project" } });
    fireEvent.click(screen.getByRole("checkbox", { name: "我已了解分析内容不完整" }));
    fireEvent.click(screen.getByRole("checkbox", { name: "我已了解这不是仓库最新 HEAD" }));
    fireEvent.click(screen.getByRole("button", { name: "保存草稿" }));

    await waitFor(() => expect(importApi.update).toHaveBeenCalledWith(expect.objectContaining({
      draftId: "draft-1",
      expectedDraftVersion: 1,
      expectedProposalVersion: 1,
      partialAcknowledged: true,
      staleAcknowledged: true,
      project: expect.objectContaining({ name: "Edited project" }),
    })));
    expect(await screen.findByText("项目草稿已保存。")).toBeInTheDocument();
  });
});
