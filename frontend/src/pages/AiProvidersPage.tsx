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
  CheckboxField,
  FormField,
  FormSection,
  SelectField,
} from "../components/ui/FormControls";
import {
  AppButton,
  EmptyState,
  Notice,
  StatusBadge,
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

type ProviderWorkbenchMode = "empty" | "detail" | "create";

export function AiProvidersPage() {
  const { t } = useTranslation();
  const [searchParams, setSearchParams] = useSearchParams();
  const [providers, setProviders] = useState<AiProvider[]>([]);
  const [draft, setDraft] = useState<AiProviderConfigInput>(EMPTY_CONFIG);
  const [createDraft, setCreateDraft] = useState<AiProviderConfigInput>(EMPTY_CONFIG);
  const [createApiKey, setCreateApiKey] = useState("");
  const [apiKeyDraft, setApiKeyDraft] = useState("");
  const [preset, setPreset] = useState("Custom");
  const [editPreset, setEditPreset] = useState("Custom");
  const [workbenchMode, setWorkbenchMode] = useState<ProviderWorkbenchMode>("empty");
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
      const selectedConfig = configFrom(selected);
      setDraft(selectedConfig);
      setEditPreset(findPresetName(selectedConfig));
      setWorkbenchMode("detail");
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
      setWorkbenchMode("detail");
      await load(created.id);
      setNotice(t("ai.serviceCreated"));
    } catch (saveError) {
      if (created) {
        setCreateDraft(EMPTY_CONFIG);
        setCreateApiKey("");
        setWorkbenchMode("detail");
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

  function beginCreate() {
    setCreateDraft(EMPTY_CONFIG);
    setCreateApiKey("");
    setPreset("Custom");
    setError(undefined);
    setNotice(undefined);
    setWorkbenchMode("create");
  }

  function cancelCreate() {
    setCreateDraft(EMPTY_CONFIG);
    setCreateApiKey("");
    setPreset("Custom");
    setWorkbenchMode(providers.length > 0 ? "detail" : "empty");
  }

  function applyCreatePreset(nextPreset: string) {
    const value = PRESETS.find((item) => item.label === nextPreset) ?? PRESETS[0];
    setPreset(value.label);
    setCreateDraft({ ...value.config });
    setCreateApiKey("");
  }

  function applyEditPreset(nextPreset: string) {
    const value = PRESETS.find((item) => item.label === nextPreset) ?? PRESETS[0];
    setEditPreset(value.label);
    if (value.label !== "Custom") {
      setDraft({
        ...value.config,
        model: draft.model,
      });
    }
  }

  if (loading) return <p className="loading-copy">{t("common.loading")}…</p>;

  return (
    <div className="page-stack ai-provider-page">
      <PageIntro
        title={t("ai.servicesTitle")}
        technicalTerm={t("terms.provider")}
        description={t("ai.servicesDescription")}
      />

      <Notice className="ai-boundary-banner" title={t("ai.safetyTitle")} tone="info">
        {t("ai.safetyDescription")}
      </Notice>
      {error ? <Notice className="provider-message" tone="danger">{error}</Notice> : null}
      {notice ? <Notice className="provider-message" tone="success">{notice}</Notice> : null}

      {workbenchMode === "empty" && providers.length === 0 ? (
        <Surface
          className="provider-empty-surface"
          padding="compact"
          variant="outlined"
        >
          <EmptyState
            action={(
              <AppButton onClick={beginCreate} variant="primary">
                {t("ai.createService")}
              </AppButton>
            )}
            density="compact"
            description={t("ai.selectServiceDescription")}
            title={t("ai.noServices")}
          />
        </Surface>
      ) : null}

      {workbenchMode !== "empty" || providers.length > 0 ? (
        <div className={`provider-workbench ${providers.length === 0 ? "provider-workbench--create-only" : ""}`}>
          {providers.length > 0 ? (
            <section className="panel provider-list-panel">
              <div className="panel-header provider-list-header">
                <div>
                  <h2>{t("ai.configuredServices")}</h2>
                  <p>{t("ai.serviceCount", { count: providers.length })}</p>
                </div>
                <AppButton onClick={beginCreate} size="compact" variant="quiet">
                  {t("ai.newService")}
                </AppButton>
              </div>
              <div className="provider-list record-list-pane">
                {providers.map((provider) => (
                  <button
                    aria-current={provider.id === selectedId && workbenchMode === "detail" ? "true" : undefined}
                    className={provider.id === selectedId && workbenchMode === "detail" ? "selected" : ""}
                    key={provider.id}
                    onClick={() => {
                      setSearchParams({ selected: provider.id });
                      setWorkbenchMode("detail");
                    }}
                    type="button"
                  >
                    <StatusBadge tone={provider.enabled ? "success" : "neutral"}>
                      {provider.enabled ? t("common.enabled") : t("common.disabled")}
                    </StatusBadge>
                    <strong>{provider.name}</strong>
                    <small>{provider.model}</small>
                    <span className="provider-list__meta">
                      {protocolLabel(provider.protocol)} · {provider.secretAvailable ? t("ai.credentialAvailable") : t("ai.credentialMissing")}
                    </span>
                  </button>
                ))}
              </div>
            </section>
          ) : null}

          <div className="provider-detail-stack">
            {workbenchMode === "create" ? (
              <form className="panel provider-create-form" onSubmit={handleCreate}>
                <div className="panel-header provider-editor-header">
                  <div>
                    <p className="eyebrow">{t("ai.newService")}</p>
                    <h2>{t("ai.createService")}</h2>
                    <p>{t("ai.createServiceDescription")}</p>
                  </div>
                </div>
                <ProviderFields
                  apiKey={createApiKey}
                  idPrefix="create-provider"
                  onApiKeyChange={setCreateApiKey}
                  onChange={setCreateDraft}
                  onPresetChange={applyCreatePreset}
                  presetName={preset}
                  value={createDraft}
                />
                <div className="form-actions provider-form-actions">
                  <AppButton onClick={cancelCreate} variant="secondary">
                    {t("common.cancel")}
                  </AppButton>
                  <AppButton loading={saving} type="submit" variant="primary">
                    {saving ? t("common.saving") : t("common.create")}
                  </AppButton>
                </div>
              </form>
            ) : selected ? (
              <>
                <form className="panel provider-editor" onSubmit={handleUpdate}>
                  <div className="panel-header provider-editor-header">
                    <div>
                      <StatusBadge tone={selected.enabled ? "success" : "neutral"}>
                        {selected.enabled ? t("common.enabled") : t("common.disabled")}
                      </StatusBadge>
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
                  <ProviderFields
                    idPrefix={`provider-${selected.id}`}
                    onChange={setDraft}
                    onPresetChange={applyEditPreset}
                    presetName={editPreset}
                    value={draft}
                  />
                  <div className="form-actions provider-form-actions">
                    <AppButton loading={saving} type="submit" variant="primary">
                      {t("ai.saveConfiguration")}
                    </AppButton>
                  </div>
                </form>

                {selected.secretSource === "credential_manager" ? (
                  <section className="panel provider-secret-panel">
                    <div className="panel-header">
                      <div>
                        <h2>{t("ai.apiKey")}</h2>
                        <p>{t("ai.credentialManagerDescription")}</p>
                      </div>
                      <StatusBadge tone={selected.apiKeyConfigured ? "success" : "warning"}>
                        {selected.apiKeyConfigured ? t("ai.keyConfigured") : t("ai.keyMissing")}
                      </StatusBadge>
                    </div>
                    <form className="provider-secret-form" onSubmit={handleSaveKey}>
                      <input
                        aria-label={t("ai.apiKey")}
                        autoComplete="new-password"
                        className="form-control"
                        placeholder={t("ai.keyPlaceholder")}
                        type="password"
                        value={apiKeyDraft}
                        onChange={(event) => setApiKeyDraft(event.target.value)}
                      />
                      <AppButton disabled={saving || !apiKeyDraft.trim()} type="submit" variant="secondary">
                        {t("ai.saveKey")}
                      </AppButton>
                      <AppButton disabled={saving || !selected.apiKeyConfigured} onClick={() => void handleClearKey()} variant="danger">
                        {t("ai.clearKey")}
                      </AppButton>
                    </form>
                  </section>
                ) : (
                  <Notice
                    tone={selected.secretAvailable ? "success" : "warning"}
                    title={t("ai.credentialSource")}
                  >
                    {selected.secretSource === "environment"
                      ? `${selected.environmentVariable ?? ""} · ${selected.secretAvailable ? t("ai.environmentAvailable") : t("ai.environmentMissing")}`
                      : t("ai.localNoCredential")}
                  </Notice>
                )}

                <section className="panel provider-diagnostics-panel">
                  <div className="panel-header">
                    <div>
                      <h2>{t("ai.connectionTest")}</h2>
                      <p>{t("ai.connectionTestDescription")}</p>
                    </div>
                    <AppButton disabled={invoking} onClick={() => void handleTestConnection()} variant="secondary">
                      {t("ai.testConnection")}
                    </AppButton>
                  </div>
                  <div className="provider-diagnostics-result">
                    {connectionTest ? (
                      <Notice tone="success">
                        {connectionTest.model} · {connectionTest.latencyMs} ms
                      </Notice>
                    ) : (
                      <p>{t("ai.connectionTestBilling")}</p>
                    )}
                  </div>
                </section>

                <details className="panel provider-tool-disclosure">
                  <summary>
                    <span>
                      <strong>{t("ai.manualCall")}</strong>
                      <small>{t("ai.manualCallDescription")}</small>
                    </span>
                  </summary>
                  <form className="provider-invoke-panel" onSubmit={handleInvoke}>
                    <div className="provider-invoke-fields">
                      <FormField id="provider-system-message" label={t("ai.systemMessage")}>
                        <textarea
                          className="form-control"
                          id="provider-system-message"
                          rows={3}
                          value={systemMessage}
                          onChange={(event) => setSystemMessage(event.target.value)}
                        />
                      </FormField>
                      <FormField id="provider-user-message" label={t("ai.userMessage")} required>
                        <textarea
                          className="form-control"
                          id="provider-user-message"
                          required
                          rows={5}
                          value={userMessage}
                          onChange={(event) => setUserMessage(event.target.value)}
                          placeholder={t("ai.testMessagePlaceholder")}
                        />
                      </FormField>
                    </div>
                    <div className="form-actions provider-form-actions">
                      <span>{selected.enabled ? t("ai.requestReady") : t("ai.enableBeforeCall")}</span>
                      <AppButton disabled={!selected.enabled} loading={invoking} type="submit" variant="secondary">
                        {invoking ? t("ai.invoking") : t("ai.invoke")}
                      </AppButton>
                    </div>
                    {result ? (
                      <div className="provider-result">
                        <header>
                          <strong>{t("ai.temporaryResponse")}</strong>
                          <span>{result.model} · {result.finishReason ?? t("common.unknown")}</span>
                        </header>
                        <pre className="document-scroll-region">{result.content}</pre>
                        {result.usage ? <small>{t("context.tokenCount", { count: result.usage.totalTokens ?? t("common.unknown") })}</small> : null}
                      </div>
                    ) : null}
                  </form>
                </details>
              </>
            ) : (
              <section className="panel provider-detail-empty">
                <strong>{t("ai.selectService")}</strong>
                <p>{t("ai.selectServiceDescription")}</p>
              </section>
            )}
          </div>
        </div>
      ) : null}
    </div>
  );
}

type ProviderFieldsProps = {
  idPrefix: string;
  value: AiProviderConfigInput;
  presetName: string;
  apiKey?: string;
  onApiKeyChange?: (value: string) => void;
  onPresetChange: (value: string) => void;
  onChange: (value: AiProviderConfigInput) => void;
};

function ProviderFields({
  idPrefix,
  value,
  presetName,
  apiKey,
  onApiKeyChange,
  onPresetChange,
  onChange,
}: ProviderFieldsProps) {
  const { t } = useTranslation();
  const isCustom = presetName === "Custom";
  const usesCredential = value.authScheme !== "none";
  const presetOptions = PRESETS.map((item) => ({
    label: item.label === "Custom" ? t("ai.customPreset") : item.label,
    value: item.label,
  }));

  return (
    <div className="provider-fields">
      <FormSection
        description={t("ai.serviceTypeDescription")}
        title={t("ai.serviceType")}
      >
        <div className="provider-field-grid provider-field-grid--preset">
          <SelectField
            id={`${idPrefix}-preset`}
            label={t("ai.preset")}
            onChange={onPresetChange}
            options={presetOptions}
            value={presetName}
          />
          <div className="provider-preset-summary">
            <span>{t("ai.technicalSummary")}</span>
            <strong>{protocolLabel(value.protocol)} · {authLabel(value.authScheme, t)}</strong>
            <small>{isCustom ? t("ai.customPresetDescription") : t("ai.presetFixedDescription")}</small>
          </div>
        </div>
      </FormSection>

      <FormSection
        description={t("ai.basicSettingsDescription")}
        title={t("ai.basicSettings")}
      >
        <div className="provider-field-grid">
          <FormField id={`${idPrefix}-name`} label={t("ai.name")} required>
            <input
              className="form-control"
              id={`${idPrefix}-name`}
              maxLength={100}
              onChange={(event) => onChange({ ...value, name: event.target.value })}
              placeholder="OpenAI / DeepSeek / Local Ollama"
              required
              value={value.name}
            />
          </FormField>
          <FormField id={`${idPrefix}-model`} label={t("ai.model")} required>
            <input
              className="form-control"
              id={`${idPrefix}-model`}
              maxLength={200}
              onChange={(event) => onChange({ ...value, model: event.target.value })}
              placeholder={t("ai.modelPlaceholder")}
              required
              value={value.model}
            />
          </FormField>
          <FormField
            className="span-2"
            description={t("ai.baseUrlHelp")}
            id={`${idPrefix}-base-url`}
            label={t("ai.baseUrl")}
            required
          >
            <input
              className="form-control"
              id={`${idPrefix}-base-url`}
              onChange={(event) => onChange({ ...value, baseUrl: event.target.value })}
              placeholder="https://api.example.com/v1"
              required
              value={value.baseUrl}
            />
          </FormField>
        </div>
      </FormSection>

      <FormSection
        description={t("ai.credentialSettingsDescription")}
        title={t("ai.credentialSettings")}
      >
        {!usesCredential ? (
          <Notice title={t("ai.noCredentialRequired")} tone="info">
            {t("ai.localNoCredential")}
          </Notice>
        ) : (
          <div className="provider-field-grid">
            <SelectField
              id={`${idPrefix}-credential-source`}
              label={t("ai.credentialSource")}
              onChange={(nextValue) => onChange({
                ...value,
                secretSource: nextValue as AiProviderConfigInput["secretSource"],
                secretRequired: nextValue === "none" ? false : value.secretRequired,
              })}
              options={[
                { value: "credential_manager", label: "Windows Credential Manager" },
                { value: "environment", label: t("ai.environmentVariable") },
                ...(isCustom
                  ? [{ value: "none", label: t("ai.noCredentialRequired") }]
                  : []),
              ]}
              required
              value={value.secretSource}
            />
            {isCustom && value.secretSource !== "none" ? (
              <CheckboxField
                checked={value.secretRequired}
                description={t("ai.credentialRequiredDescription")}
                id={`${idPrefix}-secret-required`}
                label={t("ai.credentialRequired")}
                onChange={(checked) => onChange({ ...value, secretRequired: checked })}
              />
            ) : isCustom ? (
              <div className="provider-readonly-setting">
                <span>{t("ai.credentialRequirement")}</span>
                <strong>{t("ai.noCredentialRequired")}</strong>
              </div>
            ) : (
              <div className="provider-readonly-setting">
                <span>{t("ai.credentialRequirement")}</span>
                <strong>{value.secretRequired ? t("ai.required") : t("common.optional")}</strong>
              </div>
            )}
            {value.secretSource === "environment" ? (
              <FormField
                className="span-2"
                description={t("ai.environmentRestartHint")}
                id={`${idPrefix}-environment-variable`}
                label={t("ai.environmentVariable")}
                required
              >
                <input
                  className="form-control"
                  id={`${idPrefix}-environment-variable`}
                  maxLength={128}
                  onChange={(event) => onChange({ ...value, environmentVariable: event.target.value })}
                  placeholder="DEEPSEEK_API_KEY"
                  required
                  value={value.environmentVariable ?? ""}
                />
              </FormField>
            ) : null}
            {value.secretSource === "credential_manager" && onApiKeyChange ? (
              <FormField
                className="span-2"
                description={t("ai.credentialManagerDescription")}
                id={`${idPrefix}-api-key`}
                label={t("ai.apiKey")}
              >
                <input
                  autoComplete="new-password"
                  className="form-control"
                  id={`${idPrefix}-api-key`}
                  onChange={(event) => onApiKeyChange(event.target.value)}
                  type="password"
                  value={apiKey ?? ""}
                />
              </FormField>
            ) : null}
          </div>
        )}
      </FormSection>

      <FormSection
        description={t("ai.technicalSettingsDescription")}
        title={t("ai.technicalSettings")}
      >
        {isCustom ? (
          <div className="provider-field-grid">
            <SelectField
              id={`${idPrefix}-protocol`}
              label={t("ai.protocol")}
              onChange={(nextValue) => onChange({
                ...value,
                protocol: nextValue as AiProviderConfigInput["protocol"],
              })}
              options={[
                { value: "openai_chat_completions", label: "OpenAI Chat Completions" },
                { value: "openai_responses", label: "OpenAI Responses" },
                { value: "anthropic_messages", label: "Anthropic Messages" },
                { value: "azure_openai", label: "Azure OpenAI v1" },
                { value: "openai_compatible", label: "OpenAI Compatible" },
              ]}
              required
              value={value.protocol}
            />
            <SelectField
              id={`${idPrefix}-auth-scheme`}
              label={t("ai.authScheme")}
              onChange={(nextValue) => {
                const authScheme = nextValue as AiProviderConfigInput["authScheme"];
                onChange({
                  ...value,
                  authScheme,
                  secretSource: authScheme === "none"
                    ? "none"
                    : value.secretSource === "none"
                      ? "credential_manager"
                      : value.secretSource,
                  secretRequired: authScheme === "none" ? false : value.secretRequired,
                });
              }}
              options={[
                { value: "bearer", label: "Bearer Token" },
                { value: "api_key", label: "api-key" },
                { value: "x_api_key", label: "x-api-key" },
                { value: "custom_header", label: t("ai.customAuthHeader") },
                { value: "none", label: t("ai.noAuthentication") },
              ]}
              required
              value={value.authScheme}
            />
            {value.authScheme === "custom_header" ? (
              <FormField
                className="span-2"
                id={`${idPrefix}-custom-auth-header`}
                label={t("ai.customAuthHeader")}
                required
              >
                <input
                  className="form-control"
                  id={`${idPrefix}-custom-auth-header`}
                  maxLength={128}
                  onChange={(event) => onChange({ ...value, customAuthHeader: event.target.value })}
                  placeholder="X-API-Key"
                  required
                  value={value.customAuthHeader ?? ""}
                />
              </FormField>
            ) : null}
            {value.protocol === "anthropic_messages" ? (
              <FormField
                className="span-2"
                id={`${idPrefix}-api-version`}
                label={t("ai.apiVersion")}
              >
                <input
                  className="form-control"
                  id={`${idPrefix}-api-version`}
                  maxLength={100}
                  onChange={(event) => onChange({ ...value, apiVersion: event.target.value })}
                  placeholder="2023-06-01"
                  value={value.apiVersion ?? ""}
                />
              </FormField>
            ) : null}
          </div>
        ) : (
          <div className="provider-technical-summary">
            <div>
              <span>{t("ai.protocol")}</span>
              <strong>{protocolLabel(value.protocol)}</strong>
            </div>
            <div>
              <span>{t("ai.authScheme")}</span>
              <strong>{authLabel(value.authScheme, t)}</strong>
            </div>
            <p>{t("ai.presetFixedDescription")}</p>
          </div>
        )}
      </FormSection>
    </div>
  );
}

function configFrom(provider: AiProvider): AiProviderConfigInput {
  return { name: provider.name, baseUrl: provider.baseUrl, model: provider.model, protocol: provider.protocol, authScheme: provider.authScheme, secretSource: provider.secretSource, secretRequired: provider.secretRequired, environmentVariable: provider.environmentVariable ?? "", customAuthHeader: provider.customAuthHeader ?? "", apiVersion: provider.apiVersion ?? "" };
}

function normalize(value: AiProviderConfigInput): AiProviderConfigInput {
  return { ...value, name: value.name.trim(), baseUrl: value.baseUrl.trim(), model: value.model.trim(), environmentVariable: value.environmentVariable?.trim() || undefined, customAuthHeader: value.customAuthHeader?.trim() || undefined, apiVersion: value.apiVersion?.trim() || undefined };
}

function findPresetName(value: AiProviderConfigInput) {
  return PRESETS.find((preset) => (
    preset.label !== "Custom"
    && preset.config.baseUrl === value.baseUrl
    && preset.config.protocol === value.protocol
    && preset.config.authScheme === value.authScheme
  ))?.label ?? "Custom";
}

function protocolLabel(protocol: AiProviderConfigInput["protocol"]) {
  switch (protocol) {
    case "openai_chat_completions":
      return "OpenAI Chat";
    case "openai_responses":
      return "OpenAI Responses";
    case "anthropic_messages":
      return "Anthropic Messages";
    case "azure_openai":
      return "Azure OpenAI";
    case "openai_compatible":
      return "OpenAI Compatible";
  }
}

function authLabel(
  authScheme: AiProviderConfigInput["authScheme"],
  t: (key: string) => string,
) {
  switch (authScheme) {
    case "bearer":
      return "Bearer";
    case "api_key":
      return "api-key";
    case "x_api_key":
      return "x-api-key";
    case "custom_header":
      return t("ai.customAuthHeader");
    case "none":
      return t("ai.noAuthentication");
  }
}
