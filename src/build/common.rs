use std::{fmt::Display, marker::PhantomData};

use serde::{self, Deserialize, Serialize};

use crate::{
    client_internals::{ClientError, RequestError},
    helpers::Class,
};

use crate::action::CommonAction;
use crate::client;
use crate::client_internals::path::Path;
use crate::job::{CommonJob, Job};
use crate::Jenkins;

/// Short Build that is used in lists and links from other structs
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ShortBuild<T: Build = CommonBuild> {
    /// URL for the build
    pub url: String,
    /// Build number
    pub number: u32,
    /// Display name for the build
    pub display_name: Option<String>,
    /// Timestamp for the build
    pub timestamp: Option<u64>,

    #[cfg(not(feature = "extra-fields-visibility"))]
    #[serde(flatten)]
    pub(crate) extra_fields: Option<serde_json::Value>,
    #[cfg(feature = "extra-fields-visibility")]
    /// Extra fields not parsed for a common object
    #[serde(flatten)]
    pub extra_fields: Option<serde_json::Value>,

    #[serde(skip)]
    build_type: PhantomData<T>,
}
impl<T> ShortBuild<T>
where
    T: Build,
    for<'de> T: Deserialize<'de>,
{
    /// Get the full details of a `Build` matching the `ShortBuild`
    pub async fn get_full_build(&self, jenkins_client: &Jenkins) -> Result<T, ClientError> {
        let path = jenkins_client.url_to_path(&self.url);
        if let Path::Build { .. } = path {
            let response = jenkins_client
                .get(&path)
                .await
                .map_err(|e| ClientError::Request(RequestError::Http(e)))?
                .json()
                .await
                .map_err(|e| ClientError::Request(RequestError::Http(e)))?;
            return Ok(response);
        } else if let Path::InFolder { path: sub_path, .. } = &path {
            if let Path::Build { .. } = sub_path.as_ref() {
                let response = jenkins_client
                    .get(&path)
                    .await
                    .map_err(|e| ClientError::Request(RequestError::Http(e)))?
                    .json()
                    .await
                    .map_err(|e| ClientError::Request(RequestError::Http(e)))?;
                return Ok(response);
            }
        }
        Err(ClientError::InvalidUrl {
            url: self.url.clone(),
            expected: client::error::ExpectedType::Build,
        })
    }
}

/// Status of a build
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BuildStatus {
    /// Successful build
    Success,
    /// Unstable build
    Unstable,
    /// Failed build
    Failure,
    /// Not yet built
    NotBuilt,
    /// Aborted build
    Aborted,
}

/// A file archived by a `Build`
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    /// Displayed path
    pub display_path: Option<String>,
    /// File name
    pub file_name: String,
    /// Path to the file
    pub relative_path: String,
}

/// Helper type to act on a build
#[derive(Debug, PartialEq, Clone)]
pub enum BuildNumber {
    /// Alias to last build
    LastBuild,
    /// Alias to last successful build
    LastSuccessfulBuild,
    /// Alias to last stable build
    LastStableBuild,
    /// Alias to last complete build
    LastCompletedBuild,
    /// Alias to last failed build
    LastFailedBuild,
    /// Alias to last unsuccessful build
    LastUnsuccessfulBuild,
    /// Build number
    Number(u32),
    /// Unknown alias
    UnknownAlias(String),
}
impl Display for BuildNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildNumber::LastBuild => write!(f, "lastBuild"),
            BuildNumber::LastSuccessfulBuild => write!(f, "lastSuccessfulBuild"),
            BuildNumber::LastStableBuild => write!(f, "lastStableBuild"),
            BuildNumber::LastCompletedBuild => write!(f, "lastCompletedBuild"),
            BuildNumber::LastFailedBuild => write!(f, "lastFailedBuild"),
            BuildNumber::LastUnsuccessfulBuild => write!(f, "lastUnsuccessfulBuild"),
            BuildNumber::Number(n) => write!(f, "{}", n),
            BuildNumber::UnknownAlias(s) => write!(f, "{}", s),
        }
    }
}
impl<'a> From<&'a str> for BuildNumber {
    fn from(v: &'a str) -> BuildNumber {
        match v {
            "lastBuild" => BuildNumber::LastBuild,
            "lastSuccessfulBuild" => BuildNumber::LastSuccessfulBuild,
            "lastStableBuild" => BuildNumber::LastStableBuild,
            "lastCompletedBuild" => BuildNumber::LastCompletedBuild,
            "lastFailedBuild" => BuildNumber::LastFailedBuild,
            "lastUnsuccessfulBuild" => BuildNumber::LastUnsuccessfulBuild,
            _ => BuildNumber::UnknownAlias(v.to_string()),
        }
    }
}
impl From<u32> for BuildNumber {
    fn from(v: u32) -> BuildNumber {
        BuildNumber::Number(v)
    }
}
macro_rules! safe_into_buildnumber {
    ($type_from:ty) => {
        impl From<$type_from> for BuildNumber {
            fn from(v: $type_from) -> BuildNumber {
                BuildNumber::Number(u32::from(v))
            }
        }
    };
}
macro_rules! into_buildnumber {
    ($type_from:ty) => {
        impl From<$type_from> for BuildNumber {
            fn from(v: $type_from) -> BuildNumber {
                BuildNumber::Number(v as u32)
            }
        }
    };
}
safe_into_buildnumber!(u8);
safe_into_buildnumber!(u16);
into_buildnumber!(u64);
into_buildnumber!(i8);
into_buildnumber!(i16);
into_buildnumber!(i32);
into_buildnumber!(i64);

/// Trait implemented by specializations of `Build` and providing common methods
pub trait Build {
    /// Type of the job that triggered this build
    type ParentJob: Job;

    /// Get the url of a build
    fn url(&self) -> &str;

    /// Get the `Job` from a `Build`
    fn get_job(
        &self,
        jenkins_client: &Jenkins,
    ) -> impl std::future::Future<Output = Result<Self::ParentJob, ClientError>>
    where
        for<'de> Self::ParentJob: Deserialize<'de>,
    {
        async move {
            let path = jenkins_client.url_to_path(self.url());
            if let Path::Build {
                job_name,
                configuration,
                ..
            } = path
            {
                let response = jenkins_client
                    .get(&Path::Job {
                        name: job_name,
                        configuration,
                    })
                    .await
                    .map_err(|e| ClientError::Request(RequestError::Http(e)))?
                    .json()
                    .await
                    .map_err(|e| ClientError::Request(RequestError::Http(e)))?;
                return Ok(response);
            } else if let Path::InFolder {
                path: sub_path,
                folder_name,
            } = &path
            {
                if let Path::Build {
                    job_name,
                    configuration,
                    ..
                } = sub_path.as_ref()
                {
                    let response = jenkins_client
                        .get(&Path::InFolder {
                            folder_name: folder_name.clone(),
                            path: Box::new(Path::Job {
                                name: job_name.clone(),
                                configuration: configuration.clone(),
                            }),
                        })
                        .await
                        .map_err(|e| ClientError::Request(RequestError::Http(e)))?
                        .json()
                        .await
                        .map_err(|e| ClientError::Request(RequestError::Http(e)))?;
                    return Ok(response);
                }
            }
            Err(ClientError::InvalidUrl {
                url: self.url().to_string(),
                expected: client::error::ExpectedType::Build,
            })
        }
    }

    /// Get the console output from a `Build`
    fn get_console(
        &self,
        jenkins_client: &Jenkins,
    ) -> impl std::future::Future<Output = Result<String, ClientError>> {
        async move {
            let path = jenkins_client.url_to_path(self.url());
            if let Path::Build {
                job_name,
                number,
                configuration,
            } = path
            {
                let response = jenkins_client
                    .get(&Path::ConsoleText {
                        job_name,
                        number,
                        configuration,
                        folder_name: None,
                    })
                    .await
                    .map_err(|e| ClientError::Request(RequestError::Http(e)))?
                    .text()
                    .await
                    .map_err(|e| ClientError::Request(RequestError::Http(e)))?;
                return Ok(response);
            } else if let Path::InFolder {
                path: sub_path,
                folder_name,
            } = &path
            {
                if let Path::Build {
                    job_name,
                    number,
                    configuration,
                } = sub_path.as_ref()
                {
                    let response = jenkins_client
                        .get(&Path::ConsoleText {
                            job_name: job_name.clone(),
                            number: number.clone(),
                            configuration: configuration.clone(),
                            folder_name: Some(folder_name.clone()),
                        })
                        .await
                        .map_err(|e| ClientError::Request(RequestError::Http(e)))?
                        .text()
                        .await
                        .map_err(|e| ClientError::Request(RequestError::Http(e)))?;
                    return Ok(response);
                }
            }

            Err(ClientError::InvalidUrl {
                url: self.url().to_string(),
                expected: client::error::ExpectedType::Build,
            })
        }
    }
}

macro_rules! build_with_common_fields_and_impl {
    (
        $(#[$attr:meta])*
        pub struct $name:ident {
            $(
                $(#[$field_attr:meta])*
                pub $field:ident: $field_type:ty,
            )*
            $(private_fields {
                $(
                    $(#[$private_field_attr:meta])*
                    $private_field:ident: $private_field_type:ty
                ),* $(,)*
            })*
        }
    ) => {
        build_with_common_fields_and_impl!{
            $(#[$attr])*
            pub struct $name<ParentJob = CommonJob> {
                $(
                    $(#[$field_attr])*
                    pub $field: $field_type,
                )*
                $(private_fields {
                    $(
                        $(#[$private_field_attr])*
                        $private_field: $private_field_type
                    ),*
                })*
            }
        }

    };

    (
        $(#[$attr:meta])*
        pub struct $name:ident<ParentJob = $parent_job:ty> {
            $(
                $(#[$field_attr:meta])*
                pub $field:ident: $field_type:ty,
            )*
            $(private_fields {
                $(
                    $(#[$private_field_attr:meta])*
                    $private_field:ident: $private_field_type:ty
                ),* $(,)*
            })*
        }
    ) => {
        $(#[$attr])*
        pub struct $name {
            /// URL for the build
            pub url: String,
            /// Build number for this job
            pub number: u32,
            /// Duration
            pub duration: i64,
            /// Estimated duration
            pub estimated_duration: i64,
            /// Timestamp of the build start
            pub timestamp: u64,
            /// Are the logs kept?
            pub keep_log: bool,
            /// Build result
            pub result: Option<BuildStatus>,
            /// Display name, usually "#" followed by the build number
            pub display_name: String,
            /// Full display name: job name followed by the build display name
            pub full_display_name: Option<String>,
            /// Build description
            pub description: Option<String>,
            /// Is this build currently running
            pub building: bool,
            /// Build number in string format
            pub id: String,
            /// ID while in the build queue
            pub queue_id: i32,
            /// Build actions
            pub actions: Vec<CommonAction>,
            /// Artifacts saved by archived by this build
            pub artifacts: Vec<Artifact>,
            $(
                $(#[$field_attr])*
                pub $field: $field_type,
            )*
            $($(
                $(#[$private_field_attr])*
                $private_field: $private_field_type,
            )*)*
        }
        impl Build for $name {
            type ParentJob = $parent_job;
            fn url(&self) -> &str {
                &self.url
            }
        }
    };
}

build_with_common_fields_and_impl!(
    /// A Jenkins `Build`
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct CommonBuild<ParentJob = CommonJob> {
        /// _class provided by Jenkins
        #[serde(rename = "_class")]
        pub class: Option<String>,

        #[cfg(feature = "extra-fields-visibility")]
        /// Extra fields not parsed for a common object
        #[serde(flatten)]
        pub extra_fields: serde_json::Value,
        private_fields {
            #[cfg(not(feature = "extra-fields-visibility"))]
            #[serde(flatten)]
            extra_fields: serde_json::Value,
        }
    }
);
specialize!(CommonBuild => Build);

impl CommonBuild {}
