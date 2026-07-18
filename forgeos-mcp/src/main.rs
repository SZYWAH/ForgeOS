use std::{env, path::PathBuf, sync::Arc};

use anyhow::{bail, Context};
use forgeos_lib::{
    application::ProjectMemoryQueryKernel,
    infrastructure::{database, repositories::SqliteProjectRepository},
};
use rmcp::{transport::stdio, ServiceExt};

use forgeos_mcp::server::ForgeOsMcpServer;

enum RunMode {
    Serve(Option<PathBuf>),
    CheckDatabase(Option<PathBuf>),
    Version,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mode = parse_mode(env::args_os().skip(1).collect())?;
    if matches!(mode, RunMode::Version) {
        println!("forgeos-mcp {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let explicit = match &mode {
        RunMode::Serve(path) | RunMode::CheckDatabase(path) => path.clone(),
        RunMode::Version => unreachable!(),
    };
    let database_path = resolve_database_path(explicit)?;
    let database = database::connect_read_only(&database_path)
        .await
        .with_context(|| "unable to open the ForgeOS database in read-only mode")?;
    if matches!(mode, RunMode::CheckDatabase(_)) {
        let result: String = sqlx::query_scalar("PRAGMA quick_check")
            .fetch_one(&database.pool)
            .await
            .with_context(|| "unable to validate the ForgeOS database")?;
        if result != "ok" {
            bail!("ForgeOS database check returned {result}")
        }
        // Diagnostics must not disclose the user's absolute database path.
        println!("database ok");
        return Ok(());
    }
    let repository = Arc::new(SqliteProjectRepository::new(database.pool));
    let kernel = Arc::new(ProjectMemoryQueryKernel::new(repository));

    // stdout belongs exclusively to the MCP STDIO transport.
    let service = ForgeOsMcpServer::new(kernel).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

fn parse_mode(args: Vec<std::ffi::OsString>) -> anyhow::Result<RunMode> {
    match args.as_slice() {
        [] => Ok(RunMode::Serve(None)),
        [flag] if flag == "--version" => Ok(RunMode::Version),
        [flag] if flag == "--check-database" => Ok(RunMode::CheckDatabase(None)),
        [flag, path] if flag == "--database" => {
            Ok(RunMode::Serve(Some(PathBuf::from(path))))
        }
        [check, database, path] if check == "--check-database" && database == "--database" => {
            Ok(RunMode::CheckDatabase(Some(PathBuf::from(path))))
        }
        _ => bail!("usage: forgeos-mcp [--version | --check-database [--database <path>] | --database <path>]")
    }
}

fn resolve_database_path(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let path = explicit
        .or_else(|| env::var_os("FORGEOS_DATABASE_PATH").map(PathBuf::from))
        .or_else(default_database_path)
        .context("database path is unavailable; pass --database or set FORGEOS_DATABASE_PATH")?;
    if !path.is_file() {
        bail!("ForgeOS database does not exist at the selected path")
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_modes_do_not_change_default_stdio_mode() {
        assert!(matches!(
            parse_mode(Vec::new()).unwrap(),
            RunMode::Serve(None)
        ));
        assert!(matches!(
            parse_mode(vec!["--version".into()]).unwrap(),
            RunMode::Version
        ));
        assert!(matches!(
            parse_mode(vec!["--check-database".into()]).unwrap(),
            RunMode::CheckDatabase(None)
        ));
        assert!(parse_mode(vec!["--unknown".into()]).is_err());
    }
}

fn default_database_path() -> Option<PathBuf> {
    env::var_os("APPDATA").map(|app_data| {
        PathBuf::from(app_data)
            .join("com.forgeos.desktop")
            .join("database")
            .join("forgeos.db")
    })
}
