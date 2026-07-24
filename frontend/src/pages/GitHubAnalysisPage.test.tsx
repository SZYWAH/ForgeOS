import { fireEvent, render, screen, waitFor } from "@testing-library/react";
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
const aiProviderApi = vi.hoisted(() => ({ list: vi.fn() }));
const analysisImportApi = vi.hoisted(() => ({ get: vi.fn(), prepare: vi.fn() }));

vi.mock("../api/github", () => ({ githubAnalysisApi: analysisApi, githubAnalysisImportApi: analysisImportApi }));
vi.mock("../api/project-kernel", () => ({ projectKernelApi: projectApi }));
vi.mock("../api/ai-provider", () => ({ aiProviderApi }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => undefined) }));

const status = {
  analysisRunId: "run-1",
  owner: "owner",
  name: "sample",
  repositoryUrl: "https://github.com/owner/sample",
  focusType: "overview",
  customPrompt: undefined,
  analysisMode: "rules",
  aiProviderId: undefined,
  aiModel: undefined,
  analysisMethod: "local_rules",
  aiFallbackUsed: false,
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
  analyzerRevision: "github-rules-v2",
} as const;

describe("GitHub project analysis workbench", () => {
  beforeEach(() => {
    Object.values(analysisApi).forEach((mock) => mock.mockReset());
    projectApi.getCurrentWorkspace.mockReset();
    projectApi.getCurrentWorkspace.mockResolvedValue({ id: "workspace-1" });
    aiProviderApi.list.mockReset();
    aiProviderApi.list.mockResolvedValue([]);
    analysisImportApi.get.mockReset();
    analysisImportApi.get.mockResolvedValue(undefined);
    analysisImportApi.prepare.mockReset();
    analysisApi.list.mockResolvedValue({ items: [], nextCursor: undefined });
  });

  it("keeps analysis and the existing import flow visibly separate", async () => {
    render(<MemoryRouter><GitHubAnalysisPage /></MemoryRouter>);
    expect(await screen.findByRole("heading", { name: "分析 GitHub 项目" })).toBeInTheDocument();
    expect(screen.getByText("本地规则分析")).toBeInTheDocument();
    expect(screen.getByText("使用 AI 增强")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "导入自己的项目" })).toBeInTheDocument();
    expect(analysisApi.start).not.toHaveBeenCalled();
  });

  it("requires an explicit enabled provider before starting AI enhancement", async () => {
    aiProviderApi.list.mockResolvedValue([{
      id: "provider-1", name: "Local AI", providerType: "openai_compatible",
      baseUrl: "http://127.0.0.1:1234/v1", model: "local-model", enabled: true,
      hasApiKey: false, createdAt: "2026-07-22T00:00:00Z",
      updatedAt: "2026-07-22T00:00:00Z", version: 1,
    }]);
    analysisApi.start.mockResolvedValue({ analysisRunId: "run-ai", reusedExistingRun: false, warnings: [] });
    render(<MemoryRouter><GitHubAnalysisPage /></MemoryRouter>);
    await screen.findByRole("heading", { name: "分析 GitHub 项目" });
    fireEvent.change(screen.getByPlaceholderText("https://github.com/owner/repository"), { target: { value: "https://github.com/owner/sample" } });
    fireEvent.click(screen.getByRole("radio", { name: /使用 AI 增强/ }));
    fireEvent.click(screen.getByRole("button", { name: "开始分析" }));
    await waitFor(() => expect(analysisApi.start).toHaveBeenCalledWith(expect.objectContaining({
      analysisMode: "ai",
      aiProviderId: "provider-1",
    })));
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

  it("presents a release registry as metadata infrastructure instead of product source", async () => {
    aiProviderApi.list.mockResolvedValue([{
      id: "provider-1", name: "Local AI", providerType: "openai_compatible",
      baseUrl: "http://127.0.0.1:1234/v1", model: "local-model", enabled: true,
      hasApiKey: false, createdAt: "2026-07-22T00:00:00Z",
      updatedAt: "2026-07-22T00:00:00Z", version: 1,
    }]);
    analysisApi.start.mockResolvedValue({ analysisRunId: "run-ai", reusedExistingRun: false, warnings: [] });
    const source = { id: "source-1", sourceType: "readme", sourcePath: "README.md", excerpt: "Community plugins and release metadata", sourceOrder: 0 };
    analysisApi.getReport.mockResolvedValue({
      status: { ...status, status: "ready", readiness: "warning" },
      contentMarkdown: "# release registry report",
      contentHash: "b".repeat(64),
      sources: [source],
      report: {
        schemaVersion: "github-report-v2",
        repository: { owner: "owner", name: "releases", url: status.repositoryUrl, defaultBranch: "main", headCommitSha: status.sourceCommitSha },
        repositoryKind: { value: "release_registry", confidence: "high", sourceIds: [source.id] },
        capabilities: [], techStack: [], productTechStack: [], repositoryToolingStack: [], architecture: [], entrypoints: [], buildAndRun: [], testStrategy: [], risks: [], recommendedReadingOrder: [], extensionPoints: [], recentActivity: [], qualityWarnings: ["license_missing"],
        sectionAvailability: { product_tech_stack: "not_applicable", architecture: "not_applicable", recent_activity: "source_not_present" },
        reproductionAssessment: { value: { difficulty: "unknown", prerequisites: [], caveats: ["Static analysis only"] }, confidence: "low", sourceIds: [source.id] },
        completeness: { readiness: "warning", filesSelected: 1, filesSkipped: 0, treeEntriesSeen: 5, sourceBytes: 50 },
      },
    });
    render(<MemoryRouter initialEntries={["/github-import/analyses/run-1"]}><Routes><Route path="/github-import/analyses/:analysisId" element={<GitHubAnalysisReportPage />} /></Routes></MemoryRouter>);
    expect(await screen.findByText("发布与社区目录")).toBeInTheDocument();
    expect(screen.getByText(/不是产品应用源码仓库/)).toBeInTheDocument();
    expect(screen.getAllByText(/不适用/).length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "复制分析报告" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "使用 AI 增强" }));
    await waitFor(() => expect(analysisApi.start).toHaveBeenCalledWith(expect.objectContaining({
      repositoryUrl: status.repositoryUrl,
      analysisMode: "ai",
      aiProviderId: "provider-1",
    })));
  });
});
