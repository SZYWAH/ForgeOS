use std::sync::Arc;

use chrono::Utc;
use forgeos_lib::{
    application::{
        GitHubAnalysisImportError, GitHubAnalysisImportKernel, ReviewGitHubAnalysisImportInput,
        UpdateGitHubAnalysisImportInput,
    },
    domain::{LifecycleStage, ProjectDetails, ProjectStatus, Workspace},
    infrastructure::{database, repositories::SqliteProjectRepository},
    ports::WorkspaceRepository,
};

async fn setup() -> (
    tempfile::TempDir,
    database::DatabaseState,
    Arc<SqliteProjectRepository>,
) {
    let dir = tempfile::tempdir().unwrap();
    let state = database::connect(&dir.path().join("forgeos.db"))
        .await
        .unwrap();
    let repository = Arc::new(SqliteProjectRepository::new(state.pool.clone()));
    repository
        .insert_workspace(
            &Workspace::new(
                "workspace-1".into(),
                "Personal".into(),
                "Asia/Shanghai".into(),
                Utc::now(),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    (dir, state, repository)
}

async fn insert_ready_analysis(state: &database::DatabaseState, id: &str, status: &str) {
    let readme_source = format!("{id}-readme");
    let manifest_source = format!("{id}-manifest");
    let release_source = format!("{id}-release");
    let report = serde_json::json!({
        "schemaVersion": "github-report-v2",
        "repository": {
            "owner": "SZYWAH",
            "name": "Analyzed-App",
            "url": "https://github.com/SZYWAH/Analyzed-App",
            "defaultBranch": "main",
            "headCommitSha": "a".repeat(40)
        },
        "repositoryKind": {
            "value": "source_application",
            "confidence": "high",
            "sourceIds": [readme_source]
        },
        "overview": {
            "value": "A source-backed desktop engineering application.",
            "confidence": "high",
            "sourceIds": [readme_source]
        },
        "capabilities": [{
            "value": "Manage local engineering context",
            "confidence": "high",
            "sourceIds": [readme_source]
        }],
        "techStack": [],
        "productTechStack": [{
            "value": "Rust",
            "confidence": "high",
            "sourceIds": [manifest_source]
        }, {
            "value": "TypeScript",
            "confidence": "high",
            "sourceIds": [manifest_source]
        }],
        "repositoryToolingStack": [{
            "value": "GitHub Actions",
            "confidence": "medium",
            "sourceIds": [manifest_source]
        }],
        "architecture": [],
        "entrypoints": [],
        "buildAndRun": [],
        "testStrategy": [],
        "dependencies": null,
        "license": null,
        "risks": [],
        "recommendedReadingOrder": [],
        "reproductionAssessment": {
            "value": { "difficulty": "unknown", "prerequisites": [], "caveats": [] },
            "confidence": "low",
            "sourceIds": [readme_source]
        },
        "extensionPoints": [],
        "recentActivity": [{
            "kind": "release",
            "identifier": "v0.2.0",
            "title": "Preview release",
            "occurredAt": "2026-07-22T12:00:00Z",
            "sourceIds": [release_source]
        }],
        "sectionAvailability": {},
        "userQuestionAnswer": null,
        "completeness": {
            "readiness": "warning",
            "filesSelected": 2,
            "filesSkipped": 0,
            "treeEntriesSeen": 10,
            "sourceBytes": 512
        },
        "qualityWarnings": ["license_missing"]
    })
    .to_string();
    sqlx::query(
        "INSERT INTO github_analysis_runs (
            id, workspace_id, github_repository_id, owner, name, repository_url,
            default_branch, source_commit_sha, focus_type, status, analysis_mode,
            readiness, analyzer_revision, request_hash, report_json, report_markdown,
            content_hash, completed_at, created_at, updated_at
         ) VALUES (?, 'workspace-1', 'github-repository-1', 'SZYWAH', 'Analyzed-App',
            'https://github.com/SZYWAH/Analyzed-App', 'main', ?, 'overview', ?,
            'rules', 'warning', 'github-rules-v2', ?, ?, '# report', ?,
            '2026-07-22T12:00:00Z', '2026-07-22T12:00:00Z', '2026-07-22T12:00:00Z')",
    )
    .bind(id)
    .bind("a".repeat(40))
    .bind(status)
    .bind(format!("{id:_<64}"))
    .bind(report)
    .bind("c".repeat(64))
    .execute(&state.pool)
    .await
    .unwrap();

    for (source_id, source_type, source_path, order) in [
        (readme_source, "readme", "README.md", 0_i64),
        (manifest_source, "manifest", "Cargo.toml", 1_i64),
        (release_source, "release", "release/v0.2.0", 2_i64),
    ] {
        sqlx::query(
            "INSERT INTO github_analysis_sources (
                id, analysis_run_id, source_type, source_path, content_hash, excerpt, source_order
             ) VALUES (?, ?, ?, ?, ?, 'safe excerpt', ?)",
        )
        .bind(source_id)
        .bind(id)
        .bind(source_type)
        .bind(source_path)
        .bind("d".repeat(64))
        .bind(order)
        .execute(&state.pool)
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn analyzed_report_requires_review_and_imports_only_project_and_bounded_evidence() {
    let (_dir, state, repository) = setup().await;
    insert_ready_analysis(&state, "analysis-1", "ready").await;
    let kernel = GitHubAnalysisImportKernel::new(repository);

    let prepared = kernel.prepare("workspace-1", "analysis-1").await.unwrap();
    let draft = prepared.draft.unwrap();
    let proposal = prepared.proposal.unwrap();
    assert_eq!(proposal.project.name, "Analyzed-App");
    assert_eq!(proposal.project.tech_stack, "Rust, TypeScript");
    assert!(!proposal.project.tech_stack.contains("GitHub Actions"));
    assert_eq!(proposal.project.lifecycle_stage, LifecycleStage::Definition);

    let updated = kernel
        .update(
            "workspace-1",
            draft.id(),
            UpdateGitHubAnalysisImportInput {
                expected_draft_version: draft.version(),
                expected_proposal_version: proposal.version,
                project: ProjectDetails {
                    slug: "analyzed-app".into(),
                    name: "Analyzed App".into(),
                    description: proposal.project.description,
                    vision: "Understand and extend the project safely.".into(),
                    target_user: "Independent developers".into(),
                    scope: proposal.project.scope,
                    non_goals: "No automatic execution".into(),
                    tech_stack: proposal.project.tech_stack,
                    status: ProjectStatus::Active,
                    lifecycle_stage: LifecycleStage::Definition,
                    priority: 60,
                },
                partial_acknowledged: false,
                stale_acknowledged: false,
            },
        )
        .await
        .unwrap();
    let reviewed = kernel
        .review(
            "workspace-1",
            updated.draft.as_ref().unwrap().id(),
            ReviewGitHubAnalysisImportInput {
                expected_draft_version: updated.draft.as_ref().unwrap().version(),
                expected_proposal_version: updated.proposal.as_ref().unwrap().version,
                partial_acknowledged: false,
                stale_acknowledged: false,
            },
        )
        .await
        .unwrap();
    let result = kernel
        .import_reviewed(
            "workspace-1",
            reviewed.draft.as_ref().unwrap().id(),
            reviewed.draft.as_ref().unwrap().version(),
            reviewed.proposal.as_ref().unwrap().version,
        )
        .await
        .unwrap();
    assert_eq!(result.project.name, "Analyzed App");
    assert_eq!(result.evidences.len(), 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects")
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM evidences")
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        2
    );
    for table in [
        "project_snapshots",
        "development_sessions",
        "project_achievements",
        "repositories",
    ] {
        let query = format!("SELECT COUNT(*) FROM {table}");
        assert_eq!(
            sqlx::query_scalar::<_, i64>(&query)
                .fetch_one(&state.pool)
                .await
                .unwrap(),
            0,
            "{table} must remain untouched"
        );
    }

    let retry = kernel
        .import_reviewed(
            "workspace-1",
            result.draft.id(),
            result.draft.version(),
            reviewed.proposal.unwrap().version,
        )
        .await
        .unwrap();
    assert_eq!(retry.project.id, result.project.id);
    assert!(retry.evidences.is_empty());
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT imported_project_id FROM github_analysis_runs WHERE id = 'analysis-1'",
        )
        .fetch_one(&state.pool)
        .await
        .unwrap(),
        result.project.id
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        "ok"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pragma_foreign_key_check")
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn partial_analysis_requires_explicit_acknowledgement() {
    let (_dir, state, repository) = setup().await;
    insert_ready_analysis(&state, "analysis-partial", "partial").await;
    let kernel = GitHubAnalysisImportKernel::new(repository);
    let prepared = kernel
        .prepare("workspace-1", "analysis-partial")
        .await
        .unwrap();
    let draft = prepared.draft.unwrap();
    let proposal = prepared.proposal.unwrap();
    assert!(matches!(
        kernel
            .review(
                "workspace-1",
                draft.id(),
                ReviewGitHubAnalysisImportInput {
                    expected_draft_version: draft.version(),
                    expected_proposal_version: proposal.version,
                    partial_acknowledged: false,
                    stale_acknowledged: false,
                },
            )
            .await,
        Err(GitHubAnalysisImportError::PartialAcknowledgementRequired)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects")
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn stale_analysis_requires_explicit_acknowledgement() {
    let (_dir, state, repository) = setup().await;
    insert_ready_analysis(&state, "analysis-old", "ready").await;
    let kernel = GitHubAnalysisImportKernel::new(repository);
    let prepared = kernel.prepare("workspace-1", "analysis-old").await.unwrap();
    insert_ready_analysis(&state, "analysis-new", "ready").await;
    sqlx::query(
        "UPDATE github_analysis_runs
         SET source_commit_sha = ?, created_at = '2026-07-23T00:00:00Z',
             updated_at = '2026-07-23T00:00:00Z'
         WHERE id = 'analysis-new'",
    )
    .bind("e".repeat(40))
    .execute(&state.pool)
    .await
    .unwrap();

    assert!(matches!(
        kernel
            .review(
                "workspace-1",
                prepared.draft.as_ref().unwrap().id(),
                ReviewGitHubAnalysisImportInput {
                    expected_draft_version: prepared.draft.as_ref().unwrap().version(),
                    expected_proposal_version: prepared.proposal.as_ref().unwrap().version,
                    partial_acknowledged: false,
                    stale_acknowledged: false,
                },
            )
            .await,
        Err(GitHubAnalysisImportError::StaleAcknowledgementRequired)
    ));
}
