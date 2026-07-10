export const DEVELOPMENT_SESSION_STATUSES = [
  "open",
  "completed",
  "abandoned",
] as const;

export type DevelopmentSessionStatus =
  (typeof DEVELOPMENT_SESSION_STATUSES)[number];

export type DevelopmentSession = {
  id: string;
  projectId: string;
  status: DevelopmentSessionStatus;
  goal: string;
  notes: string;
  outcome: string;
  validation: string;
  handoff: string;
  startedAt: string;
  completedAt: string | null;
  createdAt: string;
  updatedAt: string;
  version: number;
};

export type StartSessionInput = {
  goal: string;
};

export type UpdateSessionInput = {
  goal: string;
  notes: string;
  expectedVersion: number;
};

export type CompleteSessionInput = {
  outcome: string;
  validation: string;
  handoff: string;
  expectedVersion: number;
};

export type AbandonSessionInput = {
  handoff: string;
  expectedVersion: number;
};
