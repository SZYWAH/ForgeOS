import { fireEvent, render, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  clearApiKey: vi.fn(),
  create: vi.fn(),
  invoke: vi.fn(),
  list: vi.fn(),
  setApiKey: vi.fn(),
  setEnabled: vi.fn(),
  testConnection: vi.fn(),
  update: vi.fn(),
}));

vi.mock("../api/ai-provider", () => ({
  aiProviderApi: mocks,
}));

vi.mock("../api/project-kernel", () => ({
  getErrorCode: () => "",
  getErrorMessage: (error: unknown) => String(error),
}));

vi.mock("../components/ui/PlainLanguage", () => ({
  PageIntro: ({
    title,
    description,
    primaryAction,
  }: {
    title: string;
    description: string;
    primaryAction?: React.ReactNode;
  }) => (
    <header className="page-intro">
      <h1>{title}</h1>
      <p>{description}</p>
      <div className="page-intro__actions">{primaryAction}</div>
    </header>
  ),
}));

import { AiProvidersPage } from "./AiProvidersPage";

const provider = {
  id: "provider-1",
  name: "OpenAI",
  baseUrl: "https://api.openai.com/v1",
  model: "gpt-test",
  protocol: "openai_chat_completions",
  authScheme: "bearer",
  secretSource: "credential_manager",
  secretRequired: true,
  environmentVariable: null,
  customAuthHeader: null,
  apiVersion: null,
  enabled: true,
  apiKeyConfigured: true,
  secretAvailable: true,
  createdAt: "2026-07-16T00:00:00Z",
  updatedAt: "2026-07-16T00:00:00Z",
  version: 1,
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/settings/ai-providers"]}>
      <AiProvidersPage />
    </MemoryRouter>,
  );
}

describe("AiProvidersPage visual composition", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.list.mockResolvedValue([]);
  });

  it("uses one compact empty surface instead of an empty list-detail workbench", async () => {
    const { container } = renderPage();

    await waitFor(() => {
      expect(container.querySelector(".provider-empty-surface")).toBeInTheDocument();
    });

    expect(container.querySelector(".empty-state--compact")).toBeInTheDocument();
    expect(container.querySelector(".provider-workbench")).not.toBeInTheDocument();
    expect(container.querySelector(".provider-detail-empty")).not.toBeInTheDocument();
    expect(container.querySelector(".notice--info")).toBeInTheDocument();
  });

  it("replaces the empty state with the create form while creating", async () => {
    const { container } = renderPage();

    await waitFor(() => {
      expect(container.querySelector(".provider-empty-surface")).toBeInTheDocument();
    });

    const createButton = container.querySelector<HTMLButtonElement>(
      ".page-intro__actions .app-button",
    );
    expect(createButton).not.toBeNull();
    fireEvent.click(createButton!);

    expect(container.querySelector(".provider-create-form")).toBeInTheDocument();
    expect(container.querySelector(".provider-empty-surface")).not.toBeInTheDocument();
    expect(container.querySelector(".provider-workbench")).not.toBeInTheDocument();
  });

  it("keeps the existing list-detail workbench when a provider exists", async () => {
    mocks.list.mockResolvedValue([provider]);
    const { container } = renderPage();

    await waitFor(() => {
      expect(container.querySelector(".provider-editor")).toBeInTheDocument();
    });

    expect(container.querySelector(".provider-workbench")).toBeInTheDocument();
    expect(container.querySelector(".provider-list-panel")).toBeInTheDocument();
    expect(container.querySelector(".provider-empty-surface")).not.toBeInTheDocument();
  });

  it("uses an environment variable for the DeepSeek preset and no secret for local presets", async () => {
    const { container } = renderPage();
    await waitFor(() => expect(container.querySelector(".provider-empty-surface")).toBeInTheDocument());
    fireEvent.click(container.querySelector<HTMLButtonElement>(".page-intro__actions .app-button")!);

    const preset = container.querySelector<HTMLSelectElement>(".provider-preset-row select")!;
    fireEvent.change(preset, { target: { value: "DeepSeek" } });
    let selects = container.querySelectorAll<HTMLSelectElement>(".provider-create-form select");
    expect(selects[3].value).toBe("environment");
    expect(container.querySelector<HTMLInputElement>('input[placeholder="DEEPSEEK_API_KEY"]')?.value).toBe("DEEPSEEK_API_KEY");
    expect(container.querySelector('.provider-key-field input[type="password"]')).not.toBeInTheDocument();

    fireEvent.change(preset, { target: { value: "Ollama" } });
    selects = container.querySelectorAll<HTMLSelectElement>(".provider-create-form select");
    expect(selects[2].value).toBe("none");
    expect(selects[3].value).toBe("none");
    expect(container.querySelector('.provider-key-field input[type="password"]')).not.toBeInTheDocument();
  });

  it("runs connection diagnostics only after an explicit click", async () => {
    mocks.list.mockResolvedValue([provider]);
    mocks.testConnection.mockResolvedValue({
      providerId: provider.id,
      protocol: provider.protocol,
      model: provider.model,
      latencyMs: 42,
      warningCodes: [],
    });
    const { getByRole, getByText } = renderPage();
    const button = await waitFor(() => getByRole("button", { name: "测试连接" }));
    expect(mocks.testConnection).not.toHaveBeenCalled();
    fireEvent.click(button);
    await waitFor(() => expect(mocks.testConnection).toHaveBeenCalledWith(provider.id));
    expect(getByText(/42 ms/)).toBeInTheDocument();
  });
});
