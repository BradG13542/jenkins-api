//! Jenkins Client

use std::fmt::Debug;

use log::debug;
use reqwest::{header::HeaderValue, header::CONTENT_TYPE, Body, Client, RequestBuilder, Response};
use serde::Serialize;

pub mod errors;
pub use errors::{ClientError, RequestError};

mod builder;
pub mod path;
pub use self::builder::JenkinsBuilder;
pub use self::path::{Name, Path};
mod csrf;
mod tree;
pub use self::tree::{TreeBuilder, TreeQueryParam};

/// Helper type for error management
pub mod error {
    pub use super::errors::Action;
    pub use super::errors::ExpectedType;
    pub use super::errors::{ClientError, CrumbError, RequestError, SetupError};
}

#[derive(Debug, PartialEq)]
struct User {
    username: String,
    password: Option<String>,
}

/// Client struct with the methods to query Jenkins
#[derive(Debug)]
pub struct Jenkins {
    url: String,
    client: Client,
    user: Option<User>,
    csrf_enabled: bool,
    pub(crate) depth: u8,
}

/// Advanced query parameters supported by Jenkins to control the amount of data retrieved
///
/// see [taming-jenkins-json-api-depth-and-tree](https://www.cloudbees.com/blog/taming-jenkins-json-api-depth-and-tree)
#[derive(Debug)]
pub enum AdvancedQuery {
    /// depth query parameter
    Depth(u8),
    /// tree query parameter
    Tree(TreeQueryParam),
}

/// Hidden type used to represent the AdvancedQueryParams as serializer doesn't support enums
#[derive(Debug, Serialize)]
pub(crate) struct InternalAdvancedQueryParams {
    depth: Option<u8>,
    tree: Option<TreeQueryParam>,
}
impl From<AdvancedQuery> for InternalAdvancedQueryParams {
    fn from(query: AdvancedQuery) -> Self {
        match query {
            AdvancedQuery::Depth(depth) => InternalAdvancedQueryParams {
                depth: Some(depth),
                tree: None,
            },
            AdvancedQuery::Tree(tree) => InternalAdvancedQueryParams {
                depth: None,
                tree: Some(tree),
            },
        }
    }
}

impl Jenkins {
    pub(crate) fn url_api_json(&self, endpoint: &str) -> String {
        format!("{}{}/api/json", self.url, endpoint)
    }

    pub(crate) fn url(&self, endpoint: &str) -> String {
        format!("{}{}", self.url, endpoint)
    }

    async fn send(&self, mut request_builder: RequestBuilder) -> Result<Response, reqwest::Error> {
        if let Some(ref user) = self.user {
            request_builder =
                request_builder.basic_auth(user.username.clone(), user.password.clone());
        }
        let query = request_builder.build()?;
        debug!("sending {} {}", query.method(), query.url());

        let response = self.client.execute(query).await?.error_for_status()?;
        Ok(response)
    }

    pub(crate) async fn get(&self, path: &Path<'_>) -> Result<Response, reqwest::Error> {
        self.get_with_params(path, [("depth", &self.depth.to_string())])
            .await
    }

    pub(crate) async fn get_with_params<T: Serialize>(
        &self,
        path: &Path<'_>,
        qps: T,
    ) -> Result<Response, reqwest::Error> {
        let query = self
            .client
            .get(self.url_api_json(&path.to_string()))
            .query(&qps);
        let resp = self.send(query).await?;
        Ok(resp)
    }

    pub(crate) async fn post(&self, path: &Path<'_>) -> Result<Response, RequestError> {
        let mut request_builder = self.client.post(self.url(&path.to_string()));

        request_builder = self
            .add_csrf_to_request(request_builder)
            .await
            .map_err(RequestError::Crumb)?;

        let resp = self.send(request_builder).await?;

        Ok(resp)
    }

    pub(crate) async fn post_with_body<T: Into<Body> + Debug>(
        &self,
        path: &Path<'_>,
        body: T,
        qps: &[(&str, &str)],
    ) -> Result<Response, RequestError> {
        let mut request_builder = self.client.post(self.url(&path.to_string()));

        request_builder = self.add_csrf_to_request(request_builder).await?;

        request_builder = request_builder.header(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        debug!("{:?}", body);
        request_builder = request_builder.query(qps).body(body);
        let response = self.send(request_builder).await?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn can_post_with_body() {
        let mut server = mockito::Server::new_async().await;
        let jenkins_client = crate::JenkinsBuilder::new(&server.url())
            .disable_csrf()
            .build()
            .unwrap();

        let _mock = server.mock("POST", "/mypath").with_body("ok").create();

        let response = jenkins_client
            .post_with_body(&super::Path::Raw { path: "/mypath" }, "body", &[])
            .await;

        assert!(response.is_ok());
        assert_eq!(response.unwrap().text().await.unwrap(), "ok");
    }

    #[tokio::test]
    async fn can_post_with_body_and_get_error_state() {
        let mut server = mockito::Server::new_async().await;
        let jenkins_client = crate::JenkinsBuilder::new(&server.url())
            .disable_csrf()
            .build()
            .unwrap();

        let _mock = server
            .mock("POST", "/error-IllegalStateException")
            .with_status(500)
            .with_body("hviqsuvnqsodjfsqjdgo java.lang.IllegalStateException: my error\nvzfjsd")
            .create();

        let response = jenkins_client
            .post_with_body(
                &super::Path::Raw {
                    path: "/error-IllegalStateException",
                },
                "body",
                &[],
            )
            .await;

        assert!(response.is_err());
        assert_eq!(
            format!("{:?}", response),
            format!(
                r#"Err(Http(reqwest::Error {{ kind: Status(500), url: "{}/error-IllegalStateException" }}))"#,
                server.url()
            )
        );
    }

    #[tokio::test]
    async fn can_post_with_body_and_get_error_argument() {
        let mut server = mockito::Server::new_async().await;
        let jenkins_client = crate::JenkinsBuilder::new(&server.url())
            .disable_csrf()
            .build()
            .unwrap();

        let _mock = server
            .mock("POST", "/error-IllegalArgumentException")
            .with_status(500)
            .with_body("hviqsuvnqsodjfsqjdgo java.lang.IllegalArgumentException: my error\nvzfjsd")
            .create();

        let response = jenkins_client
            .post_with_body(
                &super::Path::Raw {
                    path: "/error-IllegalArgumentException",
                },
                "body",
                &[],
            )
            .await;

        assert!(response.is_err());
        assert_eq!(
            format!("{:?}", response),
            format!(
                r#"Err(Http(reqwest::Error {{ kind: Status(500), url: "{}/error-IllegalArgumentException" }}))"#,
                server.url()
            )
        );
    }

    #[tokio::test]
    async fn can_post_with_body_and_get_error_new() {
        let mut server = mockito::Server::new_async().await;
        let jenkins_client = crate::JenkinsBuilder::new(&server.url())
            .disable_csrf()
            .build()
            .unwrap();

        let _mock = server
            .mock("POST", "/error-NewException")
            .with_status(500)
            .with_body("hviqsuvnqsodjfsqjdgo java.lang.NewException: my error\nvzfjsd")
            .create();

        let response = jenkins_client
            .post_with_body(
                &super::Path::Raw {
                    path: "/error-NewException",
                },
                "body",
                &[],
            )
            .await;

        assert!(response.is_err());
        assert_eq!(
            format!("{:?}", response),
            format!(
                r#"Err(Http(reqwest::Error {{ kind: Status(500), url: "{}/error-NewException" }}))"#,
                server.url()
            ),
        );
    }

    #[tokio::test]
    async fn can_post_with_query_params() {
        let mut server = mockito::Server::new_async().await;
        let jenkins_client = crate::JenkinsBuilder::new(&server.url())
            .disable_csrf()
            .build()
            .unwrap();

        let mock = server.mock("POST", "/mypath?a=1").with_body("ok").create();

        let response = jenkins_client
            .post_with_body(&super::Path::Raw { path: "/mypath" }, "body", &[("a", "1")])
            .await;

        assert!(response.is_ok());
        assert_eq!(response.unwrap().text().await.unwrap(), "ok");
        mock.assert()
    }
}
