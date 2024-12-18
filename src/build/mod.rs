//! Jenkins Builds

use crate::client::Result;
use crate::client_internals::path::{Name, Path};
use crate::job::JobName;
use crate::Jenkins;

#[macro_use]
mod common;
pub use self::common::{Artifact, Build, BuildNumber, BuildStatus, CommonBuild, ShortBuild};
mod flow;
pub use self::flow::BuildFlowRun;
mod freestyle;
pub use self::freestyle::FreeStyleBuild;
mod pipeline;
pub use self::pipeline::WorkflowRun;
mod matrix;
pub use self::matrix::{MatrixBuild, MatrixRun};
mod maven;
pub use self::maven::{MavenBuild, MavenModuleSetBuild};
mod multijob;
pub use self::multijob::MultiJobBuild;

impl Jenkins {
    /// Get a build from a `job_name` and `build_number`
    pub async fn get_build<'a, J, B>(&self, job_name: J, build_number: B) -> Result<CommonBuild>
    where
        J: Into<JobName<'a>>,
        B: Into<BuildNumber>,
    {
        let response = self
            .get(&Path::Build {
                job_name: Name::Name(job_name.into().0),
                number: build_number.into(),
                configuration: None,
            })
            .await?
            .json()
            .await?;
        Ok(response)
    }
}
