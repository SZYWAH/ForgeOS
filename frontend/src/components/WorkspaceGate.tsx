import { type FormEvent, type ReactNode, useEffect, useState } from "react";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import type { Workspace } from "../types/project";

type WorkspaceGateProps = {
  children: (workspace: Workspace) => ReactNode;
};

type WorkspaceState =
  | { kind: "loading" }
  | { kind: "setup" }
  | { kind: "ready"; workspace: Workspace }
  | { kind: "error"; message: string };

export function WorkspaceGate({ children }: WorkspaceGateProps) {
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
    return <FullPageMessage title="正在打开 ForgeOS" detail="正在读取本地工作空间…" />;
  }

  if (state.kind === "error") {
    return (
      <FullPageMessage
        title="无法连接 ForgeOS 后端"
        detail={state.message}
        action={<button onClick={() => window.location.reload()}>重新加载</button>}
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
  const [name, setName] = useState("我的工程工作空间");
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
        <p className="eyebrow">首次运行</p>
        <h1>创建你的工程工作空间</h1>
        <p className="setup-copy">
          ForgeOS 当前只维护一个本地工作空间，用于隔离并组织你的软件项目身份。
        </p>

        <label>
          工作空间名称
          <input
            autoFocus
            required
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        <label>
          时区
          <input
            required
            value={timezone}
            onChange={(event) => setTimezone(event.target.value)}
          />
        </label>

        {error ? <p className="form-error">{error}</p> : null}
        <button className="primary-button" disabled={submitting} type="submit">
          {submitting ? "正在创建…" : "创建工作空间"}
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
