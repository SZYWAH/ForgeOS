export const AI_CANDIDATE_TYPES = [
  "snapshot_update",
  "session_summary",
  "evidence_suggestion",
  "risk_finding",
  "general_note",
] as const;

export type AiCandidateType = (typeof AI_CANDIDATE_TYPES)[number];
export type AiCandidateStatus = "pending" | "accepted" | "rejected" | "archived";

export type AiCandidate = {
  id: string;
  projectId: string;
  candidateType: AiCandidateType;
  sourceContextExportId: string;
  title: string;
  contentMarkdown: string;
  status: AiCandidateStatus;
  version: number;
  createdAt: string;
  updatedAt: string;
};

export type CreateAiCandidateInput = {
  candidateType: AiCandidateType;
  sourceContextExportId: string;
  title: string;
  contentMarkdown: string;
};
