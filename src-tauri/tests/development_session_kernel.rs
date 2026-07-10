use std::sync::Arc;

use forgeos_lib::{
    application::{
        project::CreateProjectInput,
        session::{
            AbandonSessionInput, CompleteSessionInput, StartSessionInput, UpdateSessionInput,
        },
        workspace::CreateWorkspaceInput,
        ApplicationError, ProjectKernel,
    },
    domain::{DevelopmentSession, DomainError, ProjectStatus, SessionStatus},
    infrastructure::{database, repositories::SqliteProjectRepository},
    ports::{RepositoryError, SessionRepository},
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

fn start_input(goal: &str) -> StartSessionInput {
    StartSessionInput { goal: goal.into() }
}

fn update_input(goal: &str, notes: &str, expected_version: i64) -> UpdateSessionInput {
    UpdateSessionInput {
        goal: goal.into(),
        notes: notes.into(),
        expected_version,
    }
}

fn complete_input(expected_version: i64) -> CompleteSessionInput {
    CompleteSessionInput {
        outcome: "Development Session kernel implemented".into(),
        validation: "cargo test and desktop acceptance passed".into(),
        handoff: "Continue with the next approved engineering kernel".into(),
        expected_version,
    }
}

fn abandon_input(expected_version: i64) -> AbandonSessionInput {
    AbandonSessionInput {
        handoff: "Reassess the approach before starting a replacement Session".into(),
        expected_version,
    }
}

#[tokio::test]
async fn development_session_lifecycle_concurrency_ownership_and_persistence() {
    let directory = TempDir::new().expect("temporary directory should be created");
    let database_path = directory.path().join("forgeos-session-test.db");
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
        .get_current_session(&codelens.id)
        .await
        .expect("empty current lookup should work")
        .is_none());
    assert!(kernel
        .list_sessions(&codelens.id)
        .await
        .expect("empty history lookup should work")
        .is_empty());

    let codelens_session = kernel
        .start_session(
            &codelens.id,
            start_input("Implement CodeLens-Pro streaming response optimization"),
        )
        .await
        .expect("CodeLens-Pro Session should start");
    let forgeos_session = kernel
        .start_session(
            &forgeos.id,
            start_input("Implement ForgeOS Development Session kernel"),
        )
        .await
        .expect("ForgeOS Session should start");
    let test_session = kernel
        .start_session(
            &test_project.id,
            start_input("Validate the Session lifecycle"),
        )
        .await
        .expect("测试项目 Session should start");

    for session in [&codelens_session, &forgeos_session, &test_session] {
        assert_eq!(session.status(), SessionStatus::Open);
        assert_eq!(session.version(), 1);
        assert!(session.notes().is_empty());
        assert!(session.completed_at().is_none());
    }

    assert!(matches!(
        kernel
            .start_session(&codelens.id, start_input("Duplicate open Session"))
            .await,
        Err(ApplicationError::SessionAlreadyOpen)
    ));

    assert!(matches!(
        kernel
            .update_session(
                &codelens.id,
                &forgeos_session.id().to_owned(),
                update_input("Cross-project mutation", "Must be rejected", 1),
            )
            .await,
        Err(ApplicationError::Validation(
            DomainError::SessionProjectMismatch
        ))
    ));
    assert_eq!(
        kernel
            .get_current_session(&forgeos.id)
            .await
            .expect("ForgeOS current Session should load")
            .expect("ForgeOS should retain its Session")
            .version(),
        1
    );

    let codelens_updated = kernel
        .update_session(
            &codelens.id,
            codelens_session.id(),
            update_input(
                "Optimize Agent streaming response",
                "Implemented the bounded stream buffer in Markdown.",
                1,
            ),
        )
        .await
        .expect("open Session should update");
    assert_eq!(codelens_updated.version(), 2);
    assert!(codelens_updated.notes().contains("bounded stream buffer"));

    let mut invalid_completion = complete_input(codelens_updated.version());
    invalid_completion.validation = String::new();
    assert!(matches!(
        kernel
            .complete_session(&codelens.id, codelens_updated.id(), invalid_completion,)
            .await,
        Err(ApplicationError::Validation(DomainError::Required {
            field: "session validation"
        }))
    ));
    assert_eq!(
        kernel
            .get_current_session(&codelens.id)
            .await
            .expect("current Session should remain readable")
            .expect("failed completion must leave it open")
            .version(),
        2
    );

    let codelens_completed = kernel
        .complete_session(
            &codelens.id,
            codelens_updated.id(),
            complete_input(codelens_updated.version()),
        )
        .await
        .expect("CodeLens-Pro Session should complete");
    assert_eq!(codelens_completed.status(), SessionStatus::Completed);
    assert_eq!(codelens_completed.version(), 3);
    assert!(codelens_completed.completed_at().is_some());
    assert!(kernel
        .get_current_session(&codelens.id)
        .await
        .expect("current lookup should work")
        .is_none());

    assert!(matches!(
        kernel
            .update_session(
                &codelens.id,
                codelens_completed.id(),
                update_input("Rewrite history", "Must not change", 3),
            )
            .await,
        Err(ApplicationError::Validation(
            DomainError::TerminalSessionCannotBeModified
        ))
    ));

    let concurrent_start_a = kernel.start_session(&codelens.id, start_input("Follow-up Session A"));
    let concurrent_start_b = kernel.start_session(&codelens.id, start_input("Follow-up Session B"));
    let (start_a, start_b) = tokio::join!(concurrent_start_a, concurrent_start_b);
    let starts = [start_a, start_b];
    assert_eq!(starts.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        starts
            .iter()
            .filter(|result| matches!(result, Err(ApplicationError::SessionAlreadyOpen)))
            .count(),
        1
    );
    let follow_up = starts
        .into_iter()
        .find_map(Result::ok)
        .expect("one concurrent start should win");
    let follow_up = kernel
        .abandon_session(
            &codelens.id,
            follow_up.id(),
            abandon_input(follow_up.version()),
        )
        .await
        .expect("follow-up should be abandoned cleanly");
    assert_eq!(follow_up.status(), SessionStatus::Abandoned);

    let update_a = kernel.update_session(
        &test_project.id,
        test_session.id(),
        update_input("Validate concurrency A", "Writer A", 1),
    );
    let update_b = kernel.update_session(
        &test_project.id,
        test_session.id(),
        update_input("Validate concurrency B", "Writer B", 1),
    );
    let (result_a, result_b) = tokio::join!(update_a, update_b);
    let updates = [result_a, result_b];
    assert_eq!(updates.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        updates
            .iter()
            .filter(|result| matches!(
                result,
                Err(ApplicationError::SessionVersionConflict {
                    expected: 1,
                    actual: 2
                })
            ))
            .count(),
        1
    );
    let current_test_session = kernel
        .get_current_session(&test_project.id)
        .await
        .expect("测试项目 current Session should load")
        .expect("测试项目 should keep an open Session");
    assert_eq!(current_test_session.version(), 2);
    assert_eq!(current_test_session.status(), SessionStatus::Open);

    let archived_forgeos = kernel
        .archive_project(&forgeos.id, forgeos.version)
        .await
        .expect("ForgeOS Project should archive");
    assert_eq!(archived_forgeos.status, ProjectStatus::Archived);

    let mut bypassed_application = forgeos_session.clone();
    bypassed_application
        .update_open(
            "Bypass Application".into(),
            "Repository must reject this write".into(),
            chrono::Utc::now(),
        )
        .expect("the open Session is domain-valid before persistence");
    assert!(matches!(
        repository
            .update_open_session(&bypassed_application, forgeos_session.version())
            .await,
        Err(RepositoryError::ProjectArchived)
    ));
    assert!(matches!(
        kernel
            .update_session(
                &forgeos.id,
                forgeos_session.id(),
                update_input("Archived write", "Reject", forgeos_session.version()),
            )
            .await,
        Err(ApplicationError::Validation(
            DomainError::ArchivedProjectCannotAcceptSessionChanges
        ))
    ));

    let forgeos_abandoned = kernel
        .abandon_session(
            &forgeos.id,
            forgeos_session.id(),
            abandon_input(forgeos_session.version()),
        )
        .await
        .expect("archived Project may close a pre-existing open Session");
    assert_eq!(forgeos_abandoned.status(), SessionStatus::Abandoned);
    assert_eq!(forgeos_abandoned.version(), 2);
    assert!(forgeos_abandoned.completed_at().is_some());

    let archived_codelens = kernel
        .archive_project(&codelens.id, codelens.version)
        .await
        .expect("CodeLens-Pro Project should archive");
    assert_eq!(archived_codelens.status, ProjectStatus::Archived);
    assert!(matches!(
        kernel
            .start_session(&codelens.id, start_input("Archived Project start"))
            .await,
        Err(ApplicationError::Validation(
            DomainError::ArchivedProjectCannotAcceptSessionChanges
        ))
    ));

    let orphan = DevelopmentSession::start(
        "orphan-session".into(),
        "missing-project".into(),
        "Orphan".into(),
        chrono::Utc::now(),
    )
    .expect("domain accepts a non-empty Project identity");
    assert!(matches!(
        repository.insert_session(&orphan).await,
        Err(RepositoryError::Storage(_))
    ));

    let codelens_history = kernel
        .list_sessions(&codelens.id)
        .await
        .expect("CodeLens-Pro history should load");
    assert_eq!(codelens_history.len(), 2);
    assert!(codelens_history
        .iter()
        .any(|session| session.status() == SessionStatus::Completed));
    assert!(codelens_history
        .iter()
        .any(|session| session.status() == SessionStatus::Abandoned));

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

    let persisted_test_session = reopened_kernel
        .get_current_session(&test_project.id)
        .await
        .expect("persisted current Session should load")
        .expect("测试项目 open Session should survive restart");
    assert_eq!(persisted_test_session.version(), 2);
    assert_eq!(persisted_test_session.status(), SessionStatus::Open);
    assert!(reopened_kernel
        .get_current_session(&codelens.id)
        .await
        .expect("CodeLens-Pro current lookup should work")
        .is_none());
    assert_eq!(
        reopened_kernel
            .list_sessions(&codelens.id)
            .await
            .expect("CodeLens-Pro history should persist")
            .len(),
        2
    );
    assert_eq!(
        reopened_kernel
            .list_sessions(&forgeos.id)
            .await
            .expect("ForgeOS history should persist")[0]
            .status(),
        SessionStatus::Abandoned
    );

    let persisted_integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&reopened.pool)
        .await
        .expect("reopened integrity_check should run");
    assert_eq!(persisted_integrity, "ok");
    let persisted_foreign_key_violations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
            .fetch_one(&reopened.pool)
            .await
            .expect("reopened foreign_key_check should run");
    assert_eq!(persisted_foreign_key_violations, 0);

    reopened.pool.close().await;
}
