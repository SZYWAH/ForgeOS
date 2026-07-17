import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectActivityOverview } from "../hooks/useProjectOverviewWorkbench";
import type { ActivitySyncRun, ProjectPulse, SyncStatus } from "../types/project-pulse";
import type { RepositoryRecord } from "../types/repository";

const mocks = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: { projectId: string; syncRunId: string } }) => void>(),
  listen: vi.fn(),
  listRepositories: vi.fn(),
  listSyncRuns: vi.fn(),
  getSyncRun: vi.fn(),
  getLatestPulse: vi.fn(),
  startSync: vi.fn(),
  getCurrentSession: vi.fn(),
  getCurrentSnapshot: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("../api/repository", () => ({ repositoryApi: { list: mocks.listRepositories } }));
vi.mock("../api/project-pulse", () => ({
  projectPulseApi: {
    syncRuns: mocks.listSyncRuns,
    syncRun: mocks.getSyncRun,
    latest: mocks.getLatestPulse,
    sync: mocks.startSync,
  },
}));
vi.mock("../api/session-kernel", () => ({
  sessionKernelApi: { getCurrentSession: mocks.getCurrentSession },
}));
vi.mock("../api/snapshot-kernel", () => ({
  snapshotKernelApi: { getCurrentSnapshot: mocks.getCurrentSnapshot },
}));

import { CurrentSessionCard } from "./CurrentSessionCard";
import { CurrentSnapshotCard } from "./CurrentSnapshotCard";
import { ProjectPulseCard } from "./ProjectPulseCard";
import { ProjectRuntimeStatusCard, summarizeRepositoryRuntime } from "./ProjectRuntimeStatusCard";

const NOW = "2026-07-13T10:00:00Z";

function repositoryRecord(overrides: Partial<RepositoryRecord["workingCopy"]> = {}, id = "repo-1"): RepositoryRecord {
  return {
    repository: {
      id,
      projectId: "project-1",
      path: `D:\\code\\${id}`,
      remoteUrl: null,
      defaultBranch: "main",
      lastCheckedAt: NOW,
      version: 1,
    },
    workingCopy: {
      id: `working-${id}`,
      repositoryId: id,
      branch: "main",
      headCommit: "abcdef1234567890",
      isDirty: false,
      diffSummary: {
        changedFiles: 0,
        stagedFiles: 0,
        unstagedFiles: 0,
        untrackedFiles: 0,
        insertions: 0,
        deletions: 0,
      },
      upstreamBranch: "origin/main",
      aheadCount: 0,
      behindCount: 0,
      checkedAt: NOW,
      ...overrides,
    },
    recentCommits: [],
  };
}

function syncRun(status: SyncStatus = "succeeded"): ActivitySyncRun {
  return {
    id: "run-1",
    projectId: "project-1",
    trigger: "manual",
    status,
    stage: status === "running" ? "reading_git" : "persisting",
    threadsScanned: 7,
    turnsImported: 12,
    observationsCreated: 3,
    commitsImported: 0,
    aiInvoked: true,
    startedAt: NOW,
    completedAt: status === "queued" || status === "running" ? null : NOW,
    errorCode: status === "failed" ? "sync_failed" : null,
    errorSummary: status === "failed" ? "Codex 暂时不可用" : null,
  };
}

function pulseRecord(status: ProjectPulse["status"] = "pending_review"): ProjectPulse {
  return {
    id: "pulse-1",
    projectId: "project-1",
    status,
    factStatus: status === "confirmed" ? "confirmed" : "observed_unconfirmed",
    analysisMode: "rules",
    activityType: "debugging",
    analysisRevision: "rules-v2",
    aiProviderId: null,
    aiModel: null,
    currentGoal: "修复 WebView2 安装失败",
    statusSummary: "已定位到安装器无法连接下载服务。",
    achievements: ["完成安装流程排查"],
    blockers: ["测试网络仍不稳定"],
    nextSteps: ["验证离线安装方案"],
    validationSummary: "Codex 报告：构建通过",
    handoff: "继续验证安装流程",
    gitSummary: "main，工作区有 3 个未提交文件",
    generatedAt: NOW,
    updatedAt: NOW,
    sourceWindowStartedAt: NOW,
    sourceWindowEndedAt: NOW,
    selectedObservationCount: 3,
    availableObservationCount: 12,
    qualityWarnings: [],
    lastSuccessfulSyncAt: NOW,
    freshness: "fresh",
    contentHash: "hash-1",
    sourceCount: 3,
    version: 1,
    confirmedSnapshotId: null,
    confirmedSessionId: null,
  };
}

function sharedActivity(overrides: Partial<ProjectActivityOverview> = {}): ProjectActivityOverview {
  return {
    pulse: pulseRecord(),
    run: syncRun(),
    repositories: [repositoryRecord({
      isDirty: true,
      diffSummary: {
        changedFiles: 3,
        stagedFiles: 1,
        unstagedFiles: 1,
        untrackedFiles: 1,
        insertions: 8,
        deletions: 2,
      },
    })],
    loading: false,
    running: false,
    reload: vi.fn().mockResolvedValue(undefined),
    startSync: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

describe("project workbench cards", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.listeners.clear();
    mocks.listen.mockImplementation(async (name: string, handler: (event: { payload: { projectId: string; syncRunId: string } }) => void) => {
      mocks.listeners.set(name, handler);
      return vi.fn();
    });
    mocks.listRepositories.mockResolvedValue([repositoryRecord()]);
    mocks.listSyncRuns.mockResolvedValue([syncRun()]);
    mocks.getSyncRun.mockResolvedValue(syncRun("running"));
    mocks.getLatestPulse.mockResolvedValue(pulseRecord());
    mocks.startSync.mockResolvedValue(syncRun("queued"));
    mocks.getCurrentSession.mockResolvedValue({
      id: "session-1",
      projectId: "project-1",
      status: "open",
      goal: "修复工作台布局",
      notes: "",
      outcome: "",
      validation: "",
      handoff: "",
      startedAt: NOW,
      completedAt: null,
      createdAt: NOW,
      updatedAt: NOW,
      version: 2,
    });
    mocks.getCurrentSnapshot.mockResolvedValue({
      id: "snapshot-1",
      projectId: "project-1",
      version: 3,
      currentGoal: "## My request for Codex:\n修复工作台布局",
      statusSummary: "## My request for Codex:\n建立内容驱动的单列工作台",
      achievements: ["完成 Snapshot 摘要治理"],
      blockers: [],
      nextSteps: ["验证响应式布局"],
      confirmedAt: NOW,
      createdAt: NOW,
      updatedAt: NOW,
    });
  });

  it("summarizes multiple repositories without inventing unknown ahead values", () => {
    const summary = summarizeRepositoryRuntime([
      repositoryRecord({
        isDirty: true,
        aheadCount: 2,
        diffSummary: { changedFiles: 3, stagedFiles: 1, unstagedFiles: 2, untrackedFiles: 0, insertions: 8, deletions: 2 },
      }),
      repositoryRecord({ aheadCount: null, behindCount: 1 }, "repo-2"),
    ]);
    expect(summary).toMatchObject({ repositoryCount: 2, changedFiles: 3, isDirty: true, ahead: "部分未知", behind: "1" });
  });

  it("uses the compact session layout and keeps the primary action short", async () => {
    const { container } = render(<MemoryRouter><CurrentSessionCard archived={false} projectId="project-1" projectName="ForgeOS" variant="compact" /></MemoryRouter>);
    expect(await screen.findByText("修复工作台布局")).toBeInTheDocument();
    expect(container.querySelector(".session-card--compact")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "查看开发记录" })).toBeInTheDocument();
  });

  it("renders the inline Session signal without a card shell", async () => {
    const { container } = render(<MemoryRouter><CurrentSessionCard archived={false} projectId="project-1" projectName="ForgeOS" variant="inline" /></MemoryRouter>);
    expect(await screen.findByText("修复工作台布局")).toBeInTheDocument();
    expect(container.querySelector(".workbench-signal--session")).toBeInTheDocument();
    expect(container.querySelector(".panel")).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "查看开发记录" })).toBeInTheDocument();
  });

  it("keeps only one Snapshot field expanded on the overview", async () => {
    const scrollHeight = vi.spyOn(HTMLElement.prototype, "scrollHeight", "get").mockReturnValue(180);
    const clientHeight = vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(40);
    const { container } = render(<MemoryRouter><CurrentSnapshotCard archived={false} projectId="project-1" projectName="ForgeOS" /></MemoryRouter>);
    await screen.findByText("修复工作台布局");
    fireEvent.click(screen.getAllByRole("button", { name: "展开全文" })[0]);
    expect(container.querySelectorAll('.collapsible-text[data-expanded="true"]')).toHaveLength(1);
    fireEvent.click(screen.getAllByRole("button", { name: "展开全文" })[0]);
    expect(container.querySelectorAll('.collapsible-text[data-expanded="true"]')).toHaveLength(1);
    scrollHeight.mockRestore();
    clientHeight.mockRestore();
  });

  it("keeps confirmed Snapshot facts collapsed in reference mode", async () => {
    render(<MemoryRouter><CurrentSnapshotCard archived={false} projectId="project-1" projectName="ForgeOS" variant="reference" /></MemoryRouter>);
    expect(await screen.findByRole("heading", { name: "上次人工确认状态" })).toBeInTheDocument();
    const disclosure = screen.getByText("展开已确认内容").closest("details");
    expect(disclosure).not.toHaveAttribute("open");
    fireEvent.click(screen.getByText("展开已确认内容"));
    expect(disclosure).toHaveAttribute("open");
    expect(screen.getByRole("link", { name: "查看完整当前状态" })).toBeInTheDocument();
  });

  it.each([
    ["queued", "正在同步…"],
    ["running", "正在同步…"],
    ["succeeded", "同步完成"],
    ["partial", "部分完成"],
    ["failed", "同步失败"],
  ] as const)("shows the %s sync state", async (status, label) => {
    mocks.listSyncRuns.mockResolvedValue([syncRun(status)]);
    render(<MemoryRouter><ProjectRuntimeStatusCard archived={false} projectId="project-1" /></MemoryRouter>);
    expect(await screen.findByText(label)).toBeInTheDocument();
  });

  it("keeps sync results visible when Git loading fails", async () => {
    mocks.listRepositories.mockRejectedValue(new Error("Git adapter unavailable"));
    render(<MemoryRouter><ProjectRuntimeStatusCard archived={false} projectId="project-1" /></MemoryRouter>);
    expect(await screen.findByText(/Git 状态读取失败/)).toBeInTheDocument();
    expect(screen.getByText("同步完成")).toBeInTheDocument();
  });

  it("keeps Git visible when the sync history query fails", async () => {
    mocks.listSyncRuns.mockRejectedValue(new Error("Sync history unavailable"));
    render(<MemoryRouter><ProjectRuntimeStatusCard archived={false} projectId="project-1" /></MemoryRouter>);
    expect(await screen.findByText(/同步记录读取失败/)).toBeInTheDocument();
    expect(screen.getByText("main")).toBeInTheDocument();
  });

  it("uses the shared activity state without querying Repository or SyncRun again", () => {
    render(<MemoryRouter><ProjectRuntimeStatusCard activityState={sharedActivity()} archived={false} projectId="project-1" variant="strip" /></MemoryRouter>);
    expect(screen.getByRole("group", { name: "代码仓库 · 最近同步" })).toBeInTheDocument();
    expect(screen.getByText("3 个未提交文件")).toBeInTheDocument();
    expect(mocks.listRepositories).not.toHaveBeenCalled();
    expect(mocks.listSyncRuns).not.toHaveBeenCalled();
  });

  it("renders Project Pulse as one document instead of an equal card grid", () => {
    const { container } = render(<MemoryRouter><ProjectPulseCard activityState={sharedActivity()} archived={false} projectId="project-1" variant="document" /></MemoryRouter>);
    expect(screen.getByRole("article", { name: "ForgeOS 整理的最新状态" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "最近完成" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "验证结果" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "当前问题" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "接下来做什么" })).toBeInTheDocument();
    expect(container.querySelector(".pulse-grid")).not.toBeInTheDocument();
  });

  it("refreshes only for events that belong to the current project", async () => {
    render(<MemoryRouter><ProjectRuntimeStatusCard archived={false} projectId="project-1" /></MemoryRouter>);
    await waitFor(() => expect(mocks.listRepositories).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(mocks.listeners.has("project-pulse://sync-completed")).toBe(true));

    await act(async () => {
      mocks.listeners.get("project-pulse://sync-completed")?.({ payload: { projectId: "project-2", syncRunId: "run-2" } });
    });
    expect(mocks.listRepositories).toHaveBeenCalledTimes(1);

    await act(async () => {
      mocks.listeners.get("project-pulse://sync-completed")?.({ payload: { projectId: "project-1", syncRunId: "run-1" } });
    });
    await waitFor(() => expect(mocks.listRepositories).toHaveBeenCalledTimes(2));
  });
});
