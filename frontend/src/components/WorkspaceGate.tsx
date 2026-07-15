import { type FormEvent, type ReactNode, useEffect, useState } from "react";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import type { Workspace } from "../types/project";
import { useTranslation } from "react-i18next";

type WorkspaceGateProps = {
  children: (workspace: Workspace) => ReactNode;
};

type WorkspaceState =
  | { kind: "loading" }
  | { kind: "setup" }
  | { kind: "ready"; workspace: Workspace }
  | { kind: "error"; message: string };

export function WorkspaceGate({ children }: WorkspaceGateProps) {
  const { t } = useTranslation();
  const [state, setState] = useState<WorkspaceState>({ kind: "loading" });

  useEffect(() => {
    projectKernelApi
      .getCurrentWorkspace()
      .then((workspace) =>
        setState(workspace ? { kind: "ready", workspace } : { kind: "setup" }),
      )
      .catch((error: unknown) =>
        setState({ kind: "error", message: getErrorMessage(error) }),
      );
  }, []);

  if (state.kind === "loading") {
    return <FullPageMessage title={t("workspace.opening")} detail={t("workspace.reading")} />;
  }

  if (state.kind === "error") {
    return (
      <FullPageMessage
        title={t("workspace.unavailable")}
        detail={state.message}
        action={<button onClick={() => window.location.reload()}>{t("workspace.reload")}</button>}
      />
    );
  }

  if (state.kind === "setup") {
    return (
      <WorkspaceSetup
        onCreated={(workspace) => setState({ kind: "ready", workspace })}
      />
    );
  }

  return <>{children(state.workspace)}</>;
}

function WorkspaceSetup({ onCreated }: { onCreated: (workspace: Workspace) => void }) {
  const { t } = useTranslation();
  const [name, setName] = useState(() => t("workspace.defaultName"));
  const [timezone, setTimezone] = useState(
    Intl.DateTimeFormat().resolvedOptions().timeZone || "Asia/Shanghai",
  );
  const [error, setError] = useState<string>();
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    setError(undefined);

    try {
      const workspace = await projectKernelApi.createWorkspace({
        name: name.trim(),
        timezone: timezone.trim(),
      });
      onCreated(workspace);
    } catch (submissionError) {
      setError(getErrorMessage(submissionError));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="setup-shell">
      <form className="setup-card" onSubmit={handleSubmit}>
        <p className="eyebrow">{t("workspace.firstRun")}</p>
        <h1>{t("workspace.setupTitle")}</h1>
        <p className="setup-copy">{t("workspace.setupDescription")}</p>

        <label>
          {t("workspace.name")}
          <input
            autoFocus
            required
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        <label>
          {t("workspace.timezone")}
          <input
            required
            value={timezone}
            onChange={(event) => setTimezone(event.target.value)}
          />
        </label>

        {error ? <p className="form-error">{error}</p> : null}
        <button className="primary-button" disabled={submitting} type="submit">
          {submitting ? t("common.saving") : t("workspace.create")}
        </button>
      </form>
    </div>
  );
}

function FullPageMessage({
  title,
  detail,
  action,
}: {
  title: string;
  detail: string;
  action?: ReactNode;
}) {
  return (
    <div className="setup-shell">
      <section className="setup-card message-card">
        <h1>{title}</h1>
        <p className="setup-copy">{detail}</p>
        {action}
      </section>
    </div>
  );
}
