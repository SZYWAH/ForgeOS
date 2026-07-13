use async_trait::async_trait;

use crate::ports::{CodexExecutionRequest, CodexRunner, CodexRunnerError};

/// D5 runner implementation.
///
/// It deliberately accepts metadata without launching a process, reading a
/// repository, executing validation commands, or changing any file.
#[derive(Clone, Default)]
pub struct RecordOnlyCodexRunner;

#[async_trait]
impl CodexRunner for RecordOnlyCodexRunner {
    async fn accept(&self, _request: &CodexExecutionRequest) -> Result<(), CodexRunnerError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn accepts_metadata_without_execution_side_effects() {
        let runner = RecordOnlyCodexRunner;
        runner
            .accept(&CodexExecutionRequest {
                execution_id: "execution-1".into(),
                proposal_id: "proposal-1".into(),
                project_id: "project-1".into(),
                context_export_id: "context-1".into(),
                title: "Record only".into(),
                goal: "Prove the boundary".into(),
                allowed_scope: vec![],
                forbidden_scope: vec!["**/*".into()],
                acceptance_criteria: vec!["No process launched".into()],
                validation_commands: vec![],
            })
            .await
            .unwrap();
    }
}
