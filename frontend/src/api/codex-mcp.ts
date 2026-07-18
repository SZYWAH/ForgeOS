import { invoke } from "@tauri-apps/api/core";
import type { CodexMcpRegistrationStatus, ConfigureCodexMcpResult } from "../types/codex-mcp";

export const codexMcpApi = {
  get: () => invoke<CodexMcpRegistrationStatus>("get_codex_mcp_registration"),
  configure: (replaceExisting: boolean) =>
    invoke<ConfigureCodexMcpResult>("configure_codex_mcp", {
      input: { replaceExisting },
    }),
  remove: () => invoke<ConfigureCodexMcpResult>("remove_codex_mcp_registration"),
};
