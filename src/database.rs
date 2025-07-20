use sqlx::sqlite::SqlitePool;
use crate::models::{Image, Stack, RepositoryCache};

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        // Create database file if it doesn't exist
        if database_url.starts_with("sqlite:") {
            let db_path = database_url.trim_start_matches("sqlite:");
            if !std::path::Path::new(db_path).exists() {
                // Create empty database file
                std::fs::File::create(db_path)?;
            }
        }
        
        let pool = SqlitePool::connect(database_url).await?;
        Self::migrate(&pool).await?;
        Ok(Self { pool })
    }

    async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS images (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                reference_count INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(pool)
        .await?;



        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS stacks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                repository_url TEXT NOT NULL,
                compose_path TEXT NOT NULL,
                hash TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'stopped',
                UNIQUE(name, repository_url)
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS repository_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL UNIQUE,
                last_watch TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    // Image operations
    pub async fn create_image(&self, image: &Image) -> Result<i64, sqlx::Error> {
        let id = sqlx::query(
            "INSERT OR IGNORE INTO images (name, reference_count) VALUES (?, ?)"
        )
        .bind(&image.name)
        .bind(image.reference_count)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();

        Ok(id)
    }

    pub async fn get_image_by_name(&self, name: &str) -> Result<Option<Image>, sqlx::Error> {
        let row = sqlx::query_as::<_, Image>(
            "SELECT id, name, reference_count FROM images WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn update_image_reference_count(&self, name: &str, count: i32) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE images SET reference_count = ? WHERE name = ?")
            .bind(count)
            .bind(name)
            .execute(&self.pool)
            .await?;

        Ok(())
    }





    // Stack operations
    pub async fn create_stack(&self, stack: &Stack) -> Result<i64, sqlx::Error> {
        let id = sqlx::query(
            "INSERT OR REPLACE INTO stacks (name, repository_url, compose_path, hash, status) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&stack.name)
        .bind(&stack.repository_url)
        .bind(&stack.compose_path)
        .bind(&stack.hash)
        .bind(&stack.status)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();

        Ok(id)
    }

    pub async fn get_stack_by_name(&self, name: &str, repository_url: &str) -> Result<Option<Stack>, sqlx::Error> {
        let row = sqlx::query_as::<_, Stack>(
            "SELECT id, name, repository_url, compose_path, hash, status FROM stacks WHERE name = ? AND repository_url = ?"
        )
        .bind(name)
        .bind(repository_url)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn get_all_stacks(&self) -> Result<Vec<Stack>, sqlx::Error> {
        let stacks = sqlx::query_as::<_, Stack>(
            "SELECT id, name, repository_url, compose_path, hash, status FROM stacks ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(stacks)
    }

    pub async fn update_stack_status(&self, name: &str, repository_url: &str, status: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE stacks SET status = ? WHERE name = ? AND repository_url = ?")
            .bind(status)
            .bind(name)
            .bind(repository_url)
            .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_stack_hash(&self, name: &str, repository_url: &str, hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE stacks SET hash = ? WHERE name = ? AND repository_url = ?")
            .bind(hash)
            .bind(name)
            .bind(repository_url)
            .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_all_stacks(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM stacks")
            .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Repository cache operations
    pub async fn add_repository_to_cache(&self, url: &str) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR REPLACE INTO repository_cache (url, last_watch) VALUES (?, ?)"
        )
        .bind(url)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_repository_from_cache(&self, url: &str) -> Result<Option<RepositoryCache>, sqlx::Error> {
        println!("Debug: Querying cache for URL: {}", url);
        
        let row = sqlx::query_as::<_, RepositoryCache>(
            "SELECT id, url, last_watch FROM repository_cache WHERE url = ?"
        )
        .bind(url)
        .fetch_optional(&self.pool)
        .await?;

        match &row {
            Some(repo) => println!("Debug: Found repository in database: {} (last watch: {})", repo.url, repo.last_watch),
            None => println!("Debug: No repository found in database for URL: {}", url),
        }

        Ok(row)
    }

    pub async fn get_all_repositories(&self) -> Result<Vec<RepositoryCache>, sqlx::Error> {
        let repositories = sqlx::query_as::<_, RepositoryCache>(
            "SELECT id, url, last_watch FROM repository_cache ORDER BY last_watch DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(repositories)
    }

    pub async fn clear_repository_cache(&self) -> Result<(), sqlx::Error> {
        // Use a transaction to ensure the deletion is committed
        let mut tx = self.pool.begin().await?;
        
        sqlx::query("DELETE FROM repository_cache")
            .execute(&mut *tx)
            .await?;
        
        // Commit the transaction
        tx.commit().await?;

        Ok(())
    }

    // Image management operations
    pub async fn reset_image_reference_counts(&self) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE images SET reference_count = 0")
            .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_images_with_zero_count(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM images WHERE reference_count = 0")
            .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_all_images(&self) -> Result<Vec<Image>, sqlx::Error> {
        let images = sqlx::query_as::<_, Image>(
            "SELECT id, name, reference_count FROM images ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(images)
    }
}