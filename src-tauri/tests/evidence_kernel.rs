use std::sync::Arc;

use forgeos_lib::{
    application::{
        evidence::{
            ChangeEvidenceStatusInput, CreateEvidenceInput, LinkEvidenceInput, UpdateEvidenceInput,
        },
        project::CreateProjectInput,
        session::StartSessionInput,
        snapshot::CreateSnapshotInput,
        workspace::CreateWorkspaceInput,
        ApplicationError, ProjectKernel,
    },
    domain::{DomainError, VerificationStatus},
    infrastructure::{database, repositories::SqliteProjectRepository},
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

fn snapshot_input(name: &str) -> CreateSnapshotInput {
    CreateSnapshotInput {
        current_goal: format!("Complete {name} Evidence Kernel validation"),
        status_summary: format!("{name} is under active implementation"),
        achievements: vec!["Project identity is durable".into()],
        blockers: Vec::new(),
        next_steps: vec!["Attach trusted engineering evidence".into()],
    }
}

fn document(title: &str, path: &str) -> CreateEvidenceInput {
    CreateEvidenceInput {
        title: title.into(),
        description: "A durable source supporting the current engineering fact".into(),
        evidence_type: "document".into(),
        source_type: "local_file".into(),
        source_uri: path.into(),
    }
}

fn manual_note(title: &str) -> CreateEvidenceInput {
    CreateEvidenceInput {
        title: title.into(),
        description: "Manually confirmed during D1.4 acceptance".into(),
        evidence_type: "note".into(),
        source_type: "manual_note".into(),
        source_uri: String::new(),
    }
}

#[tokio::test]
async fn evidence_lifecycle_links_constraints_and_persistence() {
    let directory = TempDir::new().expect("temporary directory should be created");
    let database_path = directory.path().join("forgeos-evidence-test.db");
    let database = database::connect(&database_path)
        .await
        .expect("database should initialize");
    let repository = Arc::new(SqliteProjectRepository::new(database.pool.clone()));
    let kernel = ProjectKernel::new(repository);

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

    let codelens_snapshot = kernel
        .create_snapshot(&codelens.id, snapshot_input("CodeLens-Pro"))
        .await
        .expect("CodeLens-Pro Snapshot should be created");
    let forgeos_snapshot = kernel
        .create_snapshot(&forgeos.id, snapshot_input("ForgeOS"))
        .await
        .expect("ForgeOS Snapshot should be created");
    let test_snapshot = kernel
        .create_snapshot(&test_project.id, snapshot_input("测试项目"))
        .await
        .expect("测试项目 Snapshot should be created");

    let codelens_session = kernel
        .start_session(
            &codelens.id,
            StartSessionInput {
                goal: "Validate streaming response evidence".into(),
            },
        )
        .await
        .expect("CodeLens-Pro Session should start");
    let forgeos_session = kernel
        .start_session(
            &forgeos.id,
            StartSessionInput {
                goal: "Implement Evidence Kernel".into(),
            },
        )
        .await
        .expect("ForgeOS Session should start");
    let test_session = kernel
        .start_session(
            &test_project.id,
            StartSessionInput {
                goal: "Exercise Evidence lifecycle".into(),
            },
        )
        .await
        .expect("测试项目 Session should start");

    let codelens_readme = kernel
        .create_evidence(
            &codelens.id,
            document("CodeLens-Pro README", r"D:\projects\codelens-pro\README.md"),
        )
        .await
        .expect("CodeLens-Pro README Evidence should be created");
    let codelens_demo = kernel
        .create_evidence(&codelens.id, manual_note("CodeLens-Pro demo screenshot"))
        .await
        .expect("CodeLens-Pro demo Evidence should be created");
    let forgeos_architecture = kernel
        .create_evidence(
            &forgeos.id,
            document("ForgeOS architecture", "docs/architecture.md"),
        )
        .await
        .expect("ForgeOS architecture Evidence should be created");
    let forgeos_tests = kernel
        .create_evidence(&forgeos.id, manual_note("ForgeOS test result"))
        .await
        .expect("ForgeOS test Evidence should be created");
    let test_document = kernel
        .create_evidence(
            &test_project.id,
            document("Test fixture", r"D:\fixtures\evidence.txt"),
        )
        .await
        .expect("测试项目 document Evidence should be created");
    let test_note = kernel
        .create_evidence(&test_project.id, manual_note("Manual acceptance note"))
        .await
        .expect("测试项目 note Evidence should be created");

    for project_id in [&codelens.id, &forgeos.id, &test_project.id] {
        let evidence = kernel
            .list_evidences(project_id)
            .await
            .expect("Project Evidence should list");
        assert_eq!(evidence.len(), 2);
        assert!(evidence
            .iter()
            .all(|item| item.verification_status() == VerificationStatus::Unverified));
    }

    kernel
        .link_evidence(
            &codelens.id,
            codelens_readme.id(),
            LinkEvidenceInput {
                target_type: "session".into(),
                target_id: codelens_session.id().into(),
                relation_type: "supports".into(),
            },
        )
        .await
        .expect("Evidence should link to its Session");
    kernel
        .link_evidence(
            &codelens.id,
            codelens_readme.id(),
            LinkEvidenceInput {
                target_type: "snapshot".into(),
                target_id: codelens_snapshot.id().into(),
                relation_type: "supports".into(),
            },
        )
        .await
        .expect("Evidence should link to its Snapshot");
    kernel
        .link_evidence(
            &codelens.id,
            codelens_readme.id(),
            LinkEvidenceInput {
                target_type: "project".into(),
                target_id: codelens.id.clone(),
                relation_type: "relates_to".into(),
            },
        )
        .await
        .expect("Evidence should link to its Project");

    let linked = kernel
        .get_evidence(&codelens.id, codelens_readme.id())
        .await
        .expect("linked Evidence should load");
    assert_eq!(linked.links.len(), 3);

    assert!(matches!(
        kernel
            .link_evidence(
                &codelens.id,
                codelens_readme.id(),
                LinkEvidenceInput {
                    target_type: "snapshot".into(),
                    target_id: forgeos_snapshot.id().into(),
                    relation_type: "supports".into(),
                },
            )
            .await,
        Err(ApplicationError::EvidenceTargetProjectMismatch)
    ));
    assert!(matches!(
        kernel
            .link_evidence(
                &codelens.id,
                codelens_readme.id(),
                LinkEvidenceInput {
                    target_type: "session".into(),
                    target_id: forgeos_session.id().into(),
                    relation_type: "supports".into(),
                },
            )
            .await,
        Err(ApplicationError::EvidenceTargetProjectMismatch)
    ));

    let verified = kernel
        .verify_evidence(
            &forgeos.id,
            forgeos_tests.id(),
            ChangeEvidenceStatusInput {
                expected_version: 1,
            },
        )
        .await
        .expect("Evidence should verify");
    assert_eq!(verified.verification_status(), VerificationStatus::Verified);
    assert_eq!(verified.version(), 2);

    let updated = kernel
        .update_evidence(
            &forgeos.id,
            verified.id(),
            UpdateEvidenceInput {
                title: verified.title().into(),
                description: "cargo test, integrity_check and foreign_key_check passed".into(),
                evidence_type: "test_result".into(),
                source_type: "manual_note".into(),
                source_uri: String::new(),
                expected_version: verified.version(),
            },
        )
        .await
        .expect("Evidence content should update");
    assert_eq!(
        updated.verification_status(),
        VerificationStatus::Unverified
    );
    assert_eq!(updated.version(), 3);

    assert!(matches!(
        kernel
            .invalidate_evidence(
                &forgeos.id,
                updated.id(),
                ChangeEvidenceStatusInput {
                    expected_version: 2,
                },
            )
            .await,
        Err(ApplicationError::EvidenceVersionConflict {
            expected: 2,
            actual: 3
        })
    ));
    let invalid = kernel
        .invalidate_evidence(
            &forgeos.id,
            updated.id(),
            ChangeEvidenceStatusInput {
                expected_version: updated.version(),
            },
        )
        .await
        .expect("Evidence should invalidate");
    assert_eq!(invalid.verification_status(), VerificationStatus::Invalid);

    assert!(matches!(
        kernel
            .get_evidence(&codelens.id, forgeos_architecture.id())
            .await,
        Err(ApplicationError::Validation(
            DomainError::EvidenceProjectMismatch
        ))
    ));

    let invalid_direct_link = sqlx::query(
        "INSERT INTO evidence_links (id, evidence_id, target_type, target_id, relation_type) \
         VALUES ('invalid-cross-project-link', ?, 'snapshot', ?, 'supports')",
    )
    .bind(test_document.id())
    .bind(codelens_snapshot.id())
    .execute(&database.pool)
    .await;
    assert!(
        invalid_direct_link.is_err(),
        "SQLite trigger must reject cross-Project links"
    );

    let invalid_direct_update = sqlx::query(
        "UPDATE evidence_links SET target_id = ? \
         WHERE evidence_id = ? AND target_type = 'snapshot'",
    )
    .bind(forgeos_snapshot.id())
    .bind(codelens_readme.id())
    .execute(&database.pool)
    .await;
    assert!(
        invalid_direct_update.is_err(),
        "SQLite trigger must reject cross-Project link updates"
    );

    let orphan = sqlx::query(
        "INSERT INTO evidences (id, project_id, title, description, type, source_type, \
         source_uri, verification_status, created_at, updated_at, version) \
         VALUES ('orphan', 'missing-project', 'Orphan', '', 'note', 'manual_note', '', \
         'unverified', '2026-07-11T00:00:00Z', '2026-07-11T00:00:00Z', 1)",
    )
    .execute(&database.pool)
    .await;
    assert!(orphan.is_err(), "foreign key must reject orphan Evidence");

    kernel
        .archive_project(&codelens.id, codelens.version)
        .await
        .expect("CodeLens-Pro should archive");
    assert!(matches!(
        kernel
            .create_evidence(&codelens.id, manual_note("Archived write"))
            .await,
        Err(ApplicationError::Validation(
            DomainError::ArchivedProjectCannotAcceptEvidenceChanges
        ))
    ));

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

    let test_note_id = test_note.id().to_owned();
    let test_session_id = test_session.id().to_owned();
    let test_snapshot_id = test_snapshot.id().to_owned();
    database.pool.close().await;
    drop(kernel);

    let reopened = database::connect(&database_path)
        .await
        .expect("database should reopen");
    let reopened_repository = Arc::new(SqliteProjectRepository::new(reopened.pool.clone()));
    let reopened_kernel = ProjectKernel::new(reopened_repository);
    assert_eq!(
        reopened_kernel
            .list_evidences(&codelens.id)
            .await
            .expect("CodeLens-Pro Evidence should persist")
            .len(),
        2
    );
    assert_eq!(
        reopened_kernel
            .get_evidence(&forgeos.id, invalid.id())
            .await
            .expect("invalid status should persist")
            .evidence
            .verification_status(),
        VerificationStatus::Invalid
    );
    assert!(reopened_kernel
        .get_evidence(&test_project.id, &test_note_id)
        .await
        .is_ok());
    assert!(reopened_kernel
        .link_evidence(
            &test_project.id,
            &test_note_id,
            LinkEvidenceInput {
                target_type: "session".into(),
                target_id: test_session_id,
                relation_type: "relates_to".into(),
            },
        )
        .await
        .is_ok());
    assert!(reopened_kernel
        .link_evidence(
            &test_project.id,
            &test_note_id,
            LinkEvidenceInput {
                target_type: "snapshot".into(),
                target_id: test_snapshot_id,
                relation_type: "supports".into(),
            },
        )
        .await
        .is_ok());

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

    let _ = codelens_demo;
    reopened.pool.close().await;
}
