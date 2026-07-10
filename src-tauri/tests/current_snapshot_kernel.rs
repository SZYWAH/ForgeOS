use std::sync::Arc;

use forgeos_lib::{
    application::{
        project::CreateProjectInput,
        snapshot::{CreateSnapshotInput, UpdateSnapshotInput},
        workspace::CreateWorkspaceInput,
        ApplicationError, ProjectKernel,
    },
    domain::{DomainError, ProjectSnapshot, ProjectStatus, SnapshotDetails},
    infrastructure::{database, repositories::SqliteProjectRepository},
    ports::{RepositoryError, SnapshotRepository},
};
use tempfile::TempDir;

fn project_input(slug: &str, name: &str) -> CreateProjectInput {
    CreateProjectInput {
        slug: slug.into(),
        name: name.into(),
        description: format!("{name} project description"),
        vision: format!("Long-term vision for {name}"),
        target_user: "Independent developers".into(),
        scope: "Trusted engineering memory".into(),
        non_goals: "Task management".into(),
        tech_stack: "React, TypeScript, Tauri, Rust, SQLite".into(),
        status: "active".into(),
        lifecycle_stage: "implementation".into(),
        priority: 80,
    }
}

fn snapshot_input(label: &str) -> CreateSnapshotInput {
    CreateSnapshotInput {
        current_goal: format!(" Complete {label} Current Snapshot "),
        status_summary: format!("{label} is under active implementation"),
        achievements: vec![format!("Created {label} Project identity"), " ".into()],
        blockers: vec![format!("Validate {label} restart persistence")],
        next_steps: vec![
            "Finish repository".into(),
            "Run verification".into(),
            "Document the result".into(),
        ],
    }
}

fn update_input(goal: &str, expected_version: i64) -> UpdateSnapshotInput {
    UpdateSnapshotInput {
        current_goal: goal.into(),
        status_summary: "Kernel implementation is complete".into(),
        achievements: vec!["Migration passes".into(), "IPC is typed".into()],
        blockers: Vec::new(),
        next_steps: vec!["Perform desktop acceptance".into()],
        expected_version,
    }
}

#[tokio::test]
async fn current_snapshot_history_concurrency_and_persistence() {
    let directory = TempDir::new().expect("temporary directory should be created");
    let database_path = directory.path().join("forgeos-snapshot-test.db");
    let database = database::connect(&database_path)
        .await
        .expect("database should initialize");
    let repository = Arc::new(SqliteProjectRepository::new(database.pool.clone()));
    let kernel = ProjectKernel::new(repository.clone());

    kernel
        .create_workspace(CreateWorkspaceInput {
            name: "Personal Engineering".into(),
            timezone: "Asia/Shanghai".into(),
        })
        .await
        .expect("Workspace should be created");

    let codelens = kernel
        .create_project(project_input("codelens-pro", "CodeLens-Pro"))
        .await
        .expect("CodeLens-Pro should be created");
    let forgeos = kernel
        .create_project(project_input("forgeos", "ForgeOS"))
        .await
        .expect("ForgeOS should be created");
    let test_project = kernel
        .create_project(project_input("test-project", "测试项目"))
        .await
        .expect("测试项目 should be created");

    assert!(kernel
        .get_current_snapshot(&codelens.id)
        .await
        .expect("empty current lookup should work")
        .is_none());
    assert!(kernel
        .get_snapshot_history(&codelens.id)
        .await
        .expect("empty history lookup should work")
        .is_empty());

    let codelens_v1 = kernel
        .create_snapshot(&codelens.id, snapshot_input("CodeLens-Pro"))
        .await
        .expect("CodeLens-Pro Snapshot should be created");
    let forgeos_v1 = kernel
        .create_snapshot(&forgeos.id, snapshot_input("ForgeOS"))
        .await
        .expect("ForgeOS Snapshot should be created");
    let test_v1 = kernel
        .create_snapshot(&test_project.id, snapshot_input("测试项目"))
        .await
        .expect("测试项目 Snapshot should be created");

    assert_eq!(codelens_v1.version(), 1);
    assert_eq!(
        codelens_v1.current_goal(),
        "Complete CodeLens-Pro Current Snapshot"
    );
    assert_eq!(codelens_v1.achievements().len(), 1);
    assert_eq!(forgeos_v1.project_id(), forgeos.id);
    assert_eq!(test_v1.project_id(), test_project.id);

    assert!(matches!(
        kernel
            .create_snapshot(&codelens.id, snapshot_input("duplicate"))
            .await,
        Err(ApplicationError::SnapshotAlreadyExists)
    ));
    assert!(matches!(
        kernel
            .create_snapshot("missing-project", snapshot_input("missing"))
            .await,
        Err(ApplicationError::ProjectNotFound)
    ));

    let wrong_project = ProjectSnapshot::next_version(
        "cross-project-version".into(),
        forgeos.id.clone(),
        &codelens_v1,
        SnapshotDetails {
            current_goal: "Wrong Project".into(),
            status_summary: "Must be rejected".into(),
            achievements: Vec::new(),
            blockers: Vec::new(),
            next_steps: Vec::new(),
        },
        chrono::Utc::now(),
    );
    assert_eq!(wrong_project, Err(DomainError::SnapshotProjectMismatch));

    let orphan = ProjectSnapshot::new(
        "orphan-snapshot".into(),
        "missing-project".into(),
        SnapshotDetails {
            current_goal: "Orphan".into(),
            status_summary: "Foreign key must reject this row".into(),
            achievements: Vec::new(),
            blockers: Vec::new(),
            next_steps: Vec::new(),
        },
        chrono::Utc::now(),
    )
    .expect("domain accepts a non-empty Project identity");
    assert!(matches!(
        repository.insert_snapshot(&orphan).await,
        Err(RepositoryError::Storage(_))
    ));

    let update_a = kernel.update_snapshot(
        &codelens.id,
        update_input("Optimize large repository analysis", 1),
    );
    let update_b =
        kernel.update_snapshot(&codelens.id, update_input("Improve repository indexing", 1));
    let (result_a, result_b) = tokio::join!(update_a, update_b);
    let results = [result_a, result_b];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                Err(ApplicationError::SnapshotVersionConflict {
                    expected: 1,
                    actual: 2
                })
            ))
            .count(),
        1
    );

    let current = kernel
        .get_current_snapshot(&codelens.id)
        .await
        .expect("current Snapshot should load")
        .expect("CodeLens-Pro should have a current Snapshot");
    assert_eq!(current.version(), 2);
    assert_ne!(current.id(), codelens_v1.id());

    let history = kernel
        .get_snapshot_history(&codelens.id)
        .await
        .expect("Snapshot history should load");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].version(), 2);
    assert_eq!(history[1].version(), 1);
    assert_eq!(history[1].current_goal(), codelens_v1.current_goal());

    assert!(matches!(
        kernel
            .update_snapshot(&codelens.id, update_input("stale write", 1))
            .await,
        Err(ApplicationError::SnapshotVersionConflict {
            expected: 1,
            actual: 2
        })
    ));

    let mut too_many_steps = update_input("Too many next steps", forgeos_v1.version());
    too_many_steps.next_steps = vec!["one".into(), "one".into(), "three".into(), "four".into()];
    assert!(matches!(
        kernel.update_snapshot(&forgeos.id, too_many_steps).await,
        Err(ApplicationError::Validation(
            DomainError::TooManyNextSteps { max: 3 }
        ))
    ));
    assert_eq!(
        kernel
            .get_current_snapshot(&forgeos.id)
            .await
            .expect("ForgeOS current Snapshot should load")
            .expect("ForgeOS should have a Snapshot")
            .version(),
        1
    );

    let archived = kernel
        .archive_project(&test_project.id, test_project.version)
        .await
        .expect("test Project should archive");
    assert_eq!(archived.status, ProjectStatus::Archived);

    let bypassed_application = ProjectSnapshot::next_version(
        "archived-direct-insert".into(),
        test_project.id.clone(),
        &test_v1,
        SnapshotDetails {
            current_goal: "Bypass Application".into(),
            status_summary: "Repository must still reject this write".into(),
            achievements: Vec::new(),
            blockers: Vec::new(),
            next_steps: Vec::new(),
        },
        chrono::Utc::now(),
    )
    .expect("the next Snapshot version is domain-valid before persistence");
    assert!(matches!(
        repository.insert_snapshot(&bypassed_application).await,
        Err(RepositoryError::ProjectArchived)
    ));

    assert!(matches!(
        kernel
            .update_snapshot(
                &test_project.id,
                update_input("Archived write", test_v1.version()),
            )
            .await,
        Err(ApplicationError::Validation(
            DomainError::ArchivedProjectCannotAcceptSnapshots
        ))
    ));
    assert_eq!(
        kernel
            .get_current_snapshot(&test_project.id)
            .await
            .expect("archived Project Snapshot remains readable")
            .expect("archived Project keeps its Snapshot")
            .version(),
        1
    );

    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&database.pool)
        .await
        .expect("integrity_check should run");
    assert_eq!(integrity, "ok");
    let foreign_key_violations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
            .fetch_one(&database.pool)
            .await
            .expect("foreign_key_check should run");
    assert_eq!(foreign_key_violations, 0);

    database.pool.close().await;
    drop(kernel);
    drop(repository);

    let reopened = database::connect(&database_path)
        .await
        .expect("database should reopen");
    let reopened_repository = Arc::new(SqliteProjectRepository::new(reopened.pool.clone()));
    let reopened_kernel = ProjectKernel::new(reopened_repository);

    let persisted_current = reopened_kernel
        .get_current_snapshot(&codelens.id)
        .await
        .expect("persisted current Snapshot should load")
        .expect("persisted current Snapshot should exist");
    assert_eq!(persisted_current.version(), 2);
    let persisted_history = reopened_kernel
        .get_snapshot_history(&codelens.id)
        .await
        .expect("persisted history should load");
    assert_eq!(persisted_history.len(), 2);
    assert_eq!(persisted_history[1].id(), codelens_v1.id());

    reopened.pool.close().await;
}
