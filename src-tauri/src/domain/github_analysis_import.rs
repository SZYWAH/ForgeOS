use chrono::{DateTime, Utc};

use super::{DomainError, ProjectDetails};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubAnalysisImportProposal {
    pub draft_id: String,
    pub workspace_id: String,
    pub analysis_run_id: String,
    pub source_commit_sha: String,
    pub source_content_hash: String,
    pub project: ProjectDetails,
    pub partial_acknowledged: bool,
    pub stale_acknowledged: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

impl GitHubAnalysisImportProposal {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        draft_id: String,
        workspace_id: String,
        analysis_run_id: String,
        source_commit_sha: String,
        source_content_hash: String,
        project: ProjectDetails,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(
            draft_id,
            workspace_id,
            analysis_run_id,
            source_commit_sha,
            source_content_hash,
            project,
            false,
            false,
            now,
            now,
            1,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        draft_id: String,
        workspace_id: String,
        analysis_run_id: String,
        source_commit_sha: String,
        source_content_hash: String,
        project: ProjectDetails,
        partial_acknowledged: bool,
        stale_acknowledged: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        version: i64,
    ) -> Result<Self, DomainError> {
        if draft_id.trim().is_empty()
            || workspace_id.trim().is_empty()
            || analysis_run_id.trim().is_empty()
            || !(7..=64).contains(&source_commit_sha.len())
            || source_content_hash.len() != 64
            || !source_content_hash
                .bytes()
                .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
            || version < 1
            || updated_at < created_at
        {
            return Err(DomainError::InvalidGitHubAnalysisImportProposal);
        }
        Ok(Self {
            draft_id,
            workspace_id,
            analysis_run_id,
            source_commit_sha,
            source_content_hash,
            project: project.validated()?,
            partial_acknowledged,
            stale_acknowledged,
            created_at,
            updated_at,
            version,
        })
    }

    pub fn update(
        &mut self,
        project: ProjectDetails,
        partial_acknowledged: bool,
        stale_acknowledged: bool,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.project = project.validated()?;
        self.partial_acknowledged = partial_acknowledged;
        self.stale_acknowledged = stale_acknowledged;
        self.bump(now)
    }

    pub fn replace_source(
        &mut self,
        analysis_run_id: String,
        source_commit_sha: String,
        source_content_hash: String,
        project: ProjectDetails,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let replacement = Self::restore(
            self.draft_id.clone(),
            self.workspace_id.clone(),
            analysis_run_id,
            source_commit_sha,
            source_content_hash,
            project,
            false,
            false,
            self.created_at,
            now.max(self.updated_at),
            self.version
                .checked_add(1)
                .ok_or(DomainError::InvalidGitHubAnalysisImportProposal)?,
        )?;
        *self = replacement;
        Ok(())
    }

    fn bump(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.updated_at = now.max(self.updated_at);
        self.version = self
            .version
            .checked_add(1)
            .ok_or(DomainError::InvalidGitHubAnalysisImportProposal)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{LifecycleStage, ProjectStatus};

    fn details() -> ProjectDetails {
        ProjectDetails {
            slug: "forge-os".into(),
            name: "ForgeOS".into(),
            description: "Project memory".into(),
            vision: String::new(),
            target_user: String::new(),
            scope: "GitHub analysis".into(),
            non_goals: String::new(),
            tech_stack: "Rust, TypeScript".into(),
            status: ProjectStatus::Active,
            lifecycle_stage: LifecycleStage::Definition,
            priority: 50,
        }
    }

    #[test]
    fn proposal_updates_with_optimistic_version() {
        let now = Utc::now();
        let mut proposal = GitHubAnalysisImportProposal::create(
            "draft".into(),
            "workspace".into(),
            "run".into(),
            "abcdef1".into(),
            "a".repeat(64),
            details(),
            now,
        )
        .expect("proposal");
        proposal.update(details(), true, true, now).expect("update");
        assert_eq!(proposal.version, 2);
        assert!(proposal.partial_acknowledged);
        assert!(proposal.stale_acknowledged);
    }

    #[test]
    fn replacement_resets_acknowledgements() {
        let now = Utc::now();
        let mut proposal = GitHubAnalysisImportProposal::create(
            "draft".into(),
            "workspace".into(),
            "run-1".into(),
            "abcdef1".into(),
            "a".repeat(64),
            details(),
            now,
        )
        .expect("proposal");
        proposal.update(details(), true, true, now).expect("update");
        proposal
            .replace_source(
                "run-2".into(),
                "abcdef2".into(),
                "b".repeat(64),
                details(),
                now,
            )
            .expect("replace");
        assert!(!proposal.partial_acknowledged);
        assert!(!proposal.stale_acknowledged);
    }
}
