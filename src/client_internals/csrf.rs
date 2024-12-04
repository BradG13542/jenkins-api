use reqwest::{header::HeaderName, header::HeaderValue, RequestBuilder};
use serde::Deserialize;

use super::{errors::CrumbError, path::Path, Jenkins};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Crumb {
    crumb: String,
    crumb_request_field: String,
}

impl Jenkins {
    pub(crate) async fn add_csrf_to_request(
        &self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, CrumbError> {
        if self.csrf_enabled {
            let crumb = self.get_csrf().await?;
            Ok(request_builder.header(
                HeaderName::from_lowercase(crumb.crumb_request_field.to_lowercase().as_bytes())
                    .map_err(CrumbError::InvalidName)?,
                HeaderValue::from_str(&crumb.crumb).map_err(CrumbError::InvalidValue)?,
            ))
        } else {
            Ok(request_builder)
        }
    }

    pub(crate) async fn get_csrf(&self) -> Result<Crumb, CrumbError> {
        let crumb: Crumb = self
            .get(&Path::CrumbIssuer)
            .await?
            .json()
            .await
            .map_err(CrumbError::Http)?;
        Ok(crumb)
    }
}
