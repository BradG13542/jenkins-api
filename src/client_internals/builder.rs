use std::str::FromStr;

use reqwest::{self, Client, Url};

use super::{errors::SetupError, Jenkins, User};

/// Builder for Jenkins client
///
/// ```rust
///# extern crate jenkins_api;
///#
///# use jenkins_api::JenkinsBuilder;
///#
///# fn example_function() {
///     let jenkins = JenkinsBuilder::new("http://localhost:8080")
///         .with_user("user", Some("password"))
///         .build()
///         .unwrap();
///# }
/// ```
#[derive(Debug)]
pub struct JenkinsBuilder {
    url: String,
    user: Option<User>,
    csrf_enabled: bool,
    depth: u8,
}

impl JenkinsBuilder {
    /// Create a new builder with Jenkins url
    pub fn new(url: &str) -> Self {
        JenkinsBuilder {
            url: {
                let last: String = url.chars().rev().take(1).collect();
                match last.as_str() {
                    "/" => url[0..(url.len() - 1)].to_string(),
                    _ => url.to_string(),
                }
            },
            user: None,
            csrf_enabled: true,
            depth: 1,
        }
    }

    /// Build the Jenkins client
    pub fn build(self) -> Result<Jenkins, SetupError> {
        let url = Url::from_str(&self.url).map_err(SetupError::InvalidUrl)?;
        if url.cannot_be_a_base() {
            return Err(SetupError::InvalidUrl(
                url::ParseError::RelativeUrlWithoutBase,
            ));
        };
        if !url.has_host() {
            return Err(SetupError::InvalidUrl(url::ParseError::EmptyHost));
        }

        Ok(Jenkins {
            url: self.url,
            client: Client::builder().build().map_err(SetupError::Client)?,
            user: self.user,
            csrf_enabled: self.csrf_enabled,
            depth: self.depth,
        })
    }

    /// Specify the user to use for authorizing queries
    pub fn with_user(mut self, login: &str, password: Option<&str>) -> Self {
        self.user = Some(User {
            username: login.to_string(),
            password: password.map(Into::into),
        });
        self
    }

    /// Disable CSRF in crumbs used for post queries
    pub fn disable_csrf(mut self) -> Self {
        self.csrf_enabled = false;
        self
    }

    /// Change the default depth parameters of requests made to Jenkins. It
    /// controls the amount of data in responses
    pub fn with_depth(mut self, depth: u8) -> Self {
        self.depth = depth;
        self
    }
}

#[cfg(test)]
mod tests {
    static JENKINS_URL: &str = "http://none:8080";

    #[test]
    fn create_builder() {
        let jenkins_client = crate::JenkinsBuilder::new(JENKINS_URL);

        assert_eq!(jenkins_client.url, JENKINS_URL);
        assert_eq!(jenkins_client.user, None);
        assert!(jenkins_client.csrf_enabled);
    }

    #[test]
    fn create_builder_with_trailing_slash() {
        let jenkins_client = crate::JenkinsBuilder::new(&format!("{}/", JENKINS_URL));

        assert_eq!(jenkins_client.url, JENKINS_URL);
        assert_eq!(jenkins_client.user, None);
        assert!(jenkins_client.csrf_enabled);
    }

    #[test]
    fn disable_csrf() {
        let jenkins_client = crate::JenkinsBuilder::new(JENKINS_URL).disable_csrf();

        assert_eq!(jenkins_client.url, JENKINS_URL);
        assert_eq!(jenkins_client.user, None);
        assert!(!jenkins_client.csrf_enabled);
    }
}
