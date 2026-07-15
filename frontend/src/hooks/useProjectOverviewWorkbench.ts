import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getErrorCode, getErrorMessage } from "../api/project-kernel";
import { projectPulseApi } from "../api/project-pulse";
import { repositoryApi } from "../api/repository";
import type {
  ActivitySyncRun,
  ProjectPulse,
  SyncStage,
  SyncTrigger,
} from "../types/project-pulse";
import type { RepositoryRecord } from "../types/repository";

type SyncEventPayload = {
  projectId: string;
  syncRunId: string;
};

type WorkbenchOptions = {
  projectId: string;
  archived: boolean;
  enabled?: boolean;
  autoSync?: boolean;
};

export type ProjectActivityOverview = {
  pulse: ProjectPulse | null | undefined;
  run: ActivitySyncRun | null | undefined;
  repositories: RepositoryRecord[] | undefined;
  pulseError?: string;
  repositoryError?: string;
  syncError?: string;
  actionError?: string;
  loading: boolean;
  running: boolean;
  reload: (preserveContent?: boolean) => Promise<void>;
  startSync: (trigger?: SyncTrigger) => Promise<void>;
};

export const SYNC_STAGE_LABELS: Record<SyncStage, string> = {
  connecting: "连接 Codex",
  reading_codex: "读取 Codex",
  reading_git: "检查 Git",
  analyzing: "分析项目状态",
  persisting: "生成 Project Pulse",
};

export function useProjectOverviewWorkbench({
  projectId,
  archived,
  enabled = true,
  autoSync = true,
}: WorkbenchOptions): ProjectActivityOverview {
  const [pulse, setPulse] = useState<ProjectPulse | null>();
  const [run, setRun] = useState<ActivitySyncRun | null>();
  const [repositories, setRepositories] = useState<RepositoryRecord[]>();
  const [pulseError, setPulseError] = useState<string>();
  const [repositoryError, setRepositoryError] = useState<string>();
  const [syncError, setSyncError] = useState<string>();
  const [actionError, setActionError] = useState<string>();
  const generation = useRef(0);
  const autoStartedProject = useRef<string | undefined>(undefined);

  const reload = useCallback(async (preserveContent = false) => {
    if (!enabled) return;
    const current = ++generation.current;
    if (!preserveContent) {
      setPulse(undefined);
      setRun(undefined);
      setRepositories(undefined);
    }
    setPulseError(undefined);
    setRepositoryError(undefined);
    setSyncError(undefined);

    const [pulseResult, syncResult, repositoryResult] = await Promise.allSettled([
      projectPulseApi.latest(projectId),
      projectPulseApi.syncRuns(projectId),
      repositoryApi.list(projectId),
    ]);
    if (current !== generation.current) return;

    if (pulseResult.status === "fulfilled") {
      setPulse(pulseResult.value);
    } else {
      setPulse(null);
      setPulseError(getErrorMessage(pulseResult.reason));
    }

    if (syncResult.status === "fulfilled") {
      setRun(syncResult.value[0] ?? null);
    } else {
      setRun(null);
      setSyncError(getErrorMessage(syncResult.reason));
    }

    if (repositoryResult.status === "fulfilled") {
      setRepositories(repositoryResult.value);
    } else {
      setRepositories([]);
      setRepositoryError(getErrorMessage(repositoryResult.reason));
    }
  }, [enabled, projectId]);

  const startSync = useCallback(async (trigger: SyncTrigger = "manual") => {
    if (!enabled || archived) return;
    setActionError(undefined);
    try {
      const nextRun = await projectPulseApi.sync(projectId, trigger);
      setRun(nextRun);
    } catch (error) {
      setActionError(
        getErrorCode(error) === "repository_not_connected"
          ? "请先连接本地 Git 仓库，ForgeOS 才能按目录匹配 Codex 任务。"
          : getErrorMessage(error),
      );
    }
  }, [archived, enabled, projectId]);

  useEffect(() => {
    if (!enabled) return;
    void reload();

    const unlisteners = Promise.all([
      listen<SyncEventPayload>("project-pulse://sync-progress", (event) => {
        if (event.payload.projectId !== projectId) return;
        const current = generation.current;
        void projectPulseApi.syncRun(event.payload.syncRunId).then((nextRun) => {
          if (current === generation.current && nextRun.projectId === projectId) {
            setRun(nextRun);
          }
        }).catch((error) => {
          if (current === generation.current) setSyncError(getErrorMessage(error));
        });
      }),
      listen<SyncEventPayload>("project-pulse://sync-completed", (event) => {
        if (event.payload.projectId === projectId) void reload(true);
      }),
    ]);

    if (autoSync && autoStartedProject.current !== projectId && !archived) {
      autoStartedProject.current = projectId;
      void startSync("project_open");
    }

    return () => {
      generation.current += 1;
      void unlisteners.then((values) => values.forEach((unlisten) => unlisten()));
    };
  }, [archived, autoSync, enabled, projectId, reload, startSync]);

  useEffect(() => {
    if (!enabled || !run || !["queued", "running"].includes(run.status)) return;
    const timer = window.setInterval(() => {
      const current = generation.current;
      void projectPulseApi.syncRun(run.id).then((nextRun) => {
        if (current !== generation.current || nextRun.projectId !== projectId) return;
        setRun(nextRun);
        if (!["queued", "running"].includes(nextRun.status)) void reload(true);
      }).catch((error) => {
        if (current === generation.current) setSyncError(getErrorMessage(error));
      });
    }, 900);
    return () => window.clearInterval(timer);
  }, [enabled, projectId, reload, run]);

  return useMemo(() => ({
    pulse,
    run,
    repositories,
    pulseError,
    repositoryError,
    syncError,
    actionError,
    loading: pulse === undefined || run === undefined || repositories === undefined,
    running: run?.status === "queued" || run?.status === "running",
    reload,
    startSync,
  }), [actionError, pulse, pulseError, reload, repositories, repositoryError, run, startSync, syncError]);
}
