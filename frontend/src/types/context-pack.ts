import type { AiInvocationResult } from "./ai-provider";

export const CONTEXT_TEMPLATES = [
  "planning",
  "coding",
  "debugging",
  "review",
  "handoff",
] as const;

export type ContextTemplateType = (typeof CONTEXT_TEMPLATES)[number];
export type ContextExportStatus = "generated" | "sent_to_ai";

export type ContextExport = {
  id: string;
  projectId: string;
  templateType: ContextTemplateType;
  snapshotVersion: number | null;
  generatedFromSessionIds: string[];
  contentMarkdown: string;
  contentHash: string;
  status: ContextExportStatus;
  createdAt: string;
  version: number;
};

export type ContextAiInvocationResult = {
  contextExport: ContextExport;
  ai: AiInvocationResult;
};
