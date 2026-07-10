use std::sync::Arc;

use forgeos_lib::{
    application::{
        project::{CreateProjectInput, UpdateProjectInput},
        workspace::CreateWorkspaceInput,
        ApplicationError, ProjectKernel,
    },
    domain::ProjectStatus,
    infrastructure::{database, repositories::SqliteProjectRepository},
};
use tempfile::TempDir;

fn create_input(
    slug: &str,
    name: &str,
    status: &str,
    lifecycle_stage: &str,
    priority: i32,
) -> CreateProjectInput {
    CreateProjectInput {
        slug: slug.into(),
        name: name.into(),
        description: format!("{name} project description"),
        vision: format!("Long-term vision for {name}"),
        target_user: "Independent developers".into(),
        scope: "Trusted engineering memory".into(),
        non_goals: "Task management".into(),
        tech_stack: "React, TypeScript, Tauri, Rust, SQLite".into(),
        status: status.into(),
        lifecycle_stage: lifecycle_stage.into(),
        priority,
    }
}

#[tokio::test]
async fn project_crud_archive_and_persistence_survive_reopen() {
    let directory = TempDir::new().expect("temporary directory should be created");
    let database_path = directory.path().join("forgeos-test.db");
    let database = database::connect(&database_path)
        .await
        .expect("database should initialize");
    let repository = Arc::new(SqliteProjectRepository::new(database.pool.clone()));
    let kernel = ProjectKernel::new(repository);

    let workspace = kernel
        .create_workspace(CreateWorkspaceInput {
            name: "Personal Engineering".into(),
            timezone: "Asia/Shanghai".into(),
        })
        .await
        .expect("the single workspace should be created");
    assert_eq!(workspace.name, "Personal Engineering");
    assert_eq!(
        kernel
            .get_current_workspace()
            .await
            .expect("workspace lookup should work")
            .expect("workspace should exist")
            .id,
        workspace.id
    );

    assert!(matches!(
        kernel
            .create_workspace(CreateWorkspaceInput {
                name: "Second Workspace".into(),
                timezone: "UTC".into(),
            })
            .await,
        Err(ApplicationError::WorkspaceAlreadyExists)
    ));

    let codelens = kernel
        .create_project(create_input(
            "codelens-pro",
            "CodeLens-Pro",
            "planned",
            "design",
            80,
        ))
        .await
        .expect("CodeLens-Pro should be created");
    let forgeos = kernel
        .create_project(create_input(
            "forgeos",
            "ForgeOS",
            "active",
            "implementation",
            100,
        ))
        .await
        .expect("ForgeOS should be created");
    let test_project = kernel
        .create_project(create_input(
            "test-project",
            "测试项目",
            "idea",
            "discovery",
            20,
        ))
        .await
        .expect("测试项目 should be created");

    let projects = kernel
        .list_projects()
        .await
        .expect("project list should load");
    assert_eq!(projects.len(), 3);
    assert!(projects
        .iter()
        .any(|project| project.name == "CodeLens-Pro"));
    assert!(projects.iter().any(|project| project.name == "ForgeOS"));
    assert!(projects.iter().any(|project| project.name == "测试项目"));

    let detail = kernel
        .get_project(&forgeos.id)
        .await
        .expect("ForgeOS detail should load");
    assert_eq!(detail.vision, "Long-term vision for ForgeOS");
    assert_eq!(detail.lifecycle_stage.as_str(), "implementation");

    let updated = kernel
        .update_project(
            &codelens.id,
            UpdateProjectInput {
                slug: codelens.slug.clone(),
                name: codelens.name.clone(),
                description: "AI-assisted source code understanding".into(),
                vision: codelens.vision.clone(),
                target_user: codelens.target_user.clone(),
                scope: codelens.scope.clone(),
                non_goals: codelens.non_goals.clone(),
                tech_stack: codelens.tech_stack.clone(),
                status: "active".into(),
                lifecycle_stage: "implementation".into(),
                priority: 90,
                expected_version: codelens.version,
            },
        )
        .await
        .expect("CodeLens-Pro should update");
    assert_eq!(updated.description, "AI-assisted source code understanding");
    assert_eq!(updated.version, 2);
    assert_eq!(updated.status, ProjectStatus::Active);

    assert!(matches!(
        kernel
            .update_project(
                &updated.id,
                UpdateProjectInput {
                    slug: updated.slug.clone(),
                    name: updated.name.clone(),
                    description: updated.description.clone(),
                    vision: updated.vision.clone(),
                    target_user: updated.target_user.clone(),
                    scope: updated.scope.clone(),
                    non_goals: updated.non_goals.clone(),
                    tech_stack: updated.tech_stack.clone(),
                    status: "active".into(),
                    lifecycle_stage: "implementation".into(),
                    priority: updated.priority,
                    expected_version: 1,
                },
            )
            .await,
        Err(ApplicationError::VersionConflict {
            expected: 1,
            actual: 2
        })
    ));

    let archived = kernel
        .archive_project(&test_project.id, test_project.version)
        .await
        .expect("测试项目 should archive");
    assert_eq!(archived.status, ProjectStatus::Archived);
    assert_eq!(archived.version, 2);

    assert!(matches!(
        kernel
            .update_project(
                &archived.id,
                UpdateProjectInput {
                    slug: archived.slug.clone(),
                    name: archived.name.clone(),
                    description: archived.description.clone(),
                    vision: archived.vision.clone(),
                    target_user: archived.target_user.clone(),
                    scope: archived.scope.clone(),
                    non_goals: archived.non_goals.clone(),
                    tech_stack: archived.tech_stack.clone(),
                    status: "active".into(),
                    lifecycle_stage: archived.lifecycle_stage.as_str().into(),
                    priority: archived.priority,
                    expected_version: archived.version,
                },
            )
            .await,
        Err(ApplicationError::Validation(_))
    ));

    database.pool.close().await;
    drop(kernel);

    let reopened = database::connect(&database_path)
        .await
        .expect("database should reopen and rerun migrations idempotently");
    let reopened_repository = Arc::new(SqliteProjectRepository::new(reopened.pool.clone()));
    let reopened_kernel = ProjectKernel::new(reopened_repository);
    let persisted = reopened_kernel
        .list_projects()
        .await
        .expect("projects should persist after reopen");
    assert_eq!(persisted.len(), 3);
    assert_eq!(
        reopened_kernel
            .get_project(&codelens.id)
            .await
            .expect("updated project should persist")
            .version,
        2
    );
    assert_eq!(
        reopened_kernel
            .get_project(&test_project.id)
            .await
            .expect("archived project should persist")
            .status,
        ProjectStatus::Archived
    );

    reopened.pool.close().await;
}
