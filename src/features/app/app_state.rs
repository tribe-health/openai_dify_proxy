use std::sync::Arc;
use tokio::sync::{RwLock, Notify};
use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::features::db::DbClient;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageTaskResult {
    pub urls: Vec<String>,
    pub ipfs_urls: Vec<String>,
}

#[derive(Clone)]
pub struct TaskData {
    pub callback_url: Option<String>,
    pub notify: Arc<Notify>,
    pub result: Arc<RwLock<Option<ImageTaskResult>>>,
}

pub struct AppState {
    pub(crate) dify_api_url: String,
    pub(crate) replicate_api_key: String,
    pub(crate) ipfs_url: String,
    pub(crate) public_url: String,
    pub(crate) tasks: Arc<RwLock<HashMap<String, TaskData>>>,
    pub(crate) db: DbClient,
}

impl AppState {
    pub fn new(
        dify_api_url: String,
        replicate_api_key: String,
        ipfs_url: String,
        public_url: String,
        supabase_url: String,
        supabase_key: String,
    ) -> Self {
        Self {
            dify_api_url,
            replicate_api_key,
            ipfs_url,
            public_url,
            tasks: Arc::new(RwLock::new(HashMap::new())),
            db: DbClient::new(supabase_url, supabase_key),
        }
    }

    pub async fn wait_for_task_result(&self, task_id: &str, timeout: Duration) -> Option<ImageTaskResult> {
        if let Some(task_data) = self.tasks.read().await.get(task_id) {
            let notify = task_data.notify.clone();
            let result = task_data.result.clone();

            // Check if result is already available
            if let Some(result) = result.read().await.clone() {
                return Some(result);
            }

            // Wait for notification with timeout
            let timeout_fut = tokio::time::sleep(timeout);
            tokio::pin!(timeout_fut);

            tokio::select! {
                _ = notify.notified() => {
                    result.read().await.clone()
                }
                _ = &mut timeout_fut => None
            }
        } else {
            None
        }
    }

    pub async fn set_task_result(&self, task_id: &str, result: ImageTaskResult) {
        if let Some(task_data) = self.tasks.read().await.get(task_id) {
            *task_data.result.write().await = Some(result.clone());
            task_data.notify.notify_waiters();
        }
    }

    pub async fn create_task(&self, task_id: String, callback_url: Option<String>) {
        let task_data = TaskData {
            callback_url,
            notify: Arc::new(Notify::new()),
            result: Arc::new(RwLock::new(None)),
        };
        self.tasks.write().await.insert(task_id, task_data);
    }

    pub async fn get_task(&self, task_id: &str) -> Option<TaskData> {
        self.tasks.read().await.get(task_id).cloned()
    }

    pub async fn remove_task(&self, task_id: &str) {
        self.tasks.write().await.remove(task_id);
    }
}
