use chrono::{DateTime, Duration, Utc};

use crate::domain::CodexActivityObservation;

const MAX_EPISODE_TURNS: usize = 20;
const MAX_EPISODE_HOURS: i64 = 72;
const EPISODE_GAP_HOURS: i64 = 8;

#[derive(Clone, Debug)]
pub struct ActivityEpisode {
    pub observations: Vec<CodexActivityObservation>,
}

impl ActivityEpisode {
    pub fn start(&self) -> DateTime<Utc> {
        self.observations
            .first()
            .map(|value| value.source_updated_at)
            .unwrap_or_else(Utc::now)
    }

    pub fn end(&self) -> DateTime<Utc> {
        self.observations
            .last()
            .map(|value| value.source_updated_at)
            .unwrap_or_else(Utc::now)
    }

    pub fn is_completed(&self) -> bool {
        self.observations
            .last()
            .is_some_and(|value| value.turn_status == "completed")
    }

    pub fn root_goal(&self) -> Option<&str> {
        self.observations
            .iter()
            .map(|value| value.goal_hint.trim())
            .find(|value| !value.is_empty())
    }

    pub fn latest(&self) -> Option<&CodexActivityObservation> {
        self.observations.last()
    }

    pub fn to_summary_json(&self) -> serde_json::Value {
        serde_json::json!({
            "goal": self.root_goal(),
            "started_at": self.start(),
            "ended_at": self.end(),
            "status": self.latest().map(|value| value.turn_status.as_str()),
            "turn_count": self.observations.len()
        })
    }
}

pub struct ActivityEpisodeBuilder;

impl ActivityEpisodeBuilder {
    pub fn build(observations: &[CodexActivityObservation]) -> Vec<ActivityEpisode> {
        let mut sorted = observations.to_vec();
        sorted.sort_by(|left, right| {
            left.source_updated_at
                .cmp(&right.source_updated_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        let mut episodes: Vec<ActivityEpisode> = Vec::new();
        for observation in sorted {
            let starts_new = episodes.last().is_none_or(|episode| {
                let Some(previous) = episode.observations.last() else {
                    return true;
                };
                previous.thread_id != observation.thread_id
                    || observation.source_updated_at - previous.source_updated_at
                        > Duration::hours(EPISODE_GAP_HOURS)
                    || observation.source_updated_at - episode.start()
                        > Duration::hours(MAX_EPISODE_HOURS)
                    || episode.observations.len() >= MAX_EPISODE_TURNS
                    || !is_continuation_request(&observation.goal_hint)
            });
            if starts_new {
                episodes.push(ActivityEpisode {
                    observations: vec![observation],
                });
            } else if let Some(episode) = episodes.last_mut() {
                episode.observations.push(observation);
            }
        }
        episodes
    }
}

fn is_continuation_request(value: &str) -> bool {
    let value = compact(value).to_ascii_lowercase();
    if value.chars().count() > 160 {
        return false;
    }
    let value = value.trim_matches(|character: char| {
        character.is_whitespace() || "，。！？,.!?：:".contains(character)
    });
    [
        "继续",
        "下一步",
        "确认",
        "按计划执行",
        "测试无误",
        "完成后汇报",
        "continue",
        "next",
        "proceed",
    ]
    .iter()
    .any(|prefix| value == *prefix || value.starts_with(prefix))
}

fn compact(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};

    use super::*;

    fn observation(id: usize, thread: &str, goal: &str, hour: i64) -> CodexActivityObservation {
        let at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::hours(hour);
        CodexActivityObservation::create(
            format!("o-{id}"),
            "p-1".into(),
            "r-1".into(),
            thread.into(),
            format!("t-{id}"),
            "cli".into(),
            "completed".into(),
            goal.into(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            vec![],
            at,
            format!("{:064x}", id + 1),
            at,
        )
        .unwrap()
    }

    #[test]
    fn continuation_and_time_boundaries_are_shared() {
        let values = vec![
            observation(1, "thread-a", "实现功能", 0),
            observation(2, "thread-a", "继续", 1),
            observation(3, "thread-a", "新的目标", 2),
            observation(4, "thread-a", "继续", 11),
        ];
        let episodes = ActivityEpisodeBuilder::build(&values);
        assert_eq!(episodes.len(), 3);
        assert_eq!(episodes[0].observations.len(), 2);
    }
}
