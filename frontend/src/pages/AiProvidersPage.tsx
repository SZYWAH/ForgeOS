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
import { useTranslation } from "react-i18next";
import { PageIntro } from "../components/ui/PlainLanguage";

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
  const { t } = useTranslation();
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
      setNotice(t("ai.serviceCreated"));
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
      setNotice(t("ai.serviceSaved"));
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
      setNotice(updated.enabled ? t("ai.serviceEnabled") : t("ai.serviceDisabled"));
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
      setNotice(t("ai.keySaved"));
    } catch (saveError) {
      setError(getErrorMessage(saveError));
    } finally {
      setSaving(false);
    }
  }

  async function handleClearKey() {
    if (!selected || !window.confirm(t("ai.keyDeleteConfirm"))) return;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const updated = await aiProviderApi.clearApiKey(selected.id);
      await load(updated.id);
      setNotice(t("ai.keyCleared"));
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

  if (loading) return <p className="loading-copy">{t("common.loading")}…</p>;

  return (
    <div className="page-stack ai-provider-page">
      <PageIntro title={t("ai.servicesTitle")} technicalTerm={t("terms.provider")} description={t("ai.servicesDescription")} primaryAction={<button className="primary-button" onClick={() => setShowCreate((value) => !value)} type="button">{showCreate ? t("common.cancel") : t("common.create")}</button>} />

      <div className="ai-boundary-banner">
        <strong>{t("ai.safetyTitle")}</strong>
        <span>{t("ai.safetyDescription")}</span>
      </div>
      {error ? <div className="error-banner provider-message">{error}</div> : null}
      {notice ? <div className="success-banner provider-message">{notice}</div> : null}

      {showCreate ? (
        <form className="panel provider-create-form" onSubmit={handleCreate}>
          <div className="panel-header"><div><h2>{t("common.create")} {t("ai.provider")}</h2><p>{t("ai.servicesDescription")}</p></div></div>
          <div className="provider-preset-row">
            <label>{t("ai.preset")}
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
            <label>{t("ai.apiKey")} <span>{t("common.optional")}</span>
              <input autoComplete="new-password" type="password" value={createApiKey} onChange={(event) => setCreateApiKey(event.target.value)} />
            </label>
          </div>
          <div className="form-actions provider-form-actions">
            <button className="primary-button" disabled={saving} type="submit">{saving ? t("common.saving") : t("common.create")}</button>
          </div>
        </form>
      ) : null}

      <div className="provider-workbench">
        <section className="panel provider-list-panel">
          <div className="panel-header"><div><h2>{t("ai.configuredServices")}</h2><p>{t("ai.serviceCount", { count: providers.length })}</p></div></div>
          {providers.length === 0 ? <p className="empty-state">{t("ai.noServices")}</p> : (
            <div className="provider-list record-list-pane">
              {providers.map((provider) => (
                <button
                  className={provider.id === selectedId ? "selected" : ""}
                  key={provider.id}
                  onClick={() => setSearchParams({ selected: provider.id })}
                  type="button"
                >
                  <span className={`provider-state provider-state--${provider.enabled ? "enabled" : "disabled"}`}>{provider.enabled ? t("common.enabled") : t("common.disabled")}</span>
                  <strong>{provider.name}</strong>
                  <small>{provider.model}</small>
                  <span className={`key-state ${provider.apiKeyConfigured ? "configured" : ""}`}>{provider.apiKeyConfigured ? t("ai.keyConfigured") : t("ai.keyMissing")}</span>
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
                    <span className={`provider-state provider-state--${selected.enabled ? "enabled" : "disabled"}`}>{selected.enabled ? t("common.enabled") : t("common.disabled")}</span>
                    <h2>{selected.name}</h2>
                    <p>v{selected.version} · {t("common.updatedAt")} {formatUpdatedAt(selected.updatedAt)}</p>
                  </div>
                  <button className={selected.enabled ? "danger-button" : "secondary-button"} disabled={saving} onClick={() => void handleToggle()} type="button">
                    {selected.enabled ? t("ai.disable") : t("ai.enable")}
                  </button>
                </div>
                <ProviderFields value={draft} onChange={setDraft} />
                <div className="form-actions provider-form-actions">
                  <button className="primary-button" disabled={saving} type="submit">{t("ai.saveConfiguration")}</button>
                </div>
              </form>

              <section className="panel provider-secret-panel">
                <div className="panel-header">
                  <div><h2>{t("ai.apiKey")}</h2><p>Windows Credential Manager</p></div>
                  <span className={`key-state ${selected.apiKeyConfigured ? "configured" : ""}`}>{selected.apiKeyConfigured ? t("ai.keyConfigured") : t("ai.keyMissing")}</span>
                </div>
                <form className="provider-secret-form" onSubmit={handleSaveKey}>
                  <input autoComplete="new-password" placeholder={t("ai.keyPlaceholder")} type="password" value={apiKeyDraft} onChange={(event) => setApiKeyDraft(event.target.value)} />
                  <button className="secondary-button" disabled={saving || !apiKeyDraft.trim()} type="submit">{t("ai.saveKey")}</button>
                  <button className="danger-button" disabled={saving || !selected.apiKeyConfigured} onClick={() => void handleClearKey()} type="button">{t("ai.clearKey")}</button>
                </form>
              </section>

              <form className="panel provider-invoke-panel" onSubmit={handleInvoke}>
                <div className="panel-header">
                  <div><h2>{t("ai.manualCall")}</h2><p>{t("ai.manualCallDescription")}</p></div>
                </div>
                <div className="provider-invoke-fields">
                  <label>{t("ai.systemMessage")} <span>{t("common.optional")}</span>
                    <textarea rows={3} value={systemMessage} onChange={(event) => setSystemMessage(event.target.value)} />
                  </label>
                  <label>{t("ai.userMessage")} <span className="required">*</span>
                    <textarea required rows={5} value={userMessage} onChange={(event) => setUserMessage(event.target.value)} placeholder={t("ai.testMessagePlaceholder")} />
                  </label>
                </div>
                <div className="form-actions provider-form-actions">
                  <span>{selected.enabled ? t("ai.requestReady") : t("ai.enableBeforeCall")}</span>
                  <button className="primary-button" disabled={invoking || !selected.enabled} type="submit">{invoking ? t("ai.invoking") : t("ai.invoke")}</button>
                </div>
                {result ? (
                  <div className="provider-result">
                    <header><strong>{t("ai.temporaryResponse")}</strong><span>{result.model} · {result.finishReason ?? t("common.unknown")}</span></header>
                    <pre className="document-scroll-region">{result.content}</pre>
                    {result.usage ? <small>{t("context.tokenCount", { count: result.usage.totalTokens ?? t("common.unknown") })}</small> : null}
                  </div>
                ) : null}
              </form>
            </>
          ) : (
            <section className="panel provider-detail-empty"><strong>{t("ai.selectService")}</strong><p>{t("ai.selectServiceDescription")}</p></section>
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
  const { t } = useTranslation();
  return (
    <div className="form-grid provider-fields">
      <label>{t("ai.name")} <span className="required">*</span>
        <input required maxLength={100} value={value.name} onChange={(event) => onChange({ ...value, name: event.target.value })} placeholder="OpenAI / DeepSeek / Local Ollama" />
      </label>
      <label>{t("ai.model")} <span className="required">*</span>
        <input required maxLength={200} value={value.model} onChange={(event) => onChange({ ...value, model: event.target.value })} placeholder={t("ai.modelPlaceholder")} />
      </label>
      <label className="span-2">{t("ai.baseUrl")} <span className="required">*</span>
        <input required value={value.baseUrl} onChange={(event) => onChange({ ...value, baseUrl: event.target.value })} placeholder="https://api.example.com/v1" />
        <small>{t("ai.baseUrlHelp")}</small>
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
