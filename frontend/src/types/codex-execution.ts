export type CodexExecutionStatus =
  | "pending"
  | "running"
  | "success"
  | "failed"
  | "cancelled";

export type CodexExecution = {
  id: string;
  proposalId: string;
  projectId: string;
  status: CodexExecutionStatus;
  startTime: string | null;
  endTime: string | null;
  outputSummary: string;
  validationResult: string;
  version: number;
  createdAt: string;
  updatedAt: string;
};

export type RecordCodexExecutionResultInput = {
  outputSummary: string;
  validationResult: string;
  expectedVersion: number;
};

export type CancelCodexExecutionInput = {
  outputSummary: string;
  expectedVersion: number;
};
