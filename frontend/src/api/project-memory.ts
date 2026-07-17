import { invoke } from "@tauri-apps/api/core";
import type {
  DevelopmentHistoryPage,
  ProjectMemoryOverview,
  ProjectMemorySummary,
  ProjectAiContext,
  SnapshotRepairDraft,
} from "../types/project-memory";

export const projectMemoryApi = {
  listSummaries: () =>
    invoke<ProjectMemorySummary[]>("list_project_memory_summaries"),
  overview: (projectId: string) =>
    invoke<ProjectMemoryOverview>("get_project_memory_overview", { projectId }),
  history: (projectId: string, cursor: string | null = null, limit = 20) =>
    invoke<DevelopmentHistoryPage>("list_development_history", {
      projectId,
      cursor,
      limit,
    }),
  buildAiContext: (projectId: string) =>
    invoke<ProjectAiContext>("build_project_ai_context", { projectId }),
  buildSnapshotRepairDraft: (projectId: string) =>
    invoke<SnapshotRepairDraft>("build_snapshot_repair_draft", { projectId }),
};
