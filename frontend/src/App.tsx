import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "./components/AppLayout";
import { WorkspaceGate } from "./components/WorkspaceGate";
import { EvidencePage } from "./pages/EvidencePage";
import { NotFoundPage } from "./pages/NotFoundPage";
import { ProjectOverviewPage } from "./pages/ProjectOverviewPage";
import { ProjectsPage } from "./pages/ProjectsPage";
import { SessionsPage } from "./pages/SessionsPage";
import { SnapshotPage } from "./pages/SnapshotPage";
import { AiProvidersPage } from "./pages/AiProvidersPage";
import { ContextPackPage } from "./pages/ContextPackPage";
import { AiCandidatesPage } from "./pages/AiCandidatesPage";
import { RepositoriesPage } from "./pages/RepositoriesPage";
import { CodexTasksPage } from "./pages/CodexTasksPage";
import { CodexExecutionsPage } from "./pages/CodexExecutionsPage";
import { GitHubImportPage } from "./pages/GitHubImportPage";
import { ProjectPulsePage } from "./pages/ProjectPulsePage";
import { ProjectSettingsPage } from "./pages/ProjectSettingsPage";
import { AppSettingsPage } from "./pages/AppSettingsPage";

function App() {
  return (
    <WorkspaceGate>
      {(workspace) => (
        <HashRouter>
          <Routes>
            <Route element={<AppLayout workspace={workspace} />}>
              <Route index element={<Navigate replace to="/projects" />} />
              <Route path="projects" element={<ProjectsPage />} />
              <Route path="projects/:projectId" element={<ProjectOverviewPage />} />
              <Route path="projects/:projectId/pulse" element={<ProjectPulsePage />} />
              <Route path="projects/:projectId/settings" element={<ProjectSettingsPage />} />
              <Route path="projects/:projectId/snapshot" element={<SnapshotPage />} />
              <Route path="projects/:projectId/sessions" element={<SessionsPage />} />
              <Route path="projects/:projectId/evidence" element={<EvidencePage />} />
              <Route path="projects/:projectId/context" element={<ContextPackPage />} />
              <Route path="projects/:projectId/candidates" element={<AiCandidatesPage />} />
              <Route path="projects/:projectId/repositories" element={<RepositoriesPage />} />
              <Route path="projects/:projectId/codex-tasks" element={<CodexTasksPage />} />
              <Route path="projects/:projectId/codex-executions" element={<CodexExecutionsPage />} />
              <Route path="settings/ai-providers" element={<AiProvidersPage />} />
              <Route path="settings" element={<AppSettingsPage />} />
              <Route path="github-import" element={<GitHubImportPage />} />
              <Route path="*" element={<NotFoundPage />} />
            </Route>
          </Routes>
        </HashRouter>
      )}
    </WorkspaceGate>
  );
}

export default App;
