use std::fmt::Display;

use super::Jenkins;
use crate::build;

/// Name of an object
#[derive(Debug, PartialEq, Clone)]
pub enum Name<'a> {
    /// Name of an object
    Name(&'a str),
    /// URL Encoded name of an object
    UrlEncodedName(&'a str),
}

impl<'a> Display for Name<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Name::Name(name) => write!(f, "{}", urlencoding::encode(name)),
            Name::UrlEncodedName(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Path<'a> {
    Home,
    View {
        name: Name<'a>,
    },
    AddJobToView {
        job_name: Name<'a>,
        view_name: Name<'a>,
    },
    RemoveJobFromView {
        job_name: Name<'a>,
        view_name: Name<'a>,
    },
    Job {
        name: Name<'a>,
        configuration: Option<Name<'a>>,
    },
    BuildJob {
        name: Name<'a>,
    },
    BuildJobWithParameters {
        name: Name<'a>,
    },
    PollSCMJob {
        name: Name<'a>,
    },
    JobEnable {
        name: Name<'a>,
    },
    JobDisable {
        name: Name<'a>,
    },
    Build {
        job_name: Name<'a>,
        number: build::BuildNumber,
        configuration: Option<Name<'a>>,
    },
    ConsoleText {
        job_name: Name<'a>,
        number: build::BuildNumber,
        configuration: Option<Name<'a>>,
        folder_name: Option<Name<'a>>,
    },
    ConfigXML {
        job_name: Name<'a>,
        folder_name: Option<Name<'a>>,
    },
    Queue,
    QueueItem {
        id: i32,
    },
    MavenArtifactRecord {
        job_name: Name<'a>,
        number: build::BuildNumber,
        configuration: Option<Name<'a>>,
    },
    InFolder {
        folder_name: Name<'a>,
        path: Box<Path<'a>>,
    },
    Computers,
    Computer {
        name: Name<'a>,
    },
    Raw {
        path: &'a str,
    },
    CrumbIssuer,
}
impl<'a> Display for Path<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Path::Home => Ok(()),
            Path::View { ref name } => write!(f, "/view/{}", name),
            Path::AddJobToView {
                ref job_name,
                ref view_name,
            } => write!(f, "/view/{}/addJobToView?name={}", view_name, job_name),
            Path::RemoveJobFromView {
                ref job_name,
                ref view_name,
            } => write!(f, "/view/{}/removeJobFromView?name={}", view_name, job_name),
            Path::Job {
                ref name,
                configuration: Some(ref configuration),
            } => write!(f, "/job/{}/{}", name, configuration),
            Path::Job {
                ref name,
                configuration: None,
            } => write!(f, "/job/{}", name),
            Path::BuildJob { ref name } => write!(f, "/job/{}/build", name),
            Path::BuildJobWithParameters { ref name } => {
                write!(f, "/job/{}/buildWithParameters", name)
            }
            Path::PollSCMJob { ref name } => write!(f, "/job/{}/polling", name),
            Path::JobEnable { ref name } => write!(f, "/job/{}/enable", name),
            Path::JobDisable { ref name } => write!(f, "/job/{}/disable", name),
            Path::Build {
                ref job_name,
                ref number,
                configuration: None,
            } => write!(f, "/job/{}/{}", job_name, number),
            Path::Build {
                ref job_name,
                ref number,
                configuration: Some(ref configuration),
            } => write!(f, "/job/{}/{}/{}", job_name, configuration, number),
            Path::ConsoleText {
                ref job_name,
                ref number,
                configuration: None,
                folder_name: None,
            } => write!(f, "/job/{}/{}/consoleText", job_name, number),
            Path::ConsoleText {
                ref job_name,
                ref number,
                configuration: Some(ref configuration),
                folder_name: None,
            } => write!(
                f,
                "/job/{}/{}/{}/consoleText",
                job_name, configuration, number
            ),
            Path::ConsoleText {
                ref job_name,
                ref number,
                configuration: None,
                folder_name: Some(ref folder_name),
            } => write!(
                f,
                "/job/{}/job/{}/{}/consoleText",
                folder_name, job_name, number
            ),
            Path::ConsoleText {
                ref job_name,
                ref number,
                configuration: Some(ref configuration),
                folder_name: Some(ref folder_name),
            } => write!(
                f,
                "/job/{}/job/{}/{}/{}/consoleText",
                folder_name, job_name, configuration, number
            ),
            Path::ConfigXML {
                ref job_name,
                folder_name: None,
            } => write!(f, "/job/{}/config.xml", job_name),
            Path::ConfigXML {
                ref job_name,
                folder_name: Some(ref folder_name),
            } => write!(f, "/job/{}/job/{}/config.xml", folder_name, job_name,),
            Path::Queue => write!(f, "/queue"),
            Path::QueueItem { ref id } => write!(f, "/queue/item/{}", id),
            Path::MavenArtifactRecord {
                ref job_name,
                ref number,
                configuration: None,
            } => write!(f, "/job/{}/{}/mavenArtifacts", job_name, number),
            Path::MavenArtifactRecord {
                ref job_name,
                ref number,
                configuration: Some(ref configuration),
            } => write!(
                f,
                "/job/{}/{}/{}/mavenArtifacts",
                job_name, configuration, number
            ),
            Path::InFolder {
                ref folder_name,
                ref path,
            } => write!(f, "/job/{}{}", folder_name, path),
            Path::Computers => write!(f, "/computer/api/json"),
            Path::Computer { ref name } => write!(f, "/computer/{}/api/json", name),
            Path::Raw { path } => write!(f, "{}", path),
            Path::CrumbIssuer => write!(f, "/crumbIssuer"),
        }
    }
}

impl Jenkins {
    pub(crate) fn url_to_path<'a>(&self, url: &'a str) -> Path<'a> {
        let path = if url.starts_with(&self.url) {
            &url[self.url.len()..]
        } else {
            url
        };
        let slashes: Vec<usize> = path
            .char_indices()
            .filter(|c| c.1 == '/')
            .map(|c| c.0)
            .collect();

        match (&path[0..slashes[1]], slashes.len()) {
            ("/view", 3) => Path::View {
                name: Name::UrlEncodedName(&path[6..(path.len() - 1)]),
            },
            ("/job", 3) => Path::Job {
                name: Name::UrlEncodedName(&path[5..(path.len() - 1)]),
                configuration: None,
            },
            ("/job", 4) => {
                let last_part = &path[(slashes[2] + 1)..(path.len() - 1)];
                let number = last_part.parse();
                if let Ok(number) = number {
                    Path::Build {
                        job_name: Name::UrlEncodedName(&path[5..slashes[2]]),
                        number: build::BuildNumber::Number(number),
                        configuration: None,
                    }
                } else {
                    Path::Job {
                        name: Name::UrlEncodedName(&path[5..slashes[2]]),
                        configuration: Some(Name::UrlEncodedName(last_part)),
                    }
                }
            }
            ("/job", 5) => {
                if &path[slashes[3]..slashes[4]] == "/mavenArtifacts" {
                    Path::MavenArtifactRecord {
                        job_name: Name::UrlEncodedName(&path[5..slashes[2]]),
                        number: build::BuildNumber::Number(
                            path[(slashes[3] + 1)..(path.len() - 1)].parse().unwrap(),
                        ),
                        configuration: None,
                    }
                } else if &path[slashes[2]..slashes[3]] == "/job" {
                    Path::InFolder {
                        folder_name: Name::UrlEncodedName(&path[5..slashes[2]]),
                        path: Box::new(self.url_to_path(&path[slashes[2]..])),
                    }
                } else {
                    Path::Build {
                        job_name: Name::UrlEncodedName(&path[5..slashes[2]]),
                        number: build::BuildNumber::Number(
                            path[(slashes[3] + 1)..(path.len() - 1)].parse().unwrap(),
                        ),
                        configuration: Some(Name::UrlEncodedName(
                            &path[(slashes[2] + 1)..slashes[3]],
                        )),
                    }
                }
            }
            ("/job", 6) => {
                if &path[slashes[2]..slashes[3]] == "/job" {
                    Path::InFolder {
                        folder_name: Name::UrlEncodedName(&path[5..slashes[2]]),
                        path: Box::new(self.url_to_path(&path[slashes[2]..])),
                    }
                } else {
                    Path::MavenArtifactRecord {
                        job_name: Name::UrlEncodedName(&path[5..slashes[2]]),
                        number: build::BuildNumber::Number(
                            path[(slashes[3] + 1)..slashes[4]].parse().unwrap(),
                        ),
                        configuration: Some(Name::UrlEncodedName(
                            &path[(slashes[2] + 1)..slashes[3]],
                        )),
                    }
                }
            }
            ("/queue", 4) => Path::QueueItem {
                id: path[(slashes[2] + 1)..(path.len() - 1)].parse().unwrap(),
            },
            (_, _) => Path::Raw { path },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static JENKINS_URL: &str = "http://none:8080";

    #[test]
    fn can_parse_view_path() {
        let jenkins_client = crate::JenkinsBuilder::new(JENKINS_URL).build().unwrap();

        let path = jenkins_client.url_to_path("/view/myview/");
        assert_eq!(
            path,
            Path::View {
                name: Name::UrlEncodedName("myview")
            }
        );
    }

    #[test]
    fn can_parse_job_path() {
        let jenkins_client = crate::JenkinsBuilder::new(JENKINS_URL).build().unwrap();

        let path = jenkins_client.url_to_path("/job/myjob/");
        assert_eq!(
            path,
            Path::Job {
                name: Name::UrlEncodedName("myjob"),
                configuration: None
            }
        );
    }

    #[test]
    fn can_parse_job_with_config_path() {
        let jenkins_client = crate::JenkinsBuilder::new(JENKINS_URL).build().unwrap();

        let path = jenkins_client.url_to_path("/job/myjob/config/");
        assert_eq!(
            path,
            Path::Job {
                name: Name::UrlEncodedName("myjob"),
                configuration: Some(Name::UrlEncodedName("config"))
            }
        );
    }

    #[test]
    fn can_parse_build_path() {
        let jenkins_client = crate::JenkinsBuilder::new(JENKINS_URL).build().unwrap();

        let path = jenkins_client.url_to_path("/job/myjob/1/");
        assert_eq!(
            path,
            Path::Build {
                job_name: Name::UrlEncodedName("myjob"),
                number: build::BuildNumber::Number(1),
                configuration: None
            }
        );
    }

    #[test]
    fn can_parse_build_with_config_path() {
        let jenkins_client = crate::JenkinsBuilder::new(JENKINS_URL).build().unwrap();

        let path = jenkins_client.url_to_path("/job/myjob/config/1/");
        assert_eq!(
            path,
            Path::Build {
                job_name: Name::UrlEncodedName("myjob"),
                number: build::BuildNumber::Number(1),
                configuration: Some(Name::UrlEncodedName("config"))
            }
        );
    }

    #[test]
    fn can_parse_unknown_path() {
        let jenkins_client = crate::JenkinsBuilder::new(JENKINS_URL).build().unwrap();

        let path = jenkins_client.url_to_path("/unknown/path/");
        assert_eq!(
            path,
            Path::Raw {
                path: "/unknown/path/"
            }
        );
    }

    #[test]
    fn can_parse_job_path_with_jenkins_url() {
        let jenkins_client = crate::JenkinsBuilder::new(JENKINS_URL).build().unwrap();

        let path_url = format!("{}/job/myjob/", JENKINS_URL);
        let path = jenkins_client.url_to_path(&path_url);
        assert_eq!(
            path,
            Path::Job {
                name: Name::UrlEncodedName("myjob"),
                configuration: None
            }
        );
    }
}
