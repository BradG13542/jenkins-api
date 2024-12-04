//! Jenkins Views, use to group Jobs

use serde::{self, Deserialize, Serialize};

use crate::{
    client_internals::{ClientError, RequestError},
    helpers::Class,
};

use crate::client;
use crate::client_internals::{Name, Path};
use crate::job::{JobName, ShortJob};
use crate::property::CommonProperty;
use crate::Jenkins;

/// Short View that is used in lists and links from other structs
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ShortView {
    /// Name of the view
    pub name: String,
    /// URL for the view
    pub url: String,

    #[cfg(not(feature = "extra-fields-visibility"))]
    #[serde(flatten)]
    pub(crate) extra_fields: Option<serde_json::Value>,
    #[cfg(feature = "extra-fields-visibility")]
    /// Extra fields not parsed for a common object
    #[serde(flatten)]
    pub extra_fields: Option<serde_json::Value>,
}

impl ShortView {
    /// Get the full details of a `View` matching the `ShortView`
    pub async fn get_full_view(&self, jenkins_client: &Jenkins) -> Result<CommonView, ClientError> {
        let path = jenkins_client.url_to_path(&self.url);
        if let Path::View { .. } = path {
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
                expected: client::error::ExpectedType::View,
            })
        }
    }
}

/// Helper type to act on a view
#[derive(Debug)]
pub struct ViewName<'a>(pub &'a str);
impl<'a> From<&'a str> for ViewName<'a> {
    fn from(v: &'a str) -> ViewName<'a> {
        ViewName(v)
    }
}
impl<'a> From<&'a String> for ViewName<'a> {
    fn from(v: &'a String) -> ViewName<'a> {
        ViewName(v)
    }
}
impl<'a> From<&'a ShortView> for ViewName<'a> {
    fn from(v: &'a ShortView) -> ViewName<'a> {
        ViewName(&v.name)
    }
}
impl<'a, T: View> From<&'a T> for ViewName<'a> {
    fn from(v: &'a T) -> ViewName<'a> {
        ViewName(v.name())
    }
}

/// Trait implemented by specialization of view
pub trait View {
    /// Get the name of the view
    fn name(&self) -> &str;
}

/// A Jenkins `View` with a list of `ShortJob`
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommonView {
    /// _class provided by Jenkins
    #[serde(rename = "_class")]
    pub class: Option<String>,
    /// Description of the view
    pub description: Option<String>,
    /// Name of the view
    pub name: String,
    /// URL for the view
    pub url: String,
    /// List of jobs in the view
    pub jobs: Vec<ShortJob>,
    /// Properties of the view
    pub property: Vec<CommonProperty>,

    #[cfg(not(feature = "extra-fields-visibility"))]
    #[serde(flatten)]
    extra_fields: serde_json::Value,
    #[cfg(feature = "extra-fields-visibility")]
    /// Extra fields not parsed for a common object
    #[serde(flatten)]
    pub extra_fields: serde_json::Value,
}
specialize!(CommonView => View);
impl View for CommonView {
    fn name(&self) -> &str {
        &self.name
    }
}

/// A Jenkins `View` with a list of `ShortJob`
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListView {
    /// Description of the view
    pub description: Option<String>,
    /// Name of the view
    pub name: String,
    /// URL for the view
    pub url: String,
    /// List of jobs in the view
    pub jobs: Vec<ShortJob>,
    /// Properties of the view
    pub property: Vec<CommonProperty>,
}
register_class!("hudson.model.ListView" => ListView);
impl View for ListView {
    fn name(&self) -> &str {
        &self.name
    }
}

impl ListView {
    /// Add the job `job_name` to this view
    pub async fn add_job<'a, J>(
        &self,
        jenkins_client: &Jenkins,
        job_name: J,
    ) -> Result<(), ClientError>
    where
        J: Into<JobName<'a>>,
    {
        let path = jenkins_client.url_to_path(&self.url);
        if let Path::View { name } = path {
            let _ = jenkins_client
                .post(&Path::AddJobToView {
                    job_name: Name::Name(job_name.into().0),
                    view_name: name,
                })
                .await
                .map_err(ClientError::Request)?;
            Ok(())
        } else {
            Err(ClientError::InvalidUrl {
                url: self.url.clone(),
                expected: client::error::ExpectedType::View,
            })
        }
    }

    /// Remove the job `job_name` from this view
    pub async fn remove_job<'a, J>(
        &self,
        jenkins_client: &Jenkins,
        job_name: J,
    ) -> Result<(), ClientError>
    where
        J: Into<JobName<'a>>,
    {
        let path = jenkins_client.url_to_path(&self.url);
        if let Path::View { name } = path {
            let _ = jenkins_client
                .post(&Path::RemoveJobFromView {
                    job_name: Name::Name(job_name.into().0),
                    view_name: name,
                })
                .await
                .map_err(ClientError::Request)?;
            Ok(())
        } else {
            Err(ClientError::InvalidUrl {
                url: self.url.clone(),
                expected: client::error::ExpectedType::View,
            })
        }
    }
}

impl Jenkins {
    /// Get a `View`
    pub async fn get_view<'a, V>(&self, view_name: V) -> Result<CommonView, ClientError>
    where
        V: Into<ViewName<'a>>,
    {
        self.get(&Path::View {
            name: Name::Name(view_name.into().0),
        })
        .await
        .map_err(|e| ClientError::Request(RequestError::Http(e)))?
        .json()
        .await
        .map_err(|e| ClientError::Request(RequestError::Http(e)))
    }

    /// Add the job `job_name` to the view `view_name`
    pub async fn add_job_to_view<'a, 'b, V, J>(
        &self,
        view_name: V,
        job_name: J,
    ) -> Result<(), ClientError>
    where
        V: Into<ViewName<'a>>,
        J: Into<JobName<'a>>,
    {
        let _ = self
            .post(&Path::AddJobToView {
                job_name: Name::Name(job_name.into().0),
                view_name: Name::Name(view_name.into().0),
            })
            .await
            .map_err(ClientError::Request)?;
        Ok(())
    }

    /// Remove the job `job_name` from the view `view_name`
    pub async fn remove_job_from_view<'a, 'b, V, J>(
        &self,
        view_name: V,
        job_name: J,
    ) -> Result<(), ClientError>
    where
        V: Into<ViewName<'a>>,
        J: Into<JobName<'a>>,
    {
        let _ = self
            .post(&Path::AddJobToView {
                job_name: Name::Name(job_name.into().0),
                view_name: Name::Name(view_name.into().0),
            })
            .await
            .map_err(ClientError::Request)?;
        Ok(())
    }
}
