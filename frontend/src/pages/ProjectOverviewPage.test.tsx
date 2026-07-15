import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  getProject: vi.fn(),
  useWorkbench: vi.fn(),
  startSync: vi.fn(),
}));

const project = {
  id: "project-1",
  workspaceId: "workspace-1",
  slug: "forgeos",
  name: "ForgeOS",
  description: "",
  vision: "",
  targetUser: "",
  scope: "",
  nonGoals: "",
  techStack: "React",
  status: "active",
  lifecycleStage: "implementation",
  priority: 1,
  createdAt: "2026-07-13T10:00:00Z",
  updatedAt: "2026-07-13T10:00:00Z",
  version: 1,
};

function workbenchState(status?: "pending_review" | "confirmed") {
  return {
    pulse: status ? { status } : null,
    run: null,
    repositories: [],
    loading: false,
    running: false,
    reload: vi.fn().mockResolvedValue(undefined),
    startSync: mocks.startSync,
  };
}

vi.mock("../api/project-kernel", () => ({
  projectKernelApi: { getProject: mocks.getProject },
  getErrorMessage: (value: unknown) => String(value),
}));
vi.mock("../hooks/useProjectOverviewWorkbench", () => ({
  useProjectOverviewWorkbench: mocks.useWorkbench,
  SYNC_STAGE_LABELS: {
    connecting: "连接 Codex",
    reading_codex: "读取 Codex",
    reading_git: "检查 Git",
    analyzing: "分析项目状态",
    persisting: "生成 Project Pulse",
  },
}));
vi.mock("../components/CurrentSnapshotCard", () => ({
  CurrentSnapshotCard: ({ variant }: { variant?: string }) => <article data-testid="snapshot-card" data-variant={variant} />,
}));
vi.mock("../components/CurrentSessionCard", () => ({
  CurrentSessionCard: ({ variant }: { variant?: string }) => <article data-testid="session-card" data-variant={variant} />,
}));
vi.mock("../components/ProjectPulseCard", () => ({
  ProjectPulseCard: ({ variant }: { variant?: string }) => <article data-testid="pulse-card" data-variant={variant} />,
}));
vi.mock("../components/ProjectRuntimeStatusCard", () => ({
  ProjectRuntimeStatusCard: ({ variant }: { variant?: string }) => <article data-testid="runtime-card" data-variant={variant} />,
}));

import { ProjectOverviewPage } from "./ProjectOverviewPage";

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/projects/project-1"]}>
      <Routes>
        <Route path="/projects/:projectId" element={<ProjectOverviewPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("ProjectOverviewPage document workbench", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getProject.mockResolvedValue(project);
    mocks.useWorkbench.mockReturnValue(workbenchState());
  });

  it("uses one document flow with project navigation and no legacy card grid", async () => {
    const { container } = renderPage();

    await screen.findByRole("heading", { name: "ForgeOS" });
    const navigation = screen.getByRole("navigation", { name: "项目研发记忆" });
    expect(navigation).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "概览" })).toHaveAttribute("aria-current", "page");
    for (const label of ["研发历史", "成果与资料", "AI 项目上下文"]) {
      expect(screen.getByRole("link", { name: label })).toBeInTheDocument();
    }
    expect(screen.queryByText("更多")).not.toBeInTheDocument();

    expect(screen.getByTestId("pulse-card")).toHaveAttribute("data-variant", "document");
    expect(screen.getByTestId("session-card")).toHaveAttribute("data-variant", "inline");
    expect(screen.getByTestId("runtime-card")).toHaveAttribute("data-variant", "strip");
    expect(screen.getByTestId("snapshot-card")).toHaveAttribute("data-variant", "reference");
    expect(container.querySelector(".project-context__layout")).not.toBeInTheDocument();
    expect(container.querySelector(".project-context__rail")).not.toBeInTheDocument();
    expect(container.querySelector(".tool-grid")).not.toBeInTheDocument();
  });

  it("shows only the review action while a Pulse is pending", async () => {
    mocks.useWorkbench.mockReturnValue(workbenchState("pending_review"));
    renderPage();
    expect(await screen.findByRole("link", { name: "审核项目状态" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "同步 Codex 与 Git" })).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "项目设置" })).toBeInTheDocument();
  });

  it("shows the sync action after the Pulse is confirmed", async () => {
    mocks.useWorkbench.mockReturnValue(workbenchState("confirmed"));
    renderPage();
    const syncButton = await screen.findByRole("button", { name: "同步 Codex 与 Git" });
    expect(screen.queryByRole("link", { name: "审核项目状态" })).not.toBeInTheDocument();
    fireEvent.click(syncButton);
    expect(mocks.startSync).toHaveBeenCalledWith("manual");
  });
});
