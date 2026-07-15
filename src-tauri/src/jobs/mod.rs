//! D6 runs project activity synchronization only while the ForgeOS desktop
//! process is alive. It never installs a Windows service or executes Codex.
use std::time::{Duration, SystemTime};

use serde_json::json;
use tauri::{AppHandle, Emitter};

use crate::{application::ProjectPulseKernel, domain::SyncTrigger};

/// Runs only while the desktop process is alive. The one-minute cadence also
/// provides a conservative delay after Windows resumes from sleep.
pub fn spawn_scheduler(app: AppHandle, kernel: ProjectPulseKernel) {
    tauri::async_runtime::spawn(async move {
        let mut last_tick = SystemTime::now();
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let now = SystemTime::now();
            if now
                .duration_since(last_tick)
                .is_ok_and(|elapsed| elapsed > Duration::from_secs(120))
            {
                // A large wall-clock gap usually means Windows resumed from
                // sleep. Give repositories and Codex one minute to settle.
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
            last_tick = SystemTime::now();
            let Ok(project_ids) = kernel.interval_projects_due().await else {
                continue;
            };
            for project_id in project_ids {
                let Ok(run) = kernel.queue_sync(&project_id, SyncTrigger::Interval).await else {
                    continue;
                };
                let run_id = run.id.clone();
                let progress_app = app.clone();
                let result = kernel
                    .execute_sync(&run_id, move |run| {
                        let _ = progress_app.emit(
                            "project-pulse://sync-progress",
                            json!({
                                "projectId": run.project_id,
                                "syncRunId": run.id,
                                "stage": run.stage.as_str(),
                                "status": run.status.as_str(),
                                "message": "自动同步正在运行",
                                "threadsScanned": run.threads_scanned,
                                "turnsImported": run.turns_imported,
                                "observationsCreated": run.observations_created
                            }),
                        );
                    })
                    .await;
                let _ = app.emit(
                    "project-pulse://sync-completed",
                    json!({
                        "projectId": project_id,
                        "syncRunId": run_id,
                        "status": if result.is_ok() { "completed" } else { "failed" }
                    }),
                );
            }
        }
    });
}
