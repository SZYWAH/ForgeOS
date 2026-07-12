import { invoke } from "@tauri-apps/api/core";
import type {
  AiInvocationResult,
  AiMessage,
  AiProvider,
  AiProviderConfigInput,
  UpdateAiProviderInput,
} from "../types/ai-provider";

export const aiProviderApi = {
  create: (input: AiProviderConfigInput) =>
    invoke<AiProvider>("create_ai_provider", { input }),

  list: () => invoke<AiProvider[]>("list_ai_providers"),

  get: (providerId: string) =>
    invoke<AiProvider>("get_ai_provider", { providerId }),

  update: (providerId: string, input: UpdateAiProviderInput) =>
    invoke<AiProvider>("update_ai_provider", { providerId, input }),

  setEnabled: (providerId: string, enabled: boolean, expectedVersion: number) =>
    invoke<AiProvider>("set_ai_provider_enabled", {
      providerId,
      input: { enabled, expectedVersion },
    }),

  setApiKey: (providerId: string, apiKey: string) =>
    invoke<AiProvider>("set_ai_provider_api_key", {
      providerId,
      input: { apiKey },
    }),

  clearApiKey: (providerId: string) =>
    invoke<AiProvider>("clear_ai_provider_api_key", { providerId }),

  invoke: (providerId: string, messages: AiMessage[]) =>
    invoke<AiInvocationResult>("invoke_ai_provider", {
      providerId,
      input: { messages },
    }),
};
