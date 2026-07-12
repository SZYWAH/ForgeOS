export type CodexTaskProposalStatus = "draft" | "proposed" | "approved" | "rejected" | "archived";
export type CodexTaskProposal = {
  id: string; projectId: string; contextExportId: string; title: string; description: string;
  goal: string; nonGoals: string[]; allowedScope: string[]; forbiddenScope: string[];
  acceptanceCriteria: string[]; validationCommands: string[]; status: CodexTaskProposalStatus;
  version: number; createdAt: string; updatedAt: string;
};
export type CreateCodexTaskProposalInput = Omit<CodexTaskProposal, "id" | "projectId" | "status" | "version" | "createdAt" | "updatedAt">;
