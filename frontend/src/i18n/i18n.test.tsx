import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PageIntro } from "../components/ui/PlainLanguage";
import { AppPreferencesProvider, resolveLocale } from "../contexts/AppPreferencesProvider";
import { AppSettingsPage } from "../pages/AppSettingsPage";
import { enUS, zhCN } from "./resources";

const preferencesApi = vi.hoisted(() => ({
  get: vi.fn(),
  update: vi.fn(),
}));

vi.mock("../api/app-preferences", () => ({
  appPreferencesApi: preferencesApi,
}));

function keys(value: object, prefix = ""): string[] {
  return Object.entries(value).flatMap(([key, child]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    return child && typeof child === "object" ? keys(child as object, path) : [path];
  });
}

describe("application language settings", () => {
  beforeEach(() => {
    preferencesApi.get.mockReset();
    preferencesApi.update.mockReset();
    preferencesApi.get.mockResolvedValue({
      languageMode: "zh-CN",
      showTechnicalTerms: false,
      version: 1,
      createdAt: "2026-07-15T00:00:00Z",
      updatedAt: "2026-07-15T00:00:00Z",
    });
  });

  it("resolves system language to one of the two supported locales", () => {
    expect(resolveLocale("system", "zh-Hans-CN")).toBe("zh-CN");
    expect(resolveLocale("system", "de-DE")).toBe("en-US");
    expect(resolveLocale("en-US", "zh-CN")).toBe("en-US");
  });

  it("keeps Chinese and English resources structurally identical", () => {
    expect(keys(enUS).sort()).toEqual(keys(zhCN).sort());
  });

  it("applies a saved language immediately and persists with the current version", async () => {
    preferencesApi.update.mockResolvedValue({
      languageMode: "en-US",
      showTechnicalTerms: false,
      version: 2,
      createdAt: "2026-07-15T00:00:00Z",
      updatedAt: "2026-07-15T00:01:00Z",
    });

    render(<AppPreferencesProvider><AppSettingsPage /></AppPreferencesProvider>);
    await screen.findByRole("heading", { name: "应用设置" });
    fireEvent.click(screen.getByRole("radio", { name: "English" }));
    fireEvent.click(screen.getByRole("button", { name: "保存应用设置" }));

    await screen.findByRole("heading", { name: "Application settings" });
    expect(preferencesApi.update).toHaveBeenCalledWith({
      languageMode: "en-US",
      showTechnicalTerms: false,
      expectedVersion: 1,
    });
  });

  it("reloads preferences instead of overwriting a concurrent update", async () => {
    preferencesApi.update.mockRejectedValue({
      code: "app_preferences_version_conflict",
      message: "stale version",
      retryable: false,
    });

    render(<AppPreferencesProvider><AppSettingsPage /></AppPreferencesProvider>);
    await screen.findByRole("heading");
    fireEvent.click(screen.getByRole("radio", { name: "English" }));
    fireEvent.click(screen.getByRole("button", { name: "保存应用设置" }));

    await waitFor(() => expect(preferencesApi.get).toHaveBeenCalledTimes(2));
    expect(screen.getByRole("heading")).toHaveTextContent("应用设置");
    expect(screen.getByText("应用设置已经发生变化")).toBeInTheDocument();
    expect(screen.getByText("重新加载设置后再保存。")).toBeInTheDocument();
    expect(screen.getByText(/app_preferences_version_conflict/)).toBeInTheDocument();
  });

  it("shows an English term only when the Chinese preference enables it", async () => {
    preferencesApi.get.mockResolvedValue({
      languageMode: "zh-CN",
      showTechnicalTerms: true,
      version: 1,
      createdAt: "2026-07-15T00:00:00Z",
      updatedAt: "2026-07-15T00:00:00Z",
    });
    render(<AppPreferencesProvider><PageIntro title="当前状态" technicalTerm="Snapshot" description="说明" /></AppPreferencesProvider>);
    await waitFor(() => expect(screen.getByRole("heading")).toHaveTextContent("当前状态（Snapshot）"));
  });

  it("does not append term hints in the English interface", async () => {
    preferencesApi.get.mockResolvedValue({
      languageMode: "en-US",
      showTechnicalTerms: true,
      version: 1,
      createdAt: "2026-07-15T00:00:00Z",
      updatedAt: "2026-07-15T00:00:00Z",
    });
    render(<AppPreferencesProvider><PageIntro title="Current state" technicalTerm="Snapshot" description="Description" /></AppPreferencesProvider>);
    await waitFor(() => expect(screen.getByRole("heading")).toHaveTextContent(/^Current state$/));
  });
});
