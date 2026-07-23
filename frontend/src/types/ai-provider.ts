export type AiProviderProtocol = "openai_chat_completions" | "openai_responses" | "anthropic_messages" | "azure_openai" | "openai_compatible";
export type AiAuthScheme = "bearer" | "api_key" | "x_api_key" | "custom_header" | "none";
export type AiSecretSource = "credential_manager" | "environment" | "none";

export type AiProvider = {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
  protocol: AiProviderProtocol;
  authScheme: AiAuthScheme;
  secretSource: AiSecretSource;
  secretRequired: boolean;
  environmentVariable: string | null;
  customAuthHeader: string | null;
  apiVersion: string | null;
  enabled: boolean;
  apiKeyConfigured: boolean;
  secretAvailable: boolean;
  createdAt: string;
  updatedAt: string;
  version: number;
};

export type AiProviderConfigInput = {
  name: string;
  baseUrl: string;
  model: string;
  protocol: AiProviderProtocol;
  authScheme: AiAuthScheme;
  secretSource: AiSecretSource;
  secretRequired: boolean;
  environmentVariable?: string;
  customAuthHeader?: string;
  apiVersion?: string;
};

export type AiProviderConnectionTest = {
  providerId: string;
  protocol: AiProviderProtocol;
  model: string;
  latencyMs: number;
  warningCodes: string[];
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
