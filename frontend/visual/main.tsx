import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";
import { mockIPC } from "@tauri-apps/api/mocks";
import { AppPreferencesProvider } from "../src/contexts/AppPreferencesProvider";
import { AiProvidersPage } from "../src/pages/AiProvidersPage";
import type { AiProvider } from "../src/types/ai-provider";
import "../src/i18n";
import "../src/styles.css";
import "./visual.css";

const now = "2026-07-28T09:00:00Z";
const configuredProviders: AiProvider[] = [
  {
    id: "provider-deepseek",
    name: "DeepSeek",
    baseUrl: "https://api.deepseek.com/v1",
    model: "deepseek-chat",
    protocol: "openai_chat_completions",
    authScheme: "bearer",
    secretSource: "environment",
    secretRequired: true,
    environmentVariable: "DEEPSEEK_API_KEY",
    customAuthHeader: null,
    apiVersion: null,
    enabled: true,
    apiKeyConfigured: false,
    secretAvailable: true,
    createdAt: now,
    updatedAt: now,
    version: 3,
  },
  {
    id: "provider-ollama",
    name: "Ollama",
    baseUrl: "http://localhost:11434/v1",
    model: "qwen3:8b",
    protocol: "openai_compatible",
    authScheme: "none",
    secretSource: "none",
    secretRequired: false,
    environmentVariable: null,
    customAuthHeader: null,
    apiVersion: null,
    enabled: false,
    apiKeyConfigured: false,
    secretAvailable: true,
    createdAt: now,
    updatedAt: now,
    version: 1,
  },
];

const scenario = new URLSearchParams(window.location.search).get("scenario") ?? "empty";
const providers = scenario === "detail" ? configuredProviders : [];

mockIPC((command) => {
  switch (command) {
    case "get_app_preferences":
      return {
        languageMode: "zh-CN",
        showTechnicalTerms: false,
        version: 1,
        createdAt: now,
        updatedAt: now,
      };
    case "list_ai_providers":
      return providers;
    default:
      throw new Error(`Unexpected visual fixture IPC command: ${command}`);
  }
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AppPreferencesProvider>
      <MemoryRouter initialEntries={["/ai-providers"]}>
        <div className="visual-harness content-scroll">
          <main className="content">
            <AiProvidersPage />
          </main>
        </div>
      </MemoryRouter>
    </AppPreferencesProvider>
  </StrictMode>,
);
