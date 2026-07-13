export const MANUAL_EVIDENCE_TYPES = [
  "document",
  "screenshot",
  "link",
  "test_result",
  "note",
  "artifact",
] as const;

export const EVIDENCE_TYPES = [
  ...MANUAL_EVIDENCE_TYPES,
  "git_commit",
  "git_status",
  "github_repository",
  "github_commit",
  "github_release",
] as const;

export const EVIDENCE_SOURCE_TYPES = [
  "local_file",
  "external_url",
  "manual_note",
  "github_api",
] as const;

export const VERIFICATION_STATUSES = [
  "unverified",
  "verified",
  "invalid",
] as const;

export const EVIDENCE_TARGET_TYPES = [
  "project",
  "snapshot",
  "session",
] as const;

export const EVIDENCE_RELATION_TYPES = ["supports", "relates_to"] as const;

export type EvidenceType = (typeof EVIDENCE_TYPES)[number];
export type EvidenceSourceType = (typeof EVIDENCE_SOURCE_TYPES)[number];
export type VerificationStatus = (typeof VERIFICATION_STATUSES)[number];
export type EvidenceTargetType = (typeof EVIDENCE_TARGET_TYPES)[number];
export type EvidenceRelationType = (typeof EVIDENCE_RELATION_TYPES)[number];

export type Evidence = {
  id: string;
  projectId: string;
  title: string;
  description: string;
  evidenceType: EvidenceType;
  sourceType: EvidenceSourceType;
  sourceUri: string;
  verificationStatus: VerificationStatus;
  createdAt: string;
  updatedAt: string;
  version: number;
};

export type EvidenceLink = {
  id: string;
  evidenceId: string;
  targetType: EvidenceTargetType;
  targetId: string;
  relationType: EvidenceRelationType;
};

export type EvidenceRecord = {
  evidence: Evidence;
  links: EvidenceLink[];
};

export type EvidenceContentInput = {
  title: string;
  description: string;
  evidenceType: EvidenceType;
  sourceType: EvidenceSourceType;
  sourceUri: string;
};

export type UpdateEvidenceInput = EvidenceContentInput & {
  expectedVersion: number;
};

export type ChangeEvidenceStatusInput = {
  expectedVersion: number;
};

export type LinkEvidenceInput = {
  targetType: EvidenceTargetType;
  targetId: string;
  relationType: EvidenceRelationType;
};
