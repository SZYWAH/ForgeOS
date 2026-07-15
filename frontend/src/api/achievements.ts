import { invoke } from "@tauri-apps/api/core";
import type { AchievementInput, ProjectAchievement } from "../types/achievement";

export const achievementApi = {
  list: (projectId: string) =>
    invoke<ProjectAchievement[]>("list_project_achievements", { projectId }),
  get: (projectId: string, achievementId: string) =>
    invoke<ProjectAchievement>("get_project_achievement", { projectId, achievementId }),
  create: (projectId: string, input: AchievementInput) =>
    invoke<ProjectAchievement>("create_project_achievement", { projectId, input }),
  fromHistory: (projectId: string, historyId: string) =>
    invoke<ProjectAchievement>("create_achievement_draft_from_history", {
      projectId,
      historyId,
    }),
  update: (
    projectId: string,
    achievementId: string,
    expectedVersion: number,
    input: AchievementInput,
  ) =>
    invoke<ProjectAchievement>("update_project_achievement", {
      projectId,
      achievementId,
      expectedVersion,
      input,
    }),
  confirm: (projectId: string, achievementId: string, expectedVersion: number) =>
    invoke<ProjectAchievement>("confirm_project_achievement", {
      projectId,
      achievementId,
      expectedVersion,
    }),
  archive: (projectId: string, achievementId: string, expectedVersion: number) =>
    invoke<ProjectAchievement>("archive_project_achievement", {
      projectId,
      achievementId,
      expectedVersion,
    }),
};
