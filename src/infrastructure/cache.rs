use crate::domain::{Org, Repo};
use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::PathBuf;

/// Cache file location following XDG base directory specification
pub fn cache_path() -> Result<PathBuf> {
    let base_dir = dirs::config_dir()
        .context("Could not determine config directory")?;

    let cache_dir = base_dir.join("g");
    std::fs::create_dir_all(&cache_dir)
        .context("Failed to create cache directory")?;

    Ok(cache_dir.join("cache.db"))
}

/// SQLite cache for storing GitHub data
pub struct Cache {
    conn: Connection,
}

impl Cache {
    /// Open or create the cache database
    pub fn open() -> Result<Self> {
        let path = cache_path()?;
        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open cache at {:?}", path))?;

        let cache = Self { conn };
        cache.init_schema()?;
        Ok(cache)
    }

    fn init_schema(&self) -> Result<()> {
        // Helper to execute statements that may return results
        let exec = |sql: &str| -> Result<()> {
            match self.conn.execute(sql, []) {
                Ok(_) => Ok(()),
                Err(rusqlite::Error::ExecuteReturnedResults) => Ok(()),
                Err(e) => Err(e.into()),
            }
        };

        // Set up pragmas for better performance
        exec("PRAGMA journal_mode = WAL")?;
        exec("PRAGMA synchronous = NORMAL")?;
        exec("CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )")?;

        // Create orgs table
        exec("CREATE TABLE IF NOT EXISTS orgs (
                id INTEGER PRIMARY KEY,
                login TEXT UNIQUE NOT NULL,
                name TEXT,
                avatar_url TEXT,
                last_accessed_at TEXT,
                access_count INTEGER DEFAULT 0
            )")?;

        // Create repos table
        exec("CREATE TABLE IF NOT EXISTS repos (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                full_name TEXT UNIQUE NOT NULL,
                owner_id INTEGER NOT NULL,
                owner_login TEXT NOT NULL,
                private BOOLEAN NOT NULL DEFAULT 0,
                description TEXT,
                language TEXT,
                default_branch TEXT,
                last_accessed_at TEXT,
                access_count INTEGER DEFAULT 0
            )")?;

        // Create indexes for faster lookups
        exec("CREATE INDEX IF NOT EXISTS idx_repos_full_name ON repos(full_name)")?;
        exec("CREATE INDEX IF NOT EXISTS idx_repos_last_accessed ON repos(last_accessed_at DESC)")?;
        exec("CREATE INDEX IF NOT EXISTS idx_repos_owner ON repos(owner_id)")?;
        exec("CREATE INDEX IF NOT EXISTS idx_orgs_login ON orgs(login)")?;

        Ok(())
    }

    /// Clear all data from the cache
    pub fn clear(&self) -> Result<()> {
        self.conn.execute("DELETE FROM repos", [])?;
        self.conn.execute("DELETE FROM orgs", [])?;
        self.conn.execute("DELETE FROM metadata", [])?;
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats> {
        let org_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM orgs",
            [],
            |row| row.get(0),
        )?;

        let repo_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM repos",
            [],
            |row| row.get(0),
        )?;

        let size_bytes = std::fs::metadata(cache_path()?)?.len();

        Ok(CacheStats {
            org_count,
            repo_count,
            size_bytes,
        })
    }

    /// Store organizations in the cache (replaces existing)
    pub fn store_orgs(&self, orgs: &[Org]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;

        for org in orgs {
            tx.execute(
                "INSERT OR REPLACE INTO orgs (id, login, name, avatar_url, last_accessed_at, access_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    org.id,
                    &org.login,
                    &org.name,
                    &org.avatar_url,
                    org.last_accessed_at.map(|d| d.to_rfc3339()),
                    org.access_count,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Store repositories in the cache (replaces existing)
    pub fn store_repos(&self, repos: &[Repo]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;

        // Delete repos whose owner_id is not in the new set
        // This prevents foreign key violations when an org is removed
        let valid_owner_ids: Vec<i64> = repos.iter().map(|r| r.owner_id).collect();
        if valid_owner_ids.is_empty() {
            // If no repos, delete all
            tx.execute("DELETE FROM repos", [])?;
        } else {
            let placeholders = valid_owner_ids.iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(",");
            let query = format!("DELETE FROM repos WHERE owner_id NOT IN ({})", placeholders);
            let params = valid_owner_ids.iter()
                .map(|id| id as &dyn rusqlite::ToSql)
                .collect::<Vec<_>>();
            tx.execute(&query, params.as_slice())?;
        }

        for repo in repos {
            tx.execute(
                "INSERT OR REPLACE INTO repos (id, name, full_name, owner_id, owner_login, private, description, language, default_branch, last_accessed_at, access_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    repo.id,
                    &repo.name,
                    &repo.full_name,
                    repo.owner_id,
                    &repo.owner_login,
                    repo.private as i32,
                    &repo.description,
                    &repo.language,
                    &repo.default_branch,
                    repo.last_accessed_at.map(|d| d.to_rfc3339()),
                    repo.access_count,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Load all organizations from the cache
    pub fn load_orgs(&self) -> Result<Vec<Org>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, login, name, avatar_url, last_accessed_at, access_count FROM orgs"
        )?;

        let orgs = stmt.query_map([], |row| {
            Ok(Org {
                id: row.get(0)?,
                login: row.get(1)?,
                name: row.get(2)?,
                avatar_url: row.get(3)?,
                last_accessed_at: row.get::<_, Option<String>>(4)?
                    .map(|s| s.parse().unwrap()),
                access_count: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(orgs)
    }

    /// Load all repositories from the cache
    pub fn load_repos(&self) -> Result<Vec<Repo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, full_name, owner_id, owner_login, private, description, language, default_branch, last_accessed_at, access_count
             FROM repos"
        )?;

        let repos = stmt.query_map([], |row| {
            Ok(Repo {
                id: row.get(0)?,
                name: row.get(1)?,
                full_name: row.get(2)?,
                owner_id: row.get(3)?,
                owner_login: row.get(4)?,
                private: row.get::<_, i32>(5)? != 0,
                description: row.get(6)?,
                language: row.get(7)?,
                default_branch: row.get(8)?,
                last_accessed_at: row.get::<_, Option<String>>(9)?
                    .map(|s| s.parse().unwrap()),
                access_count: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(repos)
    }

    /// Update repo access information
    #[allow(dead_code)]
    pub fn record_repo_access(&self, full_name: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE repos
             SET access_count = access_count + 1,
                 last_accessed_at = ?1
             WHERE full_name = ?2",
            params![now, full_name],
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub org_count: i64,
    pub repo_count: i64,
    pub size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_stats_format() {
        let stats = CacheStats {
            org_count: 5,
            repo_count: 42,
            size_bytes: 12345,
        };
        // Just verify it compiles for serde
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("org_count"));
    }
}
