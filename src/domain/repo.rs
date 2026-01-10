use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// GitHub Organization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Org {
    pub id: i64,
    pub login: String,
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub last_accessed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub access_count: u32,
}

impl Org {
    pub fn new(id: i64, login: String, name: Option<String>, avatar_url: Option<String>) -> Self {
        Self {
            id,
            login,
            name,
            avatar_url,
            last_accessed_at: None,
            access_count: 0,
        }
    }
}

/// GitHub Repository
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Repo {
    pub id: i64,
    pub name: String,
    pub full_name: String, // "org/repo"
    pub owner_id: i64,
    pub owner_login: String,
    pub private: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
    #[serde(default)]
    pub last_accessed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub access_count: u32,
}

impl Repo {
    pub fn new(
        id: i64,
        name: String,
        full_name: String,
        owner_id: i64,
        owner_login: String,
        private: bool,
        description: Option<String>,
        language: Option<String>,
        default_branch: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            full_name,
            owner_id,
            owner_login,
            private,
            description,
            language,
            default_branch,
            last_accessed_at: None,
            access_count: 0,
        }
    }

    /// Calculate a usage score for sorting (higher = more frequently used)
    #[allow(dead_code)]
    pub fn score(&self) -> f64 {
        let days_since = match self.last_accessed_at {
            Some(last) => (Utc::now() - last).num_days().max(0) as f64,
            None => 30.0, // Never accessed: treat as 30 days ago
        };
        self.access_count as f64 / (days_since + 1.0)
    }

    /// Record an access event (increments count and updates timestamp)
    #[allow(dead_code)]
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_score_never_accessed() {
        let repo = Repo::new(
            1,
            "test".to_string(),
            "org/test".to_string(),
            1,
            "org".to_string(),
            false,
            None,
            None,
            None,
        );
        assert_eq!(repo.score(), 0.0);
    }

    #[test]
    fn test_repo_score_with_access() {
        let mut repo = Repo::new(
            1,
            "test".to_string(),
            "org/test".to_string(),
            1,
            "org".to_string(),
            false,
            None,
            None,
            None,
        );
        repo.access_count = 10;
        repo.last_accessed_at = Some(Utc::now() - chrono::Duration::days(2));
        // Score = 10 / (2 + 1) = 3.33
        assert!((repo.score() - 3.33).abs() < 0.01);
    }

    #[test]
    fn test_repo_record_access() {
        let mut repo = Repo::new(
            1,
            "test".to_string(),
            "org/test".to_string(),
            1,
            "org".to_string(),
            false,
            None,
            None,
            None,
        );
        repo.record_access();
        assert_eq!(repo.access_count, 1);
        assert!(repo.last_accessed_at.is_some());
    }
}
