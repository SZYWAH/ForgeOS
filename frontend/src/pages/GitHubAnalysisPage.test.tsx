import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { GitHubAnalysisPage } from "./GitHubAnalysisPage";
import { GitHubAnalysisReportPage } from "./GitHubAnalysisReportPage";

const analysisApi = vi.hoisted(() => ({
  start: vi.fn(),
  getStatus: vi.fn(),
  getReport: vi.fn(),
  list: vi.fn(),
  reanalyze: vi.fn(),
  archive: vi.fn(),
}));
const projectApi = vi.hoisted(() => ({ getCurrentWorkspace: vi.fn() }));

vi.mock("../api/github", () => ({ githubAnalysisApi: analysisApi }));
vi.mock("../api/project-kernel", () => ({ projectKernelApi: projectApi }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => undefined) }));

const status = {
  analysisRunId: "run-1",
  owner: "owner",
  name: "sample",
  repositoryUrl: "https://github.com/owner/sample",
  focusType: "overview",
  status: "partial",
  readiness: "blocked",
  defaultBranch: "main",
  sourceCommitSha: "0123456789abcdef",
  stale: false,
  treeEntriesSeen: 12,
  filesRead: 2,
  filesSkipped: 1,
  sourceBytes: 100,
  apiRequestsUsed: 5,
  aiInvoked: false,
  createdAt: "2026-07-20T00:00:00Z",
  version: 3,
} as const;

describe("GitHub project analysis workbench", () => {
  beforeEach(() => {
    Object.values(analysisApi).forEach((mock) => mock.mockReset());
    projectApi.getCurrentWorkspace.mockReset();
    projectApi.getCurrentWorkspace.mockResolvedValue({ id: "workspace-1" });
    analysisApi.list.mockResolvedValue({ items: [], nextCursor: undefined });
  });

  it("keeps analysis and the existing import flow visibly separate", async () => {
    render(<MemoryRouter><GitHubAnalysisPage /></MemoryRouter>);
    expect(await screen.findByRole("heading", { name: "分析 GitHub 项目" })).toBeInTheDocument();
    expect(screen.getByText(/不会调用 AI/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "导入自己的项目" })).toBeInTheDocument();
    expect(analysisApi.start).not.toHaveBeenCalled();
  });

  it("shows dependency and license conclusions while blocking unsafe report copying", async () => {
    const source = { id: "source-1", sourceType: "manifest", sourcePath: "package.json", excerpt: "{}", sourceOrder: 0 };
    analysisApi.getReport.mockResolvedValue({
      status,
      contentMarkdown: "# safe report",
      contentHash: "a".repeat(64),
      sources: [source],
      report: {
        repository: { owner: "owner", name: "sample", url: status.repositoryUrl, defaultBranch: "main", headCommitSha: status.sourceCommitSha },
        capabilities: [], techStack: [], architecture: [], entrypoints: [], buildAndRun: [], testStrategy: [], risks: [], recommendedReadingOrder: [], extensionPoints: [], qualityWarnings: ["prompt_sanitized"],
        dependencies: { value: { packageManagers: ["npm"], sbomAvailable: false, summary: "Manifest fallback" }, confidence: "high", sourceIds: [source.id] },
        license: { value: { identifier: "MIT", reuseGuidance: "Review the complete license." }, confidence: "high", sourceIds: [source.id] },
        reproductionAssessment: { value: { difficulty: "medium", prerequisites: [], caveats: ["Static analysis only"] }, confidence: "low", sourceIds: [source.id] },
        completeness: { readiness: "blocked", filesSelected: 2, filesSkipped: 1, treeEntriesSeen: 12, sourceBytes: 100 },
      },
    });
    render(<MemoryRouter initialEntries={["/github-import/analyses/run-1"]}><Routes><Route path="/github-import/analyses/:analysisId" element={<GitHubAnalysisReportPage />} /></Routes></MemoryRouter>);
    expect(await screen.findByRole("heading", { name: "依赖与 SBOM" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "License 与二开提示" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "复制分析报告" })).toBeDisabled();
    expect(screen.getByText(/没有创建 ForgeOS 项目/)).toBeInTheDocument();
  });
});
