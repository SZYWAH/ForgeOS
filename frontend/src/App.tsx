import { HashRouter, Route, Routes } from "react-router-dom";
import { AppLayout } from "./components/AppLayout";
import { WorkspaceGate } from "./components/WorkspaceGate";
import { DashboardPage } from "./pages/DashboardPage";
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

function App() {
  return (
    <WorkspaceGate>
      {(workspace) => (
        <HashRouter>
          <Routes>
            <Route element={<AppLayout workspace={workspace} />}>
              <Route index element={<DashboardPage />} />
              <Route path="projects" element={<ProjectsPage />} />
              <Route path="projects/:projectId" element={<ProjectOverviewPage />} />
              <Route path="projects/:projectId/snapshot" element={<SnapshotPage />} />
              <Route path="projects/:projectId/sessions" element={<SessionsPage />} />
              <Route path="projects/:projectId/evidence" element={<EvidencePage />} />
              <Route path="projects/:projectId/context" element={<ContextPackPage />} />
              <Route path="projects/:projectId/candidates" element={<AiCandidatesPage />} />
              <Route path="projects/:projectId/repositories" element={<RepositoriesPage />} />
              <Route path="projects/:projectId/codex-tasks" element={<CodexTasksPage />} />
              <Route path="settings/ai-providers" element={<AiProvidersPage />} />
              <Route path="*" element={<NotFoundPage />} />
            </Route>
          </Routes>
        </HashRouter>
      )}
    </WorkspaceGate>
  );
}

export default App;
