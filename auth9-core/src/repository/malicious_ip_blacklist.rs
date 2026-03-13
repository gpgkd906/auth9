use crate::error::Result;
use crate::models::common::StringUuid;
use crate::models::system_settings::MaliciousIpBlacklistEntry;
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait MaliciousIpBlacklistRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<MaliciousIpBlacklistEntry>>;
    async fn replace_all(
        &self,
        entries: &[MaliciousIpBlacklistEntry],
        created_by: Option<StringUuid>,
    ) -> Result<Vec<MaliciousIpBlacklistEntry>>;
    async fn find_by_ip(&self, ip_address: &str) -> Result<Option<MaliciousIpBlacklistEntry>>;
}

pub struct MaliciousIpBlacklistRepositoryImpl {
    pool: MySqlPool,
}

impl MaliciousIpBlacklistRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MaliciousIpBlacklistRepository for MaliciousIpBlacklistRepositoryImpl {
    async fn list(&self) -> Result<Vec<MaliciousIpBlacklistEntry>> {
        let rows = sqlx::query_as::<_, MaliciousIpBlacklistEntry>(
            r#"
            SELECT id, ip_address, reason, created_by, created_at, updated_at
            FROM malicious_ip_blacklist
            ORDER BY ip_address ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn replace_all(
        &self,
        entries: &[MaliciousIpBlacklistEntry],
        created_by: Option<StringUuid>,
    ) -> Result<Vec<MaliciousIpBlacklistEntry>> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM malicious_ip_blacklist")
            .execute(&mut *tx)
            .await?;

        for entry in entries {
            sqlx::query(
                r#"
                INSERT INTO malicious_ip_blacklist (id, ip_address, reason, created_by, created_at, updated_at)
                VALUES (?, ?, ?, ?, NOW(), NOW())
                "#,
            )
            .bind(entry.id)
            .bind(&entry.ip_address)
            .bind(&entry.reason)
            .bind(created_by)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.list().await
    }

    async fn find_by_ip(&self, ip_address: &str) -> Result<Option<MaliciousIpBlacklistEntry>> {
        let row = sqlx::query_as::<_, MaliciousIpBlacklistEntry>(
            r#"
            SELECT id, ip_address, reason, created_by, created_at, updated_at
            FROM malicious_ip_blacklist
            WHERE ip_address = ?
            LIMIT 1
            "#,
        )
        .bind(ip_address)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }
}
