import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AppPreferencesProvider } from "../contexts/AppPreferencesProvider";
import { AppSettingsPage } from "./AppSettingsPage";

const preferencesApi = vi.hoisted(() => ({ get: vi.fn(), update: vi.fn() }));
const codexMcpApi = vi.hoisted(() => ({ get: vi.fn(), configure: vi.fn(), remove: vi.fn() }));

vi.mock("../api/app-preferences", () => ({ appPreferencesApi: preferencesApi }));
vi.mock("../api/codex-mcp", () => ({ codexMcpApi }));

const notConfigured = {
  state: "not_configured",
  codexVersion: "codex-cli 0.144.5",
  expectedCommand: "C:\\Program Files\\ForgeOS\\forgeos-mcp.exe",
  configuredCommand: null,
  enabled: false,
  canConfigure: true,
};

describe("Codex MCP application settings", () => {
  beforeEach(() => {
    preferencesApi.get.mockReset();
    preferencesApi.update.mockReset();
    codexMcpApi.get.mockReset();
    codexMcpApi.configure.mockReset();
    codexMcpApi.remove.mockReset();
    preferencesApi.get.mockResolvedValue({
      languageMode: "zh-CN",
      showTechnicalTerms: false,
      version: 1,
      createdAt: "2026-07-18T00:00:00Z",
      updatedAt: "2026-07-18T00:00:00Z",
    });
  });

  it("connects Codex only after an explicit click", async () => {
    codexMcpApi.get
      .mockResolvedValueOnce(notConfigured)
      .mockResolvedValue({ ...notConfigured, state: "configured", configuredCommand: notConfigured.expectedCommand, enabled: true });
    codexMcpApi.configure.mockResolvedValue({ changed: true, restartRequired: true, backupCreated: true });

    render(<AppPreferencesProvider><AppSettingsPage /></AppPreferencesProvider>);
    const connect = await screen.findByRole("button", { name: "连接 Codex" });
    expect(codexMcpApi.configure).not.toHaveBeenCalled();
    fireEvent.click(connect);

    await waitFor(() => expect(codexMcpApi.configure).toHaveBeenCalledWith(false));
    expect(await screen.findByText(/请重启 Codex/)).toBeInTheDocument();
  });

  it("requires confirmation before replacing a conflicting entry", async () => {
    codexMcpApi.get.mockResolvedValue({
      ...notConfigured,
      state: "conflict",
      configuredCommand: "C:\\Other\\server.exe",
    });
    codexMcpApi.configure.mockResolvedValue({ changed: true, restartRequired: true, backupCreated: true });
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);

    render(<AppPreferencesProvider><AppSettingsPage /></AppPreferencesProvider>);
    fireEvent.click(await screen.findByRole("button", { name: "替换并连接" }));

    expect(confirm).toHaveBeenCalledOnce();
    await waitFor(() => expect(codexMcpApi.configure).toHaveBeenCalledWith(true));
    confirm.mockRestore();
  });
});
