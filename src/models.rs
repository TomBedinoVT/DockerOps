use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Image {
    pub id: i64,
    pub name: String,
    pub reference_count: i32,
}



#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Stack {
    pub id: i64,
    pub name: String,
    pub repository_url: String,
    pub compose_path: String,
    pub hash: String,
    pub status: String, // "deployed", "stopped", "error"
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct RepositoryCache {
    pub id: i64,
    pub url: String,
    pub last_watch: String, // ISO timestamp
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StackDefinition {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VolumeDefinition {
    pub id: String,
    pub r#type: VolumeType,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VolumeType {
    Volume,
    Binding,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NfsConfig {
    pub path: String,
}

impl Image {
    pub fn new(name: String, reference_count: i32) -> Self {
        Self {
            id: 0, // Will be set by database
            name,
            reference_count,
        }
    }
}



impl Stack {
    pub fn new(name: String, repository_url: String, compose_path: String, hash: String) -> Self {
        Self {
            id: 0, // Will be set by database
            name,
            repository_url,
            compose_path,
            hash,
            status: "stopped".to_string(),
        }
    }
} 