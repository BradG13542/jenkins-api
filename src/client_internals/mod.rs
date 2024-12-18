//! Jenkins Client

use std::fmt::Debug;

use log::{debug, warn};
use regex::Regex;
use reqwest::{
    header::HeaderValue, header::CONTENT_TYPE, Body, Client, RequestBuilder, Response, StatusCode,
};
use serde::Serialize;

mod errors;
pub use self::errors::{Error, Result};
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

    async fn send(&self, mut request_builder: RequestBuilder) -> Result<Response> {
        if let Some(ref user) = self.user {
            request_builder =
                request_builder.basic_auth(user.username.clone(), user.password.clone());
        }
        let query = request_builder.build()?;
        debug!("sending {} {}", query.method(), query.url());

        let response = self.client.execute(query).await?;
        Ok(response)
    }

    fn error_for_status(response: Response) -> Result<Response> {
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            warn!("got an error: {}", status);
        }
        Ok(response.error_for_status()?)
    }

    pub(crate) async fn get(&self, path: &Path<'_>) -> Result<Response> {
        self.get_with_params(path, [("depth", &self.depth.to_string())])
            .await
    }

    pub(crate) async fn get_with_params<T: Serialize>(
        &self,
        path: &Path<'_>,
        qps: T,
    ) -> Result<Response> {
        let query = self
            .client
            .get(self.url_api_json(&path.to_string()))
            .query(&qps);
        let resp = self.send(query).await?;
        Self::error_for_status(resp)
    }

    pub(crate) async fn post(&self, path: &Path<'_>) -> Result<Response> {
        let mut request_builder = self.client.post(self.url(&path.to_string()));

        request_builder = self.add_csrf_to_request(request_builder).await?;

        let resp = self.send(request_builder).await?;
        Self::error_for_status(resp)
    }

    pub(crate) async fn post_with_body<T: Into<Body> + Debug>(
        &self,
        path: &Path<'_>,
        body: T,
        qps: &[(&str, &str)],
    ) -> Result<Response> {
        let mut request_builder = self.client.post(self.url(&path.to_string()));

        request_builder = self.add_csrf_to_request(request_builder).await?;

        request_builder = request_builder.header(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        debug!("{:?}", body);
        request_builder = request_builder.query(qps).body(body);
        let response = self.send(request_builder).await?;

        if response.status() == StatusCode::INTERNAL_SERVER_ERROR {
            // get the error before reading the body. In this case it can't be OK
            let error = match response.error_for_status_ref() {
                Ok(_) => unreachable!(),
                Err(err) => err,
            };

            let body = response.text().await?;

            let re = Regex::new(r"java.lang.([a-zA-Z]+): (.*)").unwrap();
            if let Some(captures) = re.captures(&body) {
                match captures.get(1).map(|v| v.as_str()) {
                    Some("IllegalStateException") => {
                        warn!(
                            "got an IllegalState error: {}",
                            captures.get(0).map(|v| v.as_str()).unwrap_or("unspecified")
                        );
                        Err(Error::IllegalState {
                            message: captures
                                .get(2)
                                .map(|v| v.as_str())
                                .unwrap_or("no message")
                                .to_string(),
                        })
                    }
                    Some("IllegalArgumentException") => {
                        warn!(
                            "got an IllegalArgument error: {}",
                            captures.get(0).map(|v| v.as_str()).unwrap_or("unspecified")
                        );
                        Err(Error::IllegalArgument {
                            message: captures
                                .get(2)
                                .map(|v| v.as_str())
                                .unwrap_or("no message")
                                .to_string(),
                        })
                    }
                    Some(_) => {
                        warn!(
                            "got an Unknwon error: {}",
                            captures.get(0).map(|v| v.as_str()).unwrap_or("unspecified")
                        );
                        Ok(())
                    }
                    _ => Ok(()),
                }?;
            }
            Err(error.into())
        } else {
            Ok(Self::error_for_status(response)?)
        }
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
            r#"Err(IllegalState { message: "my error" })"#
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
            r#"Err(IllegalArgument { message: "my error" })"#
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
                r#"Err(reqwest::Error {{ kind: Status(500), url: "{}/error-NewException" }})"#,
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
