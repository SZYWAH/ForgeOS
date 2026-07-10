import { HashRouter, Route, Routes } from "react-router-dom";
import { AppLayout } from "./components/AppLayout";
import { WorkspaceGate } from "./components/WorkspaceGate";
import { DashboardPage } from "./pages/DashboardPage";
import { NotFoundPage } from "./pages/NotFoundPage";
import { ProjectOverviewPage } from "./pages/ProjectOverviewPage";
import { ProjectsPage } from "./pages/ProjectsPage";
import { SessionsPage } from "./pages/SessionsPage";
import { SnapshotPage } from "./pages/SnapshotPage";

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
              <Route path="*" element={<NotFoundPage />} />
            </Route>
          </Routes>
        </HashRouter>
      )}
    </WorkspaceGate>
  );
}

export default App;
