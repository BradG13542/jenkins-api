use std::fmt;

use reqwest::header;
use thiserror::Error;

#[derive(Debug, Error)]
/// Errors that can be thrown at client setup
pub enum SetupError {
    #[error("invalid url: {0}")]
    /// Invalid Jenkins url
    InvalidUrl(#[from] url::ParseError),
    #[error("client: {0}")]
    /// Underlying client failure
    Client(#[from] reqwest::Error),
}

#[derive(Debug, Error)]
/// Errors that can be thrown when getting a crumb
pub enum CrumbError {
    #[error("invalid name: {0}")]
    /// Invalid crumb header name
    InvalidName(#[from] header::InvalidHeaderName),
    #[error("invalid value: {0}")]
    /// Invalid crumb header value
    InvalidValue(#[from] header::InvalidHeaderValue),
    #[error("crumb issuer: {0}")]
    /// Error during crumb request to Jenkins
    Http(#[from] reqwest::Error),
}

#[derive(Debug, Error)]
/// Errors that can be thrown when sending a request
pub enum RequestError {
    #[error("{0}")]
    /// Failed to set up crumb
    Crumb(#[from] CrumbError),
    #[error("http: {0}")]
    /// Failed to send request
    Http(#[from] reqwest::Error),
}

/// Errors that can be thrown
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("invalid url for {expected}: {url}")]
    ///  Error thrown when a link between objects has an unexpected format
    InvalidUrl {
        /// URL found
        url: String,
        /// Expected URL type
        expected: ExpectedType,
    },

    #[error("invalid crumbfield '{field_name}', expected 'Jenkins-Crumb'")]
    ///  Error thrown when CSRF protection use an unexpected field name
    InvalidCrumbFieldName {
        /// Field name provided by Jenkins api for crumb
        field_name: String,
    },

    #[error("illegal argument: '{message}'")]
    ///  Error thrown when building a parameterized job with an invalid parameter
    IllegalArgument {
        /// Exception message provided by Jenkins
        message: String,
    },

    #[error("can't build a job remotely with parameters")]
    ///  Error when trying to remotely build a job with parameters
    UnsupportedBuildConfiguration,

    #[error("can't do '{action}' on a {object_type} of type {variant_name}")]
    ///  Error when trying to do an action on an object not supporting it
    InvalidObjectType {
        /// Object type
        object_type: ExpectedType,
        /// Variant name
        variant_name: String,
        /// Action
        action: Action,
    },

    #[error("request: {0}")]
    /// Error when trying to complete some request
    Request(#[from] RequestError),
}

/// Possible type of URL expected in links between items
#[derive(Debug, Copy, Clone)]
pub enum ExpectedType {
    /// a `Build`
    Build,
    /// a `Job`
    Job,
    /// a `QueueItem`
    QueueItem,
    /// a `View`
    View,
    /// a `ShortView`
    ShortView,
    /// a `MavenArtifactRecord`
    MavenArtifactRecord,
}

impl fmt::Display for ExpectedType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ExpectedType::Build => write!(f, "Build"),
            ExpectedType::Job => write!(f, "Job"),
            ExpectedType::QueueItem => write!(f, "QueueItem"),
            ExpectedType::View => write!(f, "View"),
            ExpectedType::ShortView => write!(f, "ShortView"),
            ExpectedType::MavenArtifactRecord => write!(f, "MavenArtifactRecord"),
        }
    }
}

/// Possible action done on an object
#[derive(Debug, Copy, Clone)]
pub enum Action {
    /// Get a field
    GetField(&'static str),
    /// Get linked item
    GetLinkedItem(ExpectedType),
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Action::GetField(field) => write!(f, "get field '{}'", field),
            Action::GetLinkedItem(item) => write!(f, "get linked item '{}'", item),
        }
    }
}
