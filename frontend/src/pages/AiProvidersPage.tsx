import { type FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { aiProviderApi } from "../api/ai-provider";
import { getErrorCode, getErrorMessage } from "../api/project-kernel";
import type {
  AiInvocationResult,
  AiProvider,
  AiProviderConfigInput,
} from "../types/ai-provider";
import { formatUpdatedAt } from "../utils/format";

const EMPTY_CONFIG: AiProviderConfigInput = {
  name: "",
  baseUrl: "",
  model: "",
};

const PRESETS = [
  { label: "Custom", name: "", baseUrl: "", model: "" },
  { label: "OpenAI", name: "OpenAI", baseUrl: "https://api.openai.com/v1", model: "" },
  { label: "DeepSeek", name: "DeepSeek", baseUrl: "https://api.deepseek.com/v1", model: "" },
  { label: "Ollama", name: "Ollama", baseUrl: "http://localhost:11434/v1", model: "" },
  { label: "LM Studio", name: "LM Studio", baseUrl: "http://localhost:1234/v1", model: "" },
  { label: "Azure OpenAI", name: "Azure OpenAI", baseUrl: "", model: "" },
] as const;

export function AiProvidersPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [providers, setProviders] = useState<AiProvider[]>([]);
  const [draft, setDraft] = useState<AiProviderConfigInput>(EMPTY_CONFIG);
  const [createDraft, setCreateDraft] = useState<AiProviderConfigInput>(EMPTY_CONFIG);
  const [createApiKey, setCreateApiKey] = useState("");
  const [apiKeyDraft, setApiKeyDraft] = useState("");
  const [preset, setPreset] = useState("Custom");
  const [showCreate, setShowCreate] = useState(false);
  const [systemMessage, setSystemMessage] = useState("");
  const [userMessage, setUserMessage] = useState("");
  const [result, setResult] = useState<AiInvocationResult>();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [invoking, setInvoking] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const generation = useRef(0);

  const selectedId = searchParams.get("selected") ?? "";
  const selected = useMemo(
    () => providers.find((provider) => provider.id === selectedId),
    [providers, selectedId],
  );

  const load = useCallback(async (preferId?: string) => {
    const requestGeneration = ++generation.current;
    setLoading(true);
    setError(undefined);
    try {
      const value = await aiProviderApi.list();
      if (requestGeneration !== generation.current) return;
      setProviders(value);
      const nextId =
        preferId ??
        searchParams.get("selected") ??
        value[0]?.id;
      if (nextId && value.some((provider) => provider.id === nextId)) {
        setSearchParams({ selected: nextId }, { replace: true });
      } else if (value.length === 0) {
        setSearchParams({}, { replace: true });
      }
    } catch (loadError) {
      if (requestGeneration === generation.current) {
        setError(getErrorMessage(loadError));
      }
    } finally {
      if (requestGeneration === generation.current) setLoading(false);
    }
  }, [searchParams, setSearchParams]);

  useEffect(() => {
    void load();
    return () => {
      generation.current += 1;
    };
    // Provider selection is local; the index loads once on entry.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (selected) {
      setDraft(configFrom(selected));
    }
    setApiKeyDraft("");
    setResult(undefined);
    setError(undefined);
    setNotice(undefined);
  }, [selected]);

  async function handleCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    let created: AiProvider | undefined;
    try {
      created = await aiProviderApi.create(normalize(createDraft));
      if (createApiKey.trim()) {
        created = await aiProviderApi.setApiKey(created.id, createApiKey.trim());
      }
      setCreateDraft(EMPTY_CONFIG);
      setCreateApiKey("");
      setPreset("Custom");
      setShowCreate(false);
      await load(created.id);
      setNotice("AI 服务已创建，默认保持停用，只有用户显式启用后才能调用。");
    } catch (saveError) {
      if (created) {
        setCreateDraft(EMPTY_CONFIG);
        setCreateApiKey("");
        setShowCreate(false);
        await load(created.id);
      }
      setError(getErrorMessage(saveError));
    } finally {
      setSaving(false);
    }
  }

  async function handleUpdate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selected) return;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const updated = await aiProviderApi.update(selected.id, {
        ...normalize(draft),
        expectedVersion: selected.version,
      });
      await load(updated.id);
      setNotice("AI 服务配置已保存。");
    } catch (saveError) {
      setError(getErrorMessage(saveError));
      if (getErrorCode(saveError) === "ai_provider_version_conflict") {
        await load(selected.id);
      }
    } finally {
      setSaving(false);
    }
  }

  async function handleToggle() {
    if (!selected) return;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const updated = await aiProviderApi.setEnabled(
        selected.id,
        !selected.enabled,
        selected.version,
      );
      await load(updated.id);
      setNotice(updated.enabled ? "AI 服务已启用。" : "AI 服务已停用，不会接受调用。");
    } catch (saveError) {
      setError(getErrorMessage(saveError));
    } finally {
      setSaving(false);
    }
  }

  async function handleSaveKey(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selected || !apiKeyDraft.trim()) return;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const updated = await aiProviderApi.setApiKey(selected.id, apiKeyDraft.trim());
      setApiKeyDraft("");
      await load(updated.id);
      setNotice("API Key 已写入 Windows Credential Manager，不会保存到 SQLite。");
    } catch (saveError) {
      setError(getErrorMessage(saveError));
    } finally {
      setSaving(false);
    }
  }

  async function handleClearKey() {
    if (!selected || !window.confirm("确认从 Windows Credential Manager 删除该 API Key？")) return;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const updated = await aiProviderApi.clearApiKey(selected.id);
      await load(updated.id);
      setNotice("API Key 已清除。");
    } catch (saveError) {
      setError(getErrorMessage(saveError));
    } finally {
      setSaving(false);
    }
  }

  async function handleInvoke(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selected || !userMessage.trim()) return;
    setInvoking(true);
    setError(undefined);
    setNotice(undefined);
    setResult(undefined);
    try {
      const messages = [
        ...(systemMessage.trim()
          ? [{ role: "system" as const, content: systemMessage.trim() }]
          : []),
        { role: "user" as const, content: userMessage.trim() },
      ];
      setResult(await aiProviderApi.invoke(selected.id, messages));
    } catch (invokeError) {
      setError(getErrorMessage(invokeError));
    } finally {
      setInvoking(false);
    }
  }

  if (loading) return <p className="loading-copy">正在加载 AI 服务…</p>;

  return (
    <div className="page-stack ai-provider-page">
      <header className="page-header ai-provider-page-header">
        <div>
          <p className="ai-provider-kicker">AI 服务基础</p>
          <h1>AI 服务配置</h1>
          <p>管理 OpenAI-compatible 端点。所有调用都必须由用户在此页面显式触发。</p>
        </div>
        <button className="primary-button" onClick={() => setShowCreate((value) => !value)} type="button">
          {showCreate ? "取消" : "添加 AI 服务"}
        </button>
      </header>

      <div className="ai-boundary-banner">
        <strong>安全边界</strong>
        <span>AI 服务返回内容不会自动写入项目、快照、会话、证据或其他数据库事实。</span>
      </div>
      {error ? <div className="error-banner provider-message">{error}</div> : null}
      {notice ? <div className="success-banner provider-message">{notice}</div> : null}

      {showCreate ? (
        <form className="panel provider-create-form" onSubmit={handleCreate}>
          <div className="panel-header"><div><h2>新建 AI 服务</h2><p>新服务默认处于停用状态。</p></div></div>
          <div className="provider-preset-row">
            <label>预设
              <select
                value={preset}
                onChange={(event) => {
                  const value = PRESETS.find((item) => item.label === event.target.value) ?? PRESETS[0];
                  setPreset(value.label);
                  setCreateDraft({ name: value.name, baseUrl: value.baseUrl, model: value.model });
                }}
              >
                {PRESETS.map((item) => <option key={item.label}>{item.label}</option>)}
              </select>
            </label>
          </div>
          <ProviderFields value={createDraft} onChange={setCreateDraft} />
          <div className="provider-key-field">
            <label>API Key <span>本地服务可不填</span>
              <input autoComplete="new-password" type="password" value={createApiKey} onChange={(event) => setCreateApiKey(event.target.value)} />
            </label>
          </div>
          <div className="form-actions provider-form-actions">
            <button className="primary-button" disabled={saving} type="submit">{saving ? "正在创建…" : "创建 AI 服务"}</button>
          </div>
        </form>
      ) : null}

      <div className="provider-workbench">
        <section className="panel provider-list-panel">
          <div className="panel-header"><div><h2>已配置服务</h2><p>{providers.length} 个服务</p></div></div>
          {providers.length === 0 ? <p className="empty-state">尚未配置 AI 服务。</p> : (
            <div className="provider-list">
              {providers.map((provider) => (
                <button
                  className={provider.id === selectedId ? "selected" : ""}
                  key={provider.id}
                  onClick={() => setSearchParams({ selected: provider.id })}
                  type="button"
                >
                  <span className={`provider-state provider-state--${provider.enabled ? "enabled" : "disabled"}`}>{provider.enabled ? "已启用" : "已停用"}</span>
                  <strong>{provider.name}</strong>
                  <small>{provider.model}</small>
                  <span className={`key-state ${provider.apiKeyConfigured ? "configured" : ""}`}>{provider.apiKeyConfigured ? "密钥已配置" : "未配置密钥"}</span>
                </button>
              ))}
            </div>
          )}
        </section>

        <div className="provider-detail-stack">
          {selected ? (
            <>
              <form className="panel provider-editor" onSubmit={handleUpdate}>
                <div className="panel-header provider-editor-header">
                  <div>
                    <span className={`provider-state provider-state--${selected.enabled ? "enabled" : "disabled"}`}>{selected.enabled ? "已启用" : "已停用"}</span>
                    <h2>{selected.name}</h2>
                    <p>v{selected.version} · 更新于 {formatUpdatedAt(selected.updatedAt)}</p>
                  </div>
                  <button className={selected.enabled ? "danger-button" : "secondary-button"} disabled={saving} onClick={() => void handleToggle()} type="button">
                    {selected.enabled ? "停用" : "启用"}
                  </button>
                </div>
                <ProviderFields value={draft} onChange={setDraft} />
                <div className="form-actions provider-form-actions">
                  <button className="primary-button" disabled={saving} type="submit">保存配置</button>
                </div>
              </form>

              <section className="panel provider-secret-panel">
                <div className="panel-header">
                  <div><h2>API Key</h2><p>存储在 Windows Credential Manager；前端无法读取已有明文。</p></div>
                  <span className={`key-state ${selected.apiKeyConfigured ? "configured" : ""}`}>{selected.apiKeyConfigured ? "已配置" : "未配置"}</span>
                </div>
                <form className="provider-secret-form" onSubmit={handleSaveKey}>
                  <input autoComplete="new-password" placeholder="输入新的 API Key" type="password" value={apiKeyDraft} onChange={(event) => setApiKeyDraft(event.target.value)} />
                  <button className="secondary-button" disabled={saving || !apiKeyDraft.trim()} type="submit">保存密钥</button>
                  <button className="danger-button" disabled={saving || !selected.apiKeyConfigured} onClick={() => void handleClearKey()} type="button">清除密钥</button>
                </form>
              </section>

              <form className="panel provider-invoke-panel" onSubmit={handleInvoke}>
                <div className="panel-header">
                  <div><h2>手动调用</h2><p>仅用于显式连接验证。提示词与响应不会保存。</p></div>
                </div>
                <div className="provider-invoke-fields">
                  <label>系统消息 <span>可选</span>
                    <textarea rows={3} value={systemMessage} onChange={(event) => setSystemMessage(event.target.value)} />
                  </label>
                  <label>用户消息 <span className="required">*</span>
                    <textarea required rows={5} value={userMessage} onChange={(event) => setUserMessage(event.target.value)} placeholder="发送一条最小连接测试消息" />
                  </label>
                </div>
                <div className="form-actions provider-form-actions">
                  <span>{selected.enabled ? "请求将发送到已配置的端点。" : "请先启用 AI 服务再调用。"}</span>
                  <button className="primary-button" disabled={invoking || !selected.enabled} type="submit">{invoking ? "正在调用…" : "调用 AI 服务"}</button>
                </div>
                {result ? (
                  <div className="provider-result">
                    <header><strong>临时响应</strong><span>{result.model} · {result.finishReason ?? "未知"}</span></header>
                    <pre>{result.content}</pre>
                    {result.usage ? <small>Token 数：{result.usage.totalTokens ?? "未知"}</small> : null}
                  </div>
                ) : null}
              </form>
            </>
          ) : (
            <section className="panel provider-detail-empty"><strong>选择或创建 AI 服务</strong><p>配置端点、模型、密钥和启停状态。</p></section>
          )}
        </div>
      </div>
    </div>
  );
}

type ProviderFieldsProps = {
  value: AiProviderConfigInput;
  onChange: (value: AiProviderConfigInput) => void;
};

function ProviderFields({ value, onChange }: ProviderFieldsProps) {
  return (
    <div className="form-grid provider-fields">
      <label>名称 <span className="required">*</span>
        <input required maxLength={100} value={value.name} onChange={(event) => onChange({ ...value, name: event.target.value })} placeholder="OpenAI / DeepSeek / Local Ollama" />
      </label>
      <label>模型 <span className="required">*</span>
        <input required maxLength={200} value={value.model} onChange={(event) => onChange({ ...value, model: event.target.value })} placeholder="模型 ID 或部署名称" />
      </label>
      <label className="span-2">Base URL <span className="required">*</span>
        <input required value={value.baseUrl} onChange={(event) => onChange({ ...value, baseUrl: event.target.value })} placeholder="https://api.example.com/v1" />
        <small>可填写 API 根路径或完整 `/chat/completions` 地址。</small>
      </label>
    </div>
  );
}

function configFrom(provider: AiProvider): AiProviderConfigInput {
  return { name: provider.name, baseUrl: provider.baseUrl, model: provider.model };
}

function normalize(value: AiProviderConfigInput): AiProviderConfigInput {
  return { name: value.name.trim(), baseUrl: value.baseUrl.trim(), model: value.model.trim() };
}
