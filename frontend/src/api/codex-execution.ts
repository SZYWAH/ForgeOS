import { invoke } from "@tauri-apps/api/core";
import type {
  CancelCodexExecutionInput,
  CodexExecution,
  RecordCodexExecutionResultInput,
} from "../types/codex-execution";

export const codexExecutionApi = {
  create: (projectId: string, proposalId: string) =>
    invoke<CodexExecution>("create_codex_execution", { projectId, proposalId }),
  list: (projectId: string) =>
    invoke<CodexExecution[]>("list_codex_executions", { projectId }),
  get: (projectId: string, executionId: string) =>
    invoke<CodexExecution>("get_codex_execution", { projectId, executionId }),
  start: (projectId: string, executionId: string, expectedVersion: number) =>
    invoke<CodexExecution>("start_codex_execution", {
      projectId,
      executionId,
      input: { expectedVersion },
    }),
  complete: (
    projectId: string,
    executionId: string,
    input: RecordCodexExecutionResultInput,
  ) =>
    invoke<CodexExecution>("complete_codex_execution", {
      projectId,
      executionId,
      input,
    }),
  fail: (
    projectId: string,
    executionId: string,
    input: RecordCodexExecutionResultInput,
  ) =>
    invoke<CodexExecution>("fail_codex_execution", {
      projectId,
      executionId,
      input,
    }),
  cancel: (
    projectId: string,
    executionId: string,
    input: CancelCodexExecutionInput,
  ) =>
    invoke<CodexExecution>("cancel_codex_execution", {
      projectId,
      executionId,
      input,
    }),
};
