use std::{env, path::PathBuf, sync::Arc};

use anyhow::{bail, Context};
use forgeos_lib::{
    application::ProjectMemoryQueryKernel,
    infrastructure::{database, repositories::SqliteProjectRepository},
};
use rmcp::{transport::stdio, ServiceExt};

use forgeos_mcp::server::ForgeOsMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_path = resolve_database_path()?;
    let database = database::connect_read_only(&database_path)
        .await
        .with_context(|| "unable to open the ForgeOS database in read-only mode")?;
    let repository = Arc::new(SqliteProjectRepository::new(database.pool));
    let kernel = Arc::new(ProjectMemoryQueryKernel::new(repository));

    // stdout belongs exclusively to the MCP STDIO transport.
    let service = ForgeOsMcpServer::new(kernel).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

fn resolve_database_path() -> anyhow::Result<PathBuf> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    let explicit = match args.as_slice() {
        [] => None,
        [flag, path] if flag == "--database" => Some(PathBuf::from(path)),
        _ => bail!("usage: forgeos-mcp [--database <path>]"),
    };
    let path = explicit
        .or_else(|| env::var_os("FORGEOS_DATABASE_PATH").map(PathBuf::from))
        .or_else(default_database_path)
        .context("database path is unavailable; pass --database or set FORGEOS_DATABASE_PATH")?;
    if !path.is_file() {
        bail!("ForgeOS database does not exist at the selected path")
    }
    Ok(path)
}

fn default_database_path() -> Option<PathBuf> {
    env::var_os("APPDATA").map(|app_data| {
        PathBuf::from(app_data)
            .join("com.forgeos.desktop")
            .join("database")
            .join("forgeos.db")
    })
}
