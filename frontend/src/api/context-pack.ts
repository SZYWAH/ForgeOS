import { invoke } from "@tauri-apps/api/core";
import type {
  ContextAiInvocationResult,
  ContextExport,
  ContextTemplateType,
} from "../types/context-pack";

export const contextPackApi = {
  generate: (projectId: string, templateType: ContextTemplateType) =>
    invoke<ContextExport>("generate_context_pack", {
      projectId,
      input: { templateType },
    }),

  list: (projectId: string) =>
    invoke<ContextExport[]>("list_context_exports", { projectId }),

  get: (projectId: string, exportId: string) =>
    invoke<ContextExport>("get_context_export", { projectId, exportId }),

  invokeWithAi: (
    projectId: string,
    exportId: string,
    providerId: string,
    userPrompt: string,
  ) =>
    invoke<ContextAiInvocationResult>("invoke_context_with_ai", {
      projectId,
      exportId,
      input: { providerId, userPrompt },
    }),
};
