import { invoke } from "@tauri-apps/api/core";
import type {
  ProjectSnapshot,
  SnapshotContentInput,
  UpdateSnapshotInput,
} from "../types/snapshot";

export const snapshotKernelApi = {
  createSnapshot: (projectId: string, input: SnapshotContentInput) =>
    invoke<ProjectSnapshot>("create_snapshot", { projectId, input }),

  getCurrentSnapshot: (projectId: string) =>
    invoke<ProjectSnapshot | null>("get_current_snapshot", { projectId }),

  getSnapshotHistory: (projectId: string) =>
    invoke<ProjectSnapshot[]>("get_snapshot_history", { projectId }),

  updateSnapshot: (projectId: string, input: UpdateSnapshotInput) =>
    invoke<ProjectSnapshot>("update_snapshot", { projectId, input }),
};
