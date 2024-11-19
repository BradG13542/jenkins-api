use std::fmt::Display;

use serde::{Serialize, Serializer};

/// Jenkins tree query parameter
#[derive(Debug)]
pub struct TreeQueryParam {
    /// Name of the key at the root of this tree
    keyname: Option<String>,
    /// Children keys
    subkeys: Vec<TreeQueryParam>,
}
impl Serialize for TreeQueryParam {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
impl Display for TreeQueryParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.keyname.as_ref(), self.subkeys.len()) {
            (Some(keyname), 0) => write!(f, "{}", keyname),
            (Some(keyname), _) => write!(
                f,
                "{}[{}]",
                keyname,
                self.subkeys
                    .iter()
                    .map(TreeQueryParam::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            (None, _) => write!(
                f,
                "{}",
                self.subkeys
                    .iter()
                    .map(TreeQueryParam::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

/// Helper to build a `TreeQueryParam`
///
/// ```
/// jenkins_api::client::TreeBuilder::object("builds")
///     .with_subfield("url")
///     .with_subfield("result")
///     .with_subfield(
///         jenkins_api::client::TreeBuilder::object("actions").with_subfield("causes"),
///     )
///     .build();
/// ```
#[derive(Debug)]
pub struct TreeBuilder {
    tree: TreeQueryParam,
}
impl TreeBuilder {
    /// Build a new empty `TreeBuilder`
    pub fn new() -> Self {
        TreeBuilder {
            tree: TreeQueryParam {
                keyname: None,
                subkeys: vec![],
            },
        }
    }
    /// Add a field to the `TreeQueryParam`
    pub fn with_field<T: Into<TreeQueryParam>>(mut self, subfield: T) -> Self {
        self.tree.subkeys.push(subfield.into());
        self
    }
    /// Create a parent `TreeQueryParam`
    pub fn object(name: &str) -> Self {
        TreeBuilder {
            tree: TreeQueryParam {
                keyname: Some(name.to_string()),
                subkeys: vec![],
            },
        }
    }
    /// Add a subfield to the `TreeQueryParam`
    pub fn with_subfield<T: Into<TreeQueryParam>>(self, subfield: T) -> Self {
        self.with_field(subfield)
    }
    /// Build the `TreeQueryParam`
    pub fn build(self) -> TreeQueryParam {
        self.tree
    }
}
impl From<TreeBuilder> for TreeQueryParam {
    fn from(value: TreeBuilder) -> Self {
        value.build()
    }
}
impl<'a> From<&'a str> for TreeQueryParam {
    fn from(value: &'a str) -> Self {
        TreeQueryParam {
            keyname: Some(value.to_string()),
            subkeys: vec![],
        }
    }
}
impl From<TreeQueryParam> for Option<super::AdvancedQuery> {
    fn from(value: TreeQueryParam) -> Self {
        Some(super::AdvancedQuery::Tree(value))
    }
}
impl Default for TreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
