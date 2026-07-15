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
import { useTranslation } from "react-i18next";
import { PageIntro } from "../components/ui/PlainLanguage";

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
  const { t } = useTranslation();
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
      <PageIntro title={t("projects.title")} description={t("projects.description")} primaryAction={<button className="primary-button" onClick={() => setFormOpen((open) => !open)}>{formOpen ? t("common.close") : t("projects.createProject")}</button>} />

      {formOpen ? (
        <section className="panel create-panel">
          <div className="panel-header">
            <div>
              <h2>{t("projects.createProject")}</h2>
              <p>{t("projects.createDescription")}</p>
            </div>
          </div>
          <form className="project-form compact-form" onSubmit={handleCreate}>
            <div className="form-grid">
              <label>
                {t("projects.name")} <span className="required">*</span>
                <input
                  autoFocus
                  required
                  value={form.name}
                  onChange={(event) => updateName(event.target.value)}
                  placeholder="CodeLens-Pro"
                />
              </label>
              <label>
                {t("projects.slug")} <span className="required">*</span>
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
                <small>{t("projects.slugHelp")}</small>
              </label>
              <label className="span-2">
                {t("projects.projectDescription")}
                <textarea
                  rows={3}
                  value={form.description}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      description: event.target.value,
                    }))
                  }
                  placeholder={t("projects.descriptionPlaceholder")}
                />
              </label>
              <label>
                {t("projects.projectStatus")}
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
                {t("projects.lifecycle")}
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
                {t("projects.priority")}
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
                {t("common.cancel")}
              </button>
              <button className="primary-button" disabled={submitting} type="submit">
                {submitting ? t("common.saving") : t("projects.createProject")}
              </button>
            </div>
          </form>
        </section>
      ) : null}

      <section className="panel">
        <div className="panel-header">
          <div>
            <h2>{t("projects.all")}</h2>
            <p>{loading ? `${t("common.loading")}…` : t("projects.workspaceCount", { count: projects.length })}</p>
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
              {t("common.retry")}
            </button>
          </div>
        ) : null}
        {!loading && !listError ? (
          <ProjectTable
            projects={projects}
            emptyMessage={t("projects.empty")}
          />
        ) : loading ? (
          <p className="loading-copy">{t("common.loading")}…</p>
        ) : null}
      </section>
    </div>
  );
}
