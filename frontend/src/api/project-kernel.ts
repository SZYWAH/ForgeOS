import { invoke } from "@tauri-apps/api/core";
import i18n from "../i18n";
import { localizedErrorText } from "../utils/appError";
import type {
  CreateProjectInput,
  CreateWorkspaceInput,
  Project,
  ProjectSummary,
  UpdateProjectInput,
  Workspace,
} from "../types/project";

export const projectKernelApi = {
  getCurrentWorkspace: () =>
    invoke<Workspace | null>("get_current_workspace"),

  createWorkspace: (input: CreateWorkspaceInput) =>
    invoke<Workspace>("create_workspace", { input }),

  listProjects: () => invoke<ProjectSummary[]>("list_projects"),

  createProject: (input: CreateProjectInput) =>
    invoke<Project>("create_project", { input }),

  getProject: (projectId: string) =>
    invoke<Project>("get_project", { projectId }),

  updateProject: (projectId: string, input: UpdateProjectInput) =>
    invoke<Project>("update_project", { projectId, input }),

  archiveProject: (projectId: string, expectedVersion: number) =>
    invoke<Project>("archive_project", { projectId, expectedVersion }),
};

export function getErrorMessage(error: unknown): string {
  if (error && typeof error === "object" && "code" in error) {
    return localizedErrorText(error, i18n.t);
  }
  if (typeof error === "string") {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  if (error && typeof error === "object" && "message" in error) {
    return String(error.message);
  }

  return i18n.t("errors.unknownTitle");
}

export function getErrorCode(error: unknown): string | undefined {
  if (error && typeof error === "object" && "code" in error) {
    return String(error.code);
  }

  return undefined;
}
