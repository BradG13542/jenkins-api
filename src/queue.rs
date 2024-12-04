//! Jenkins build queue

use serde::{Deserialize, Serialize};

use crate::client;
use crate::client_internals::Path;
use crate::job::ShortJob;
use crate::Jenkins;
use crate::{action::CommonAction, client_internals::ClientError};
use crate::{build::ShortBuild, client_internals::RequestError};

/// Short Queue Item that is returned when building a job
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShortQueueItem {
    /// URL to this queued item
    pub url: String,

    #[cfg(not(feature = "extra-fields-visibility"))]
    #[serde(flatten)]
    pub(crate) extra_fields: Option<serde_json::Value>,
    #[cfg(feature = "extra-fields-visibility")]
    /// Extra fields not parsed for a common object
    #[serde(flatten)]
    pub extra_fields: Option<serde_json::Value>,
}
impl ShortQueueItem {
    /// Get the full details of a `QueueItem` matching the `ShortQueueItem`
    pub async fn get_full_queue_item(
        &self,
        jenkins_client: &Jenkins,
    ) -> Result<QueueItem, ClientError> {
        let path = jenkins_client.url_to_path(&self.url);
        if let Path::QueueItem { .. } = path {
            jenkins_client
                .get(&path)
                .await
                .map_err(|e| ClientError::Request(RequestError::Http(e)))?
                .json()
                .await
                .map_err(|e| ClientError::Request(RequestError::Http(e)))
        } else {
            Err(ClientError::InvalidUrl {
                url: self.url.clone(),
                expected: client::error::ExpectedType::QueueItem,
            })
        }
    }
}

/// A queued item in Jenkins, with information about the `Job` and why / since when it's waiting
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    /// Is this item blocked
    pub blocked: bool,
    /// Is this item buildable
    pub buildable: bool,
    /// Has this item been cancelled
    pub cancelled: Option<bool>,
    /// ID in the queue
    pub id: u32,
    /// When was it added to the queue
    pub in_queue_since: u64,
    /// Task parameters
    pub params: String,
    /// Is the job stuck? Node needed is offline, or waitied for very long in queue
    pub stuck: bool,
    /// Link to the job waiting in the queue
    pub task: ShortJob,
    /// URL to this queued item
    pub url: String,
    /// Why is this task in the queue
    pub why: Option<String>,
    /// When did the job exited the queue
    pub buildable_start_milliseconds: Option<u64>,
    /// Link to the build once it has started
    pub executable: Option<ShortBuild>,
    /// Build actions
    pub actions: Vec<CommonAction>,
}
impl QueueItem {
    /// Refresh a `QueueItem`, consuming the existing one and returning a new `QueueItem`
    pub async fn refresh_item(self, jenkins_client: &Jenkins) -> Result<Self, ClientError> {
        let path = jenkins_client.url_to_path(&self.url);
        if let Path::QueueItem { .. } = path {
            jenkins_client
                .get(&path)
                .await
                .map_err(|e| ClientError::Request(RequestError::Http(e)))?
                .json()
                .await
                .map_err(|e| ClientError::Request(RequestError::Http(e)))
        } else {
            Err(ClientError::InvalidUrl {
                url: self.url.clone(),
                expected: client::error::ExpectedType::QueueItem,
            })
        }
    }
}

/// The Jenkins `Queue`, the list of `QueueItem` that are waiting to be built
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    /// List of items currently in the queue
    pub items: Vec<QueueItem>,
}

impl Jenkins {
    /// Get the Jenkins items queue
    pub async fn get_queue(&self) -> Result<Queue, reqwest::Error> {
        self.get(&Path::Queue).await?.json().await
    }

    /// Get a queue item from it's ID
    pub async fn get_queue_item(&self, id: i32) -> Result<QueueItem, reqwest::Error> {
        self.get(&Path::QueueItem { id }).await?.json().await
    }
}
