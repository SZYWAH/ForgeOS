import { invoke } from "@tauri-apps/api/core";
import type { CodexTaskProposal, CreateCodexTaskProposalInput } from "../types/codex-task-proposal";

export const codexTaskProposalApi = {
  create: (projectId: string, input: CreateCodexTaskProposalInput) => invoke<CodexTaskProposal>("create_proposal", { projectId, input }),
  list: (projectId: string) => invoke<CodexTaskProposal[]>("list_proposals", { projectId }),
  get: (projectId: string, proposalId: string) => invoke<CodexTaskProposal>("get_proposal", { projectId, proposalId }),
  submit: (projectId: string, proposalId: string, expectedVersion: number) => invoke<CodexTaskProposal>("submit_proposal", { projectId, proposalId, input: { expectedVersion } }),
  approve: (projectId: string, proposalId: string, expectedVersion: number) => invoke<CodexTaskProposal>("approve_proposal", { projectId, proposalId, input: { expectedVersion } }),
  reject: (projectId: string, proposalId: string, expectedVersion: number) => invoke<CodexTaskProposal>("reject_proposal", { projectId, proposalId, input: { expectedVersion } }),
  archive: (projectId: string, proposalId: string, expectedVersion: number) => invoke<CodexTaskProposal>("archive_proposal", { projectId, proposalId, input: { expectedVersion } }),
};
