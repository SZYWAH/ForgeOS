import { invoke } from "@tauri-apps/api/core";
import type {
  AbandonSessionInput,
  CompleteSessionInput,
  DevelopmentSession,
  StartSessionInput,
  UpdateSessionInput,
} from "../types/session";

export const sessionKernelApi = {
  startSession: (projectId: string, input: StartSessionInput) =>
    invoke<DevelopmentSession>("start_session", { projectId, input }),

  getCurrentSession: (projectId: string) =>
    invoke<DevelopmentSession | null>("get_current_session", { projectId }),

  listSessions: (projectId: string) =>
    invoke<DevelopmentSession[]>("list_sessions", { projectId }),

  updateSession: (
    projectId: string,
    sessionId: string,
    input: UpdateSessionInput,
  ) =>
    invoke<DevelopmentSession>("update_session", {
      projectId,
      sessionId,
      input,
    }),

  completeSession: (
    projectId: string,
    sessionId: string,
    input: CompleteSessionInput,
  ) =>
    invoke<DevelopmentSession>("complete_session", {
      projectId,
      sessionId,
      input,
    }),

  abandonSession: (
    projectId: string,
    sessionId: string,
    input: AbandonSessionInput,
  ) =>
    invoke<DevelopmentSession>("abandon_session", {
      projectId,
      sessionId,
      input,
    }),
};
