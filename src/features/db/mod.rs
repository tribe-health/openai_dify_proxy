use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use supabase_rust::Client;
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageJob {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: ImageJobStatus,
    pub prompt: String,
    pub model: String,
    pub size: String,
    pub urls: Option<Vec<String>>,
    pub ipfs_urls: Option<Vec<String>>,
    pub user_id: Option<String>,
    pub callback_url: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageJobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

pub struct DbClient {
    client: Client,
}

impl DbClient {
    pub fn new(supabase_url: String, supabase_key: String) -> Self {
        Self {
            client: Client::new(supabase_url, supabase_key),
        }
    }

    pub async fn create_image_job(
        &self,
        job: &ImageJob,
    ) -> Result<(), Box<dyn Error>> {
        self.client
            .from("image_jobs")
            .insert(serde_json::to_value(job)?)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn update_image_job(
        &self,
        id: Uuid,
        status: ImageJobStatus,
        urls: Option<Vec<String>>,
        ipfs_urls: Option<Vec<String>>,
        error: Option<String>,
    ) -> Result<(), Box<dyn Error>> {
        let mut update = serde_json::json!({
            "status": status,
            "updated_at": Utc::now(),
        });

        if let Some(urls) = urls {
            update["urls"] = serde_json::json!(urls);
        }
        if let Some(ipfs_urls) = ipfs_urls {
            update["ipfs_urls"] = serde_json::json!(ipfs_urls);
        }
        if let Some(error) = error {
            update["error"] = serde_json::json!(error);
        }

        self.client
            .from("image_jobs")
            .eq("id", id.to_string())
            .update(update)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn get_image_job(
        &self,
        id: Uuid,
    ) -> Result<Option<ImageJob>, Box<dyn Error>> {
        let response = self.client
            .from("image_jobs")
            .eq("id", id.to_string())
            .select("*")
            .execute()
            .await?;

        let jobs: Vec<ImageJob> = response.json()?;
        Ok(jobs.into_iter().next())
    }

    pub async fn list_user_jobs(
        &self,
        user_id: &str,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<ImageJob>, Box<dyn Error>> {
        let mut query = self.client
            .from("image_jobs")
            .eq("user_id", user_id)
            .order("created_at.desc");

        if let Some(limit) = limit {
            query = query.limit(limit);
        }
        if let Some(offset) = offset {
            query = query.range(offset, offset + limit.unwrap_or(10));
        }

        let response = query.execute().await?;
        let jobs: Vec<ImageJob> = response.json()?;
        Ok(jobs)
    }
}
