import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

type SystemStatus = {
  appName: string;
  stage: string;
  databaseConnected: boolean;
  workspaceCount: number;
  projectCount: number;
};

type StatusState =
  | { kind: "loading" }
  | { kind: "desktop"; value: SystemStatus }
  | { kind: "browser" };

function App() {
  const [status, setStatus] = useState<StatusState>({ kind: "loading" });

  useEffect(() => {
    invoke<SystemStatus>("get_system_status")
      .then((value) => setStatus({ kind: "desktop", value }))
      .catch(() => setStatus({ kind: "browser" }));
  }, []);

  return (
    <div className="app-shell">
      <header className="app-header">
        <span className="brand">ForgeOS</span>
        <span className="stage-badge">D0 · Engineering Foundation</span>
      </header>

      <main className="foundation-panel">
        <p className="eyebrow">Personal AI-Assisted Engineering Operating System</p>
        <h1>Hello ForgeOS</h1>
        <p className="summary">
          The desktop foundation is ready for a trusted engineering memory system.
          Business features intentionally begin after D0 approval.
        </p>

        <section className="status-grid" aria-label="Foundation status">
          <article>
            <h2>Frontend</h2>
            <p>React + TypeScript + Vite</p>
          </article>
          <article>
            <h2>Desktop</h2>
            <p>Tauri 2 + Rust</p>
          </article>
          <article>
            <h2>Database</h2>
            <p>{renderDatabaseStatus(status)}</p>
          </article>
        </section>
      </main>
    </div>
  );
}

function renderDatabaseStatus(status: StatusState) {
  if (status.kind === "loading") {
    return "Checking desktop foundation…";
  }

  if (status.kind === "browser") {
    return "Browser preview · desktop check unavailable";
  }

  const { databaseConnected, workspaceCount, projectCount } = status.value;
  return databaseConnected
    ? `SQLite connected · ${workspaceCount} workspaces · ${projectCount} projects`
    : "SQLite unavailable";
}

export default App;

