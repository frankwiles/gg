use crate::domain::{Org, Repo};
use nucleo::{
    pattern::{CaseMatching, Normalization},
    Config, Utf32String,
};
use std::sync::Arc;

/// A repo item that can be fuzzy matched
#[derive(Debug, Clone)]
pub struct RepoItem {
    /// Display name (org/repo format)
    pub full_name: String,
    /// Repository data from cache
    pub repo: Repo,
    /// GitHub URL for opening in browser
    pub url: String,
}

impl RepoItem {
    pub fn new(repo: Repo) -> Self {
        let url = format!("https://github.com/{}", repo.full_name);
        Self {
            full_name: repo.full_name.clone(),
            repo,
            url,
        }
    }
}

/// Fuzzy matcher for repositories using nucleo
pub struct RepoMatcher {
    /// Nucleo matcher worker (runs matching in background)
    nucleo: nucleo::Nucleo<String>,
    /// Current pattern string
    pattern: String,
    /// All repo items for lookup by index
    items: Vec<RepoItem>,
}

impl RepoMatcher {
    /// Create a new matcher from the given repos and orgs
    pub fn new(repos: Vec<Repo>, orgs: Vec<Org>) -> Self {
        let config = Config::DEFAULT;

        // Create the nucleo matcher
        let nucleo = nucleo::Nucleo::new(
            config,
            Arc::new(|| {}), // No notification needed
            None,            // Use default number of threads
            1,               // Number of columns for display (must be at least 1)
        );

        // Collect all full_names for injection
        let mut all_names: Vec<String> = Vec::new();
        for repo in &repos {
            all_names.push(repo.full_name.clone());
        }
        for org in &orgs {
            all_names.push(format!("{}/", org.login));
        }

        // Inject items
        let injector = nucleo.injector();
        for name in all_names {
            injector.push(name, |data, columns| {
                // Fill the first column with the data for matching
                columns[0] = Utf32String::from(data.as_str());
            });
        }

        // Create items map for lookup
        let mut items = Vec::new();
        for repo in repos {
            items.push(RepoItem::new(repo));
        }
        for org in orgs {
            // Create a pseudo-repo item for the org
            let pseudo_repo = Repo {
                id: org.id,
                name: String::new(),
                full_name: format!("{}/", org.login),
                owner_id: org.id,
                owner_login: org.login.clone(),
                private: false,
                description: None,
                language: None,
                default_branch: None,
                last_accessed_at: org.last_accessed_at,
                access_count: org.access_count,
            };
            items.push(RepoItem::new(pseudo_repo));
        }

        Self {
            nucleo,
            pattern: String::new(),
            items,
        }
    }

    /// Update the search pattern
    pub fn update_pattern(&mut self, pattern: String) {
        self.pattern = pattern.clone();
        self.nucleo.pattern.reparse(
            0,                              // column index
            &pattern,
            CaseMatching::Ignore,
            Normalization::Smart,
            true, // append
        );
    }

    /// Tick the matcher (process pending pattern changes)
    pub fn tick(&mut self) {
        self.nucleo.tick(100); // 100ms timeout
    }

    /// Get the current matches as a sorted vector
    pub fn matches_sorted(&self) -> Vec<&RepoItem> {
        let snapshot = self.nucleo.snapshot();
        let matched_count = snapshot.matched_item_count();

        let mut matches: Vec<_> = snapshot
            .matched_items(0..matched_count)
            .filter_map(|item| {
                // Find the corresponding RepoItem by matching the full_name
                self.items.iter().find(|ri| &ri.full_name == item.data).map(|ri| {
                    // For now, use a default fuzzy score since Item doesn't have a score field
                    // In a more sophisticated implementation, we could use matcher_columns
                    (ri, 100.0_f64)
                })
            })
            .collect();

        // Sort by combined score (fuzzy match score + usage score)
        matches.sort_by(|a, b| {
            let score_a = self.combined_score(a.0, a.1);
            let score_b = self.combined_score(b.0, b.1);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        matches.into_iter().map(|(item, _)| item).collect()
    }

    /// Get the number of matches
    pub fn match_count(&self) -> usize {
        let snapshot = self.nucleo.snapshot();
        snapshot.matched_item_count() as usize
    }

    /// Calculate combined score from fuzzy match and usage
    fn combined_score(&self, item: &RepoItem, fuzzy_score: f64) -> f64 {
        // Usage-based score from the repo
        let usage_score = item.repo.score();

        // Combined score: prioritize fuzzy match but also consider usage
        // Scale usage_score to a reasonable range (0-30 points bonus)
        let usage_bonus = (usage_score * 10.0).min(30.0);
        fuzzy_score + usage_bonus
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_repo(full_name: &str, access_count: u32, days_since_access: i64) -> Repo {
        let parts: Vec<&str> = full_name.split('/').collect();
        let owner_login = parts[0].to_string();
        let name = parts.get(1).unwrap_or(&"").to_string();

        Repo {
            id: 1,
            name,
            full_name: full_name.to_string(),
            owner_id: 1,
            owner_login,
            private: false,
            description: None,
            language: None,
            default_branch: None,
            last_accessed_at: Some(Utc::now() - chrono::Duration::days(days_since_access)),
            access_count,
        }
    }

    #[test]
    fn test_repo_item_creation() {
        let repo = create_test_repo("facebook/react", 10, 1);
        let item = RepoItem::new(repo.clone());

        assert_eq!(item.full_name, "facebook/react");
        assert_eq!(item.url, "https://github.com/facebook/react");
    }
}
