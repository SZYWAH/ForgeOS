use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use forgeos_lib::{
    application::{
        evidence::{CreateEvidenceInput, UpdateEvidenceInput},
        repository::{RefreshRepositoryInput, RegisterRepositoryInput},
        ApplicationError, ProjectKernel, RepositoryKernel, RepositoryKernelError,
    },
    domain::{DiffSummary, DomainError},
    infrastructure::{database, repositories::SqliteProjectRepository},
    ports::{GitAdapter, GitAdapterError, GitCommit, GitInspection},
};
use tempfile::TempDir;

#[derive(Default)]
struct FakeGitAdapter {
    inspections: Mutex<HashMap<String, GitInspection>>,
}

impl FakeGitAdapter {
    fn set(&self, path: &str, inspection: GitInspection) {
        self.inspections
            .lock()
            .unwrap()
            .insert(path.into(), inspection);
    }
}

#[async_trait]
impl GitAdapter for FakeGitAdapter {
    async fn inspect(&self, path: &str) -> Result<GitInspection, GitAdapterError> {
        self.inspections
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or(GitAdapterError::NotRepository)
    }

    async fn get_commit(&self, path: &str, sha: &str) -> Result<GitCommit, GitAdapterError> {
        self.inspections
            .lock()
            .unwrap()
            .get(path)
            .and_then(|inspection| {
                inspection
                    .recent_commits
                    .iter()
                    .find(|item| item.sha == sha)
            })
            .cloned()
            .ok_or_else(|| GitAdapterError::InvalidOutput("commit not found".into()))
    }
}

#[tokio::test]
async fn repositories_working_copies_and_git_evidence_are_isolated_and_persistent() {
    let directory = TempDir::new().unwrap();
    let database_path = directory.path().join("forgeos-git-evidence.db");
    let database = database::connect(&database_path).await.unwrap();
    seed_projects(&database.pool).await;

    let git = Arc::new(FakeGitAdapter::default());
    let codelens_path = r"D:\repos\CodeLens-Pro";
    let forgeos_path = r"D:\wenjian\Desktop\ForgeOS";
    git.set(
        codelens_path,
        inspection(codelens_path, "main", &"a".repeat(40), false, 1),
    );
    git.set(
        forgeos_path,
        inspection(forgeos_path, "master", &"b".repeat(40), true, 2),
    );

    let repository = Arc::new(SqliteProjectRepository::new(database.pool.clone()));
    let project_kernel = ProjectKernel::new(repository.clone());
    let kernel = RepositoryKernel::new(repository, git.clone());

    let manual_git_evidence = project_kernel
        .create_evidence(
            "project-codelens",
            CreateEvidenceInput {
                title: "Forged Git fact".into(),
                description: "Must use RepositoryKernel".into(),
                evidence_type: "git_commit".into(),
                source_type: "local_file".into(),
                source_uri: codelens_path.into(),
            },
        )
        .await;
    assert!(matches!(
        manual_git_evidence,
        Err(ApplicationError::Validation(
            DomainError::GitEvidenceRequiresRepositoryIntegration
        ))
    ));
    let codelens = kernel
        .register_repository(
            "project-codelens",
            RegisterRepositoryInput {
                path: codelens_path.into(),
            },
        )
        .await
        .unwrap();
    let forgeos = kernel
        .register_repository(
            "project-forgeos",
            RegisterRepositoryInput {
                path: forgeos_path.into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(codelens.working_copy.branch(), "main");
    assert!(!codelens.working_copy.is_dirty());
    assert_eq!(forgeos.working_copy.branch(), "master");
    assert!(forgeos.working_copy.is_dirty());

    let duplicate = kernel
        .register_repository(
            "project-test",
            RegisterRepositoryInput {
                path: codelens_path.into(),
            },
        )
        .await;
    assert!(matches!(
        duplicate,
        Err(RepositoryKernelError::RepositoryAlreadyRegistered)
    ));

    let commit = kernel
        .create_commit_evidence(
            "project-codelens",
            codelens.repository.id(),
            codelens.working_copy.head_commit(),
        )
        .await
        .unwrap();
    assert_eq!(commit.evidence_type().as_str(), "git_commit");
    assert_eq!(commit.project_id(), "project-codelens");
    assert!(commit.description().contains("Commit:"));
    let rewrite_commit = project_kernel
        .update_evidence(
            "project-codelens",
            commit.id(),
            UpdateEvidenceInput {
                title: "Rewritten".into(),
                description: "Must be rejected".into(),
                evidence_type: "note".into(),
                source_type: "manual_note".into(),
                source_uri: "".into(),
                expected_version: commit.version(),
            },
        )
        .await;
    assert!(matches!(
        rewrite_commit,
        Err(ApplicationError::Validation(
            DomainError::GitEvidenceContentReadOnly
        ))
    ));
    let status = kernel
        .create_status_evidence("project-codelens", codelens.repository.id())
        .await
        .unwrap();
    assert_eq!(status.evidence_type().as_str(), "git_status");
    assert!(status.description().contains("Diff Summary:"));

    let listed = kernel
        .list_repository_evidence("project-codelens", codelens.repository.id())
        .await
        .unwrap();
    assert_eq!(listed.len(), 2);
    assert!(listed
        .iter()
        .all(|item| item.project_id() == "project-codelens"));
    let cross_project = kernel
        .list_repository_evidence("project-forgeos", codelens.repository.id())
        .await;
    assert!(matches!(
        cross_project,
        Err(RepositoryKernelError::Validation(
            DomainError::RepositoryProjectMismatch
        ))
    ));

    git.set(
        codelens_path,
        inspection(codelens_path, "feature/index", &"c".repeat(40), true, 3),
    );
    let refreshed = kernel
        .refresh_repository(
            "project-codelens",
            codelens.repository.id(),
            RefreshRepositoryInput {
                expected_version: 1,
            },
        )
        .await
        .unwrap();
    assert_eq!(refreshed.repository.version(), 2);
    assert_eq!(refreshed.working_copy.branch(), "feature/index");
    assert!(refreshed.working_copy.is_dirty());
    let working_copy_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM working_copies WHERE repository_id = ?")
            .bind(codelens.repository.id())
            .fetch_one(&database.pool)
            .await
            .unwrap();
    assert_eq!(
        working_copy_count, 2,
        "refresh must append a WorkingCopy observation"
    );

    let stale = kernel
        .refresh_repository(
            "project-codelens",
            codelens.repository.id(),
            RefreshRepositoryInput {
                expected_version: 1,
            },
        )
        .await;
    assert!(matches!(
        stale,
        Err(RepositoryKernelError::VersionConflict {
            expected: 1,
            actual: 2
        })
    ));

    sqlx::query(
        "INSERT INTO evidences (id, project_id, title, description, type, source_type, \
         source_uri, verification_status, created_at, updated_at, version) VALUES ( \
         'cross-evidence', 'project-codelens', 'Cross', '', 'git_status', 'local_file', \
         'D:/repo', 'unverified', '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 1)",
    )
    .execute(&database.pool)
    .await
    .unwrap();
    let direct_cross = sqlx::query(
        "INSERT INTO repository_evidence_links \
         (evidence_id, repository_id, project_id, commit_sha, captured_at) \
         VALUES ('cross-evidence', ?, 'project-codelens', NULL, '2026-07-12T00:00:00Z')",
    )
    .bind(forgeos.repository.id())
    .execute(&database.pool)
    .await;
    assert!(
        direct_cross.is_err(),
        "composite foreign key must reject cross-Project evidence"
    );

    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(integrity, "ok");
    let foreign_keys: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(foreign_keys, 0);

    let codelens_id = codelens.repository.id().to_owned();
    database.pool.close().await;
    let reopened = database::connect(&database_path).await.unwrap();
    let reopened_kernel = RepositoryKernel::new(
        Arc::new(SqliteProjectRepository::new(reopened.pool.clone())),
        git,
    );
    let records = reopened_kernel
        .list_repositories("project-codelens")
        .await
        .unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].repository.version(), 2);
    assert_eq!(records[0].working_copy.branch(), "feature/index");
    assert_eq!(
        reopened_kernel
            .list_repository_evidence("project-codelens", &codelens_id)
            .await
            .unwrap()
            .len(),
        2
    );
    reopened.pool.close().await;
}

fn inspection(path: &str, branch: &str, sha: &str, dirty: bool, hour: u32) -> GitInspection {
    GitInspection {
        root_path: path.into(),
        remote_url: Some("https://example.com/repository.git".into()),
        default_branch: "main".into(),
        branch: branch.into(),
        head_commit: sha.into(),
        is_dirty: dirty,
        diff_summary: DiffSummary {
            changed_files: i64::from(dirty),
            staged_files: 0,
            unstaged_files: i64::from(dirty),
            untracked_files: 0,
            insertions: if dirty { 12 } else { 0 },
            deletions: if dirty { 3 } else { 0 },
        },
        checked_at: Utc.with_ymd_and_hms(2026, 7, 12, hour, 0, 0).unwrap(),
        recent_commits: vec![GitCommit {
            sha: sha.into(),
            message: "feat: observed commit".into(),
            author_name: "ForgeOS Test".into(),
            author_email: "test@example.com".into(),
            authored_at: Utc.with_ymd_and_hms(2026, 7, 12, hour, 0, 0).unwrap(),
        }],
    }
}

async fn seed_projects(pool: &sqlx::SqlitePool) {
    sqlx::query("INSERT INTO workspaces (id, name, timezone, created_at, updated_at) VALUES ('workspace-1', 'Personal', 'Asia/Shanghai', '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z')")
        .execute(pool).await.unwrap();
    for (id, slug, name) in [
        ("project-codelens", "codelens-pro", "CodeLens-Pro"),
        ("project-forgeos", "forgeos", "ForgeOS"),
        ("project-test", "test-project", "测试项目"),
    ] {
        sqlx::query("INSERT INTO projects (id, workspace_id, slug, name, description, vision, target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, created_at, updated_at, version) VALUES (?, 'workspace-1', ?, ?, '', '', '', '', '', '', 'active', 'implementation', 80, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 1)")
            .bind(id).bind(slug).bind(name).execute(pool).await.unwrap();
    }
}
