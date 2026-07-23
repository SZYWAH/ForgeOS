import { type FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { aiProviderApi } from "../api/ai-provider";
import { getErrorCode, getErrorMessage } from "../api/project-kernel";
import type {
  AiInvocationResult,
  AiProvider,
  AiProviderConnectionTest,
  AiProviderConfigInput,
} from "../types/ai-provider";
import { formatUpdatedAt } from "../utils/format";
import { useTranslation } from "react-i18next";
import { PageIntro } from "../components/ui/PlainLanguage";
import {
  AppButton,
  EmptyState,
  Notice,
  Surface,
} from "../components/ui/VisualSystem";

const EMPTY_CONFIG: AiProviderConfigInput = {
  name: "",
  baseUrl: "",
  model: "",
  protocol: "openai_chat_completions",
  authScheme: "bearer",
  secretSource: "credential_manager",
  secretRequired: false,
  environmentVariable: "",
  customAuthHeader: "",
  apiVersion: "",
};

const PRESETS = [
  { label: "Custom", config: EMPTY_CONFIG },
  { label: "OpenAI", config: { ...EMPTY_CONFIG, name: "OpenAI", baseUrl: "https://api.openai.com/v1", secretRequired: true } },
  { label: "DeepSeek", config: { ...EMPTY_CONFIG, name: "DeepSeek", baseUrl: "https://api.deepseek.com/v1", secretSource: "environment", secretRequired: true, environmentVariable: "DEEPSEEK_API_KEY" } },
  { label: "Anthropic Claude", config: { ...EMPTY_CONFIG, name: "Anthropic Claude", baseUrl: "https://api.anthropic.com/v1", protocol: "anthropic_messages" as const, authScheme: "x_api_key" as const, secretRequired: true, apiVersion: "2023-06-01" } },
  { label: "Azure OpenAI", config: { ...EMPTY_CONFIG, name: "Azure OpenAI", baseUrl: "", protocol: "azure_openai" as const, authScheme: "api_key" as const, secretRequired: true } },
  { label: "Gemini", config: { ...EMPTY_CONFIG, name: "Gemini", baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai", protocol: "openai_compatible" as const, secretRequired: true } },
  { label: "Mistral", config: { ...EMPTY_CONFIG, name: "Mistral", baseUrl: "https://api.mistral.ai/v1", protocol: "openai_compatible" as const, secretRequired: true } },
  { label: "Ollama", config: { ...EMPTY_CONFIG, name: "Ollama", baseUrl: "http://localhost:11434/v1", protocol: "openai_compatible" as const, authScheme: "none" as const, secretSource: "none" as const } },
  { label: "LM Studio", config: { ...EMPTY_CONFIG, name: "LM Studio", baseUrl: "http://localhost:1234/v1", protocol: "openai_compatible" as const, authScheme: "none" as const, secretSource: "none" as const } },
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
  const [connectionTest, setConnectionTest] = useState<AiProviderConnectionTest>();
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
    setConnectionTest(undefined);
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
      if (createDraft.secretSource === "credential_manager" && createApiKey.trim()) {
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

  async function handleTestConnection() {
    if (!selected) return;
    setInvoking(true);
    setError(undefined);
    setConnectionTest(undefined);
    try {
      setConnectionTest(await aiProviderApi.testConnection(selected.id));
      setNotice(t("ai.connectionSucceeded"));
    } catch (testError) {
      setError(getErrorMessage(testError));
    } finally {
      setInvoking(false);
    }
  }

  if (loading) return <p className="loading-copy">{t("common.loading")}…</p>;

  return (
    <div className="page-stack ai-provider-page">
      <PageIntro
        title={t("ai.servicesTitle")}
        technicalTerm={t("terms.provider")}
        description={t("ai.servicesDescription")}
        primaryAction={(
          <AppButton
            onClick={() => setShowCreate((value) => !value)}
            variant="primary"
          >
            {showCreate ? t("common.cancel") : t("common.create")}
          </AppButton>
        )}
      />

      <Notice className="ai-boundary-banner" title={t("ai.safetyTitle")} tone="info">
        {t("ai.safetyDescription")}
      </Notice>
      {error ? <Notice className="provider-message" tone="danger">{error}</Notice> : null}
      {notice ? <Notice className="provider-message" tone="success">{notice}</Notice> : null}

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
                  setCreateDraft({ ...value.config });
                }}
              >
                {PRESETS.map((item) => <option key={item.label}>{item.label}</option>)}
              </select>
            </label>
          </div>
          <ProviderFields value={createDraft} onChange={setCreateDraft} />
          {createDraft.secretSource === "credential_manager" ? <div className="provider-key-field">
            <label>{t("ai.apiKey")} <span>{t("common.optional")}</span>
              <input autoComplete="new-password" type="password" value={createApiKey} onChange={(event) => setCreateApiKey(event.target.value)} />
            </label>
          </div> : null}
          <div className="form-actions provider-form-actions">
            <AppButton loading={saving} type="submit" variant="primary">
              {saving ? t("common.saving") : t("common.create")}
            </AppButton>
          </div>
        </form>
      ) : null}

      {!showCreate && providers.length === 0 ? (
        <Surface
          className="provider-empty-surface"
          padding="compact"
          variant="outlined"
        >
          <EmptyState
            density="compact"
            description={t("ai.selectServiceDescription")}
            title={t("ai.noServices")}
          />
        </Surface>
      ) : null}

      {providers.length > 0 ? <div className="provider-workbench">
        <section className="panel provider-list-panel">
          <div className="panel-header"><div><h2>{t("ai.configuredServices")}</h2><p>{t("ai.serviceCount", { count: providers.length })}</p></div></div>
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
                <span className={`key-state ${provider.secretAvailable ? "configured" : ""}`}>{provider.secretAvailable ? t("ai.credentialAvailable") : t("ai.credentialMissing")}</span>
              </button>
            ))}
          </div>
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
                  <AppButton
                    disabled={saving}
                    onClick={() => void handleToggle()}
                    variant={selected.enabled ? "danger" : "secondary"}
                  >
                    {selected.enabled ? t("ai.disable") : t("ai.enable")}
                  </AppButton>
                </div>
                <ProviderFields value={draft} onChange={setDraft} />
                <div className="form-actions provider-form-actions">
                  <AppButton loading={saving} type="submit" variant="primary">
                    {t("ai.saveConfiguration")}
                  </AppButton>
                </div>
              </form>

              {selected.secretSource === "credential_manager" ? <section className="panel provider-secret-panel">
                <div className="panel-header">
                  <div><h2>{t("ai.apiKey")}</h2><p>Windows Credential Manager</p></div>
                  <span className={`key-state ${selected.apiKeyConfigured ? "configured" : ""}`}>{selected.apiKeyConfigured ? t("ai.keyConfigured") : t("ai.keyMissing")}</span>
                </div>
                <form className="provider-secret-form" onSubmit={handleSaveKey}>
                  <input autoComplete="new-password" placeholder={t("ai.keyPlaceholder")} type="password" value={apiKeyDraft} onChange={(event) => setApiKeyDraft(event.target.value)} />
                  <AppButton disabled={saving || !apiKeyDraft.trim()} type="submit" variant="secondary">{t("ai.saveKey")}</AppButton>
                  <AppButton disabled={saving || !selected.apiKeyConfigured} onClick={() => void handleClearKey()} variant="danger">{t("ai.clearKey")}</AppButton>
                </form>
              </section> : (
                <Notice tone={selected.secretAvailable ? "success" : "warning"} title={t("ai.credentialSource")}>{selected.secretSource === "environment" ? `${selected.environmentVariable ?? ""} · ${selected.secretAvailable ? t("ai.environmentAvailable") : t("ai.environmentMissing")}` : t("ai.noCredentialRequired")}</Notice>
              )}

              <section className="panel provider-secret-panel">
                <div className="panel-header"><div><h2>{t("ai.connectionTest")}</h2><p>{t("ai.connectionTestDescription")}</p></div></div>
                <div className="form-actions provider-form-actions">
                  {connectionTest ? <span>{connectionTest.model} · {connectionTest.latencyMs} ms</span> : <span>{t("ai.connectionTestBilling")}</span>}
                  <AppButton disabled={invoking} onClick={() => void handleTestConnection()} variant="secondary">{t("ai.testConnection")}</AppButton>
                </div>
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
                  <AppButton disabled={!selected.enabled} loading={invoking} type="submit" variant="primary">{invoking ? t("ai.invoking") : t("ai.invoke")}</AppButton>
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
      </div> : null}
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
      <label>{t("ai.protocol")} <span className="required">*</span>
        <select value={value.protocol} onChange={(event) => onChange({ ...value, protocol: event.target.value as AiProviderConfigInput["protocol"] })}>
          <option value="openai_chat_completions">OpenAI Chat Completions</option>
          <option value="openai_responses">OpenAI Responses</option>
          <option value="anthropic_messages">Anthropic Messages</option>
          <option value="azure_openai">Azure OpenAI v1</option>
          <option value="openai_compatible">OpenAI Compatible</option>
        </select>
      </label>
      <label>{t("ai.authScheme")} <span className="required">*</span>
        <select value={value.authScheme} onChange={(event) => {
          const authScheme = event.target.value as AiProviderConfigInput["authScheme"];
          onChange({ ...value, authScheme, secretSource: authScheme === "none" ? "none" : value.secretSource === "none" ? "credential_manager" : value.secretSource, secretRequired: authScheme === "none" ? false : value.secretRequired });
        }}>
          <option value="bearer">Bearer Token</option>
          <option value="api_key">api-key</option>
          <option value="x_api_key">x-api-key</option>
          <option value="custom_header">{t("ai.customAuthHeader")}</option>
          <option value="none">{t("ai.noAuthentication")}</option>
        </select>
      </label>
      <label>{t("ai.credentialSource")} <span className="required">*</span>
        <select disabled={value.authScheme === "none"} value={value.secretSource} onChange={(event) => onChange({ ...value, secretSource: event.target.value as AiProviderConfigInput["secretSource"] })}>
          <option value="credential_manager">Windows Credential Manager</option>
          <option value="environment">{t("ai.environmentVariable")}</option>
          <option value="none">{t("ai.noCredentialRequired")}</option>
        </select>
      </label>
      <label className="provider-checkbox"><input checked={value.secretRequired} disabled={value.authScheme === "none"} onChange={(event) => onChange({ ...value, secretRequired: event.target.checked })} type="checkbox" /> {t("ai.credentialRequired")}</label>
      {value.secretSource === "environment" ? <label className="span-2">{t("ai.environmentVariable")} <span className="required">*</span><input required maxLength={128} value={value.environmentVariable ?? ""} onChange={(event) => onChange({ ...value, environmentVariable: event.target.value })} placeholder="DEEPSEEK_API_KEY" /><small>{t("ai.environmentRestartHint")}</small></label> : null}
      {value.authScheme === "custom_header" ? <label className="span-2">{t("ai.customAuthHeader")} <span className="required">*</span><input required maxLength={128} value={value.customAuthHeader ?? ""} onChange={(event) => onChange({ ...value, customAuthHeader: event.target.value })} placeholder="X-API-Key" /></label> : null}
      {value.protocol === "anthropic_messages" ? <label className="span-2">{t("ai.apiVersion")}<input maxLength={100} value={value.apiVersion ?? ""} onChange={(event) => onChange({ ...value, apiVersion: event.target.value })} placeholder="2023-06-01" /></label> : null}
    </div>
  );
}

function configFrom(provider: AiProvider): AiProviderConfigInput {
  return { name: provider.name, baseUrl: provider.baseUrl, model: provider.model, protocol: provider.protocol, authScheme: provider.authScheme, secretSource: provider.secretSource, secretRequired: provider.secretRequired, environmentVariable: provider.environmentVariable ?? "", customAuthHeader: provider.customAuthHeader ?? "", apiVersion: provider.apiVersion ?? "" };
}

function normalize(value: AiProviderConfigInput): AiProviderConfigInput {
  return { ...value, name: value.name.trim(), baseUrl: value.baseUrl.trim(), model: value.model.trim(), environmentVariable: value.environmentVariable?.trim() || undefined, customAuthHeader: value.customAuthHeader?.trim() || undefined, apiVersion: value.apiVersion?.trim() || undefined };
}
