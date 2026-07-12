import { invoke } from "@tauri-apps/api/core";
import type {
  ChangeEvidenceStatusInput,
  Evidence,
  EvidenceContentInput,
  EvidenceLink,
  EvidenceRecord,
  LinkEvidenceInput,
  UpdateEvidenceInput,
} from "../types/evidence";

export const evidenceKernelApi = {
  createEvidence: (projectId: string, input: EvidenceContentInput) =>
    invoke<Evidence>("create_evidence", { projectId, input }),

  listEvidences: (projectId: string) =>
    invoke<Evidence[]>("list_evidences", { projectId }),

  getEvidence: (projectId: string, evidenceId: string) =>
    invoke<EvidenceRecord>("get_evidence", { projectId, evidenceId }),

  updateEvidence: (
    projectId: string,
    evidenceId: string,
    input: UpdateEvidenceInput,
  ) => invoke<Evidence>("update_evidence", { projectId, evidenceId, input }),

  verifyEvidence: (
    projectId: string,
    evidenceId: string,
    input: ChangeEvidenceStatusInput,
  ) => invoke<Evidence>("verify_evidence", { projectId, evidenceId, input }),

  invalidateEvidence: (
    projectId: string,
    evidenceId: string,
    input: ChangeEvidenceStatusInput,
  ) =>
    invoke<Evidence>("invalidate_evidence", { projectId, evidenceId, input }),

  linkEvidence: (
    projectId: string,
    evidenceId: string,
    input: LinkEvidenceInput,
  ) => invoke<EvidenceLink>("link_evidence", { projectId, evidenceId, input }),
};
