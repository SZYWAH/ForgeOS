import { invoke } from "@tauri-apps/api/core";
import type { AiCandidate, CreateAiCandidateInput } from "../types/ai-candidate";

export const aiCandidateApi = {
  create: (projectId: string, input: CreateAiCandidateInput) =>
    invoke<AiCandidate>("create_ai_candidate", { projectId, input }),

  list: (projectId: string) =>
    invoke<AiCandidate[]>("list_candidates", { projectId }),

  get: (projectId: string, candidateId: string) =>
    invoke<AiCandidate>("get_candidate", { projectId, candidateId }),

  accept: (projectId: string, candidateId: string, expectedVersion: number) =>
    invoke<AiCandidate>("accept_candidate", {
      projectId,
      candidateId,
      input: { expectedVersion },
    }),

  reject: (projectId: string, candidateId: string, expectedVersion: number) =>
    invoke<AiCandidate>("reject_candidate", {
      projectId,
      candidateId,
      input: { expectedVersion },
    }),

  archive: (projectId: string, candidateId: string, expectedVersion: number) =>
    invoke<AiCandidate>("archive_candidate", {
      projectId,
      candidateId,
      input: { expectedVersion },
    }),
};
