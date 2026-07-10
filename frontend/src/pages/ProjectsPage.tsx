import { type FormEvent, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import { ProjectTable } from "../components/ProjectTable";
import {
  LIFECYCLE_STAGES,
  PROJECT_STATUSES,
  type CreateProjectInput,
  type ProjectSummary,
} from "../types/project";
import { lifecycleLabel, statusLabel, toProjectSlug } from "../utils/format";

const INITIAL_FORM: CreateProjectInput = {
  slug: "",
  name: "",
  description: "",
  vision: "",
  targetUser: "",
  scope: "",
  nonGoals: "",
  techStack: "",
  status: "idea",
  lifecycleStage: "discovery",
  priority: 50,
};

export function ProjectsPage() {
  const navigate = useNavigate();
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [form, setForm] = useState<CreateProjectInput>(INITIAL_FORM);
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [listError, setListError] = useState<string>();
  const [formError, setFormError] = useState<string>();
  const [formOpen, setFormOpen] = useState(false);
  const [slugWasEdited, setSlugWasEdited] = useState(false);

  useEffect(() => {
    void loadProjects();
  }, []);

  async function loadProjects() {
    setLoading(true);
    setListError(undefined);

    try {
      setProjects(await projectKernelApi.listProjects());
    } catch (loadError) {
      setListError(getErrorMessage(loadError));
    } finally {
      setLoading(false);
    }
  }

  function updateName(name: string) {
    setForm((current) => ({
      ...current,
      name,
      slug: slugWasEdited ? current.slug : toProjectSlug(name),
    }));
  }

  async function handleCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    setFormError(undefined);

    try {
      const project = await projectKernelApi.createProject({
        ...form,
        name: form.name.trim(),
        slug: form.slug.trim(),
        description: form.description.trim(),
      });
      navigate(`/projects/${project.id}`);
    } catch (submissionError) {
      setFormError(getErrorMessage(submissionError));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">Project Registry</p>
          <h1>Projects</h1>
          <p>建立并维护当前 Workspace 中的软件项目身份。</p>
        </div>
        <button className="primary-button" onClick={() => setFormOpen((open) => !open)}>
          {formOpen ? "Close Form" : "New Project"}
        </button>
      </header>

      {formOpen ? (
        <section className="panel create-panel">
          <div className="panel-header">
            <div>
              <h2>Create Project</h2>
              <p>先建立稳定身份，产品与技术上下文可在 Overview 中补充。</p>
            </div>
          </div>
          <form className="project-form compact-form" onSubmit={handleCreate}>
            <div className="form-grid">
              <label>
                Name <span className="required">*</span>
                <input
                  autoFocus
                  required
                  value={form.name}
                  onChange={(event) => updateName(event.target.value)}
                  placeholder="CodeLens-Pro"
                />
              </label>
              <label>
                Slug <span className="required">*</span>
                <input
                  required
                  minLength={2}
                  maxLength={64}
                  pattern="[a-z0-9]+(?:-[a-z0-9]+)*"
                  value={form.slug}
                  onChange={(event) => {
                    setSlugWasEdited(true);
                    setForm((current) => ({ ...current, slug: event.target.value }));
                  }}
                  placeholder="codelens-pro"
                />
                <small>小写字母、数字与连字符</small>
              </label>
              <label className="span-2">
                Description
                <textarea
                  rows={3}
                  value={form.description}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      description: event.target.value,
                    }))
                  }
                  placeholder="项目解决什么问题？"
                />
              </label>
              <label>
                Status
                <select
                  value={form.status}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      status: event.target.value as CreateProjectInput["status"],
                    }))
                  }
                >
                  {PROJECT_STATUSES.filter((status) => status !== "archived").map(
                    (status) => (
                      <option key={status} value={status}>
                        {statusLabel(status)}
                      </option>
                    ),
                  )}
                </select>
              </label>
              <label>
                Lifecycle
                <select
                  value={form.lifecycleStage}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      lifecycleStage: event.target
                        .value as CreateProjectInput["lifecycleStage"],
                    }))
                  }
                >
                  {LIFECYCLE_STAGES.map((stage) => (
                    <option key={stage} value={stage}>
                      {lifecycleLabel(stage)}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Priority
                <input
                  max={100}
                  min={0}
                  type="number"
                  value={form.priority}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      priority: Number(event.target.value),
                    }))
                  }
                />
              </label>
            </div>
            {formError ? <div className="error-banner">{formError}</div> : null}
            <div className="form-actions">
              <button
                className="secondary-button"
                onClick={() => setFormOpen(false)}
                type="button"
              >
                Cancel
              </button>
              <button className="primary-button" disabled={submitting} type="submit">
                {submitting ? "Creating…" : "Create Project"}
              </button>
            </div>
          </form>
        </section>
      ) : null}

      <section className="panel">
        <div className="panel-header">
          <div>
            <h2>All Projects</h2>
            <p>{loading ? "Loading…" : `${projects.length} projects in this workspace`}</p>
          </div>
        </div>
        {listError ? (
          <div className="load-error-state">
            <div className="error-banner">{listError}</div>
            <button
              className="secondary-button"
              onClick={() => void loadProjects()}
              type="button"
            >
              Retry
            </button>
          </div>
        ) : null}
        {!loading && !listError ? (
          <ProjectTable
            projects={projects}
            emptyMessage="尚未创建项目。点击 New Project 开始。"
          />
        ) : loading ? (
          <p className="loading-copy">Loading projects…</p>
        ) : null}
      </section>
    </div>
  );
}
