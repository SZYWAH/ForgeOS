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
    const { container, getByRole } = renderPage();

    await waitFor(() => {
      expect(container.querySelector(".provider-empty-surface")).toBeInTheDocument();
    });

    fireEvent.click(getByRole("button", { name: "创建 AI 服务" }));

    expect(container.querySelector(".provider-create-form")).toBeInTheDocument();
    expect(container.querySelector(".provider-empty-surface")).not.toBeInTheDocument();
    expect(container.querySelector(".provider-workbench--create-only")).toBeInTheDocument();
    expect(container.querySelectorAll(".app-button--primary")).toHaveLength(1);
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

  it("creates inside the workbench and returns to the selected service on cancel", async () => {
    mocks.list.mockResolvedValue([provider]);
    const { container, getByRole } = renderPage();

    await waitFor(() => {
      expect(container.querySelector(".provider-editor")).toBeInTheDocument();
    });

    fireEvent.click(getByRole("button", { name: "新建服务" }));
    expect(container.querySelector(".provider-list-panel")).toBeInTheDocument();
    expect(container.querySelector(".provider-create-form")).toBeInTheDocument();
    expect(container.querySelector(".provider-editor")).not.toBeInTheDocument();

    fireEvent.click(getByRole("button", { name: "取消" }));
    expect(container.querySelector(".provider-editor")).toBeInTheDocument();
    expect(container.querySelector(".provider-create-form")).not.toBeInTheDocument();
  });

  it("uses an environment variable for the DeepSeek preset and no secret for local presets", async () => {
    const { container, getByLabelText, getByRole, queryByLabelText, getByText } = renderPage();
    await waitFor(() => expect(container.querySelector(".provider-empty-surface")).toBeInTheDocument());
    fireEvent.click(getByRole("button", { name: "创建 AI 服务" }));

    const preset = getByLabelText("预设") as HTMLSelectElement;
    fireEvent.change(preset, { target: { value: "DeepSeek" } });
    expect((getByLabelText(/密钥来源/) as HTMLSelectElement).value).toBe("environment");
    expect((getByLabelText(/环境变量/) as HTMLInputElement).value).toBe("DEEPSEEK_API_KEY");
    expect(queryByLabelText("访问密钥")).not.toBeInTheDocument();
    expect(queryByLabelText("接口协议")).not.toBeInTheDocument();

    fireEvent.change(preset, { target: { value: "Ollama" } });
    expect(queryByLabelText("密钥来源")).not.toBeInTheDocument();
    expect(queryByLabelText("访问密钥")).not.toBeInTheDocument();
    expect(getByText("这个本地接口不需要访问密钥。")).toBeInTheDocument();
  });

  it("keeps custom technical controls accessible and aligns the credential checkbox", async () => {
    const { container, getByLabelText, getByRole } = renderPage();
    await waitFor(() => expect(container.querySelector(".provider-empty-surface")).toBeInTheDocument());
    fireEvent.click(getByRole("button", { name: "创建 AI 服务" }));

    expect(getByLabelText(/接口协议/)).toBeInTheDocument();
    expect(getByLabelText(/鉴权方式/)).toBeInTheDocument();
    const checkbox = getByLabelText(/调用时必须提供密钥/);
    expect(checkbox).toHaveAttribute("type", "checkbox");
    expect(checkbox.closest(".checkbox-field")).toBeInTheDocument();
    fireEvent.click(checkbox);
    expect(checkbox).toBeChecked();
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
