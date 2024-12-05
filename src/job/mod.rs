//! Jenkins Jobs

use crate::client_internals::{Name, Path};
use crate::{client_internals::ClientError, queue::ShortQueueItem};
use crate::{client_internals::RequestError, Jenkins};

pub mod builder;
use self::builder::JobBuilder;

#[macro_use]
mod common;
pub use self::common::{
    BallColor, BuildableJob, CommonJob, HealthReport, Job, JobName, SCMPollable, ShortJob,
};
mod flow;
pub use self::flow::BuildFlowJob;
mod freestyle;
pub use self::freestyle::FreeStyleProject;
mod pipeline;
pub use self::pipeline::WorkflowJob;
mod matrix;
pub use self::matrix::{MatrixConfiguration, MatrixProject};
mod maven;
pub use self::maven::{MavenModule, MavenModuleSet};
mod multijob;
pub use self::multijob::MultiJobProject;
mod external;
pub use self::external::ExternalJob;
mod folder;
pub use self::folder::Folder;
mod multibranch_pipeline;
pub use self::multibranch_pipeline::WorkflowMultiBranchProject;

impl Jenkins {
    /// Get a `Job` from it's `job_name`
    pub async fn get_job<'a, J>(&self, job_name: J) -> Result<CommonJob, ClientError>
    where
        J: Into<JobName<'a>>,
    {
        let response = self
            .get(&Path::Job {
                name: Name::Name(job_name.into().0),
                configuration: None,
            })
            .await
            .map_err(|e| ClientError::Request(RequestError::Http(e)))?
            .json()
            .await
            .map_err(|e| ClientError::Request(RequestError::Http(e)))?;
        Ok(response)
    }

    /// Build a `Job` from it's `job_name`
    pub async fn build_job<'a, J>(&self, job_name: J) -> Result<ShortQueueItem, ClientError>
    where
        J: Into<JobName<'a>>,
    {
        JobBuilder::new_from_job_name(job_name.into().0, self)
            .send()
            .await
    }

    /// Create a `JobBuilder` to setup a build of a `Job` from it's `job_name`
    pub fn job_builder<'a, 'b, 'c, 'd>(&'b self, job_name: &'a str) -> JobBuilder<'a, 'b, 'c, 'd> {
        JobBuilder::new_from_job_name(job_name, self)
    }

    /// Poll SCM of a `Job` from it's `job_name`
    pub async fn poll_scm_job<'a, J>(&self, job_name: J) -> Result<(), ClientError>
    where
        J: Into<JobName<'a>>,
    {
        let _ = self
            .post(&Path::PollSCMJob {
                name: Name::Name(job_name.into().0),
            })
            .await?;
        Ok(())
    }
}
