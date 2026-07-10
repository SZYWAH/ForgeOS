export type ProjectSnapshot = {
  id: string;
  projectId: string;
  version: number;
  currentGoal: string;
  statusSummary: string;
  achievements: string[];
  blockers: string[];
  nextSteps: string[];
  confirmedAt: string;
  createdAt: string;
  updatedAt: string;
};

export type SnapshotContentInput = {
  currentGoal: string;
  statusSummary: string;
  achievements: string[];
  blockers: string[];
  nextSteps: string[];
};

export type UpdateSnapshotInput = SnapshotContentInput & {
  expectedVersion: number;
};
