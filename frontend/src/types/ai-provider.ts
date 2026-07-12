export type AiProvider = {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
  enabled: boolean;
  apiKeyConfigured: boolean;
  createdAt: string;
  updatedAt: string;
  version: number;
};

export type AiProviderConfigInput = {
  name: string;
  baseUrl: string;
  model: string;
};

export type UpdateAiProviderInput = AiProviderConfigInput & {
  expectedVersion: number;
};

export type AiMessageRole = "system" | "user" | "assistant";

export type AiMessage = {
  role: AiMessageRole;
  content: string;
};

export type AiTokenUsage = {
  promptTokens: number | null;
  completionTokens: number | null;
  totalTokens: number | null;
};

export type AiInvocationResult = {
  providerId: string;
  providerName: string;
  model: string;
  content: string;
  finishReason: string | null;
  usage: AiTokenUsage | null;
};
