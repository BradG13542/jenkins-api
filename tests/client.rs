extern crate spectral;

extern crate env_logger;
#[macro_use]
extern crate serde;
extern crate serde_json;

extern crate jenkins_api;

use spectral::prelude::*;

use jenkins_api::job::{BuildableJob, Job, SCMPollable};
use jenkins_api::JenkinsBuilder;
use jenkins_api::{
    build::Build,
    client::error::{ClientError, RequestError},
};
use std::time;

use std::sync::Once;

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        env_logger::init();
    });
}

static JENKINS_URL: &str = "http://localhost:8080";

#[tokio::test]
async fn can_get_jenkins_home() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    assert!(jenkins.get_home().await.is_ok());
}

#[tokio::test]
async fn should_be_unauthorized() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("unknown", Some("password"))
        .build()
        .unwrap();
    let response = jenkins.get_home().await.unwrap_err();
    let ClientError::Request(RequestError::Http(response)) = response else {
        panic!("expected http error, got {}", response);
    };
    assert_eq!(response.status(), Some(reqwest::StatusCode::UNAUTHORIZED));
}

#[tokio::test]
async fn should_be_connection_error() {
    setup();
    let jenkins = JenkinsBuilder::new("http://localhost:1234")
        .build()
        .unwrap();
    let response = jenkins.get_home().await;
    assert!(response.is_err());
}

#[tokio::test]
async fn can_get_view() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    assert!(jenkins.get_view("view disabled").await.is_ok());
}

#[tokio::test]
async fn should_get_view_not_found() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    let response = jenkins.get_view("zut").await.unwrap_err();
    let ClientError::Request(RequestError::Http(response)) = response else {
        panic!("expected http error, got {}", response)
    };
    assert_eq!(response.status(), Some(reqwest::StatusCode::NOT_FOUND));
}

#[tokio::test]
async fn can_get_job() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    let job = jenkins.get_job("normal job").await;
    assert!(job.is_ok());
}

#[tokio::test]
async fn can_get_build() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    assert!(jenkins.get_build("normal job", 1).await.is_ok());
}

#[tokio::test]
async fn can_get_jenkins_view_from_home() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    let home = jenkins.get_home().await;
    assert!(home.is_ok());
    let home_ok = home.unwrap();
    let first_view = home_ok
        .views
        .iter()
        .find(|view| view.name == "view disabled")
        .unwrap();
    let full_view = first_view.get_full_view(&jenkins).await;
    assert!(full_view.is_ok());
    let full_job = full_view.unwrap().jobs[0].get_full_job(&jenkins).await;
    assert!(full_job.is_ok());
}

#[tokio::test]
async fn can_get_build_from_job_and_back() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    let job = jenkins.get_job("normal job").await;
    assert_that!(job).is_ok();
    let job_variant = job
        .unwrap()
        .as_variant::<jenkins_api::job::FreeStyleProject>();
    assert_that!(job_variant).is_ok();
    let job_ok = job_variant.unwrap();
    let last_build = &job_ok.last_build;
    let build = last_build.as_ref().unwrap().get_full_build(&jenkins).await;
    assert!(build.is_ok());
    let job_back = build.unwrap().get_job(&jenkins).await;
    assert_that!(job_back).is_ok();
    assert_that!(job_back.unwrap().name).is_equal_to(job_ok.name);
}

#[tokio::test]
async fn can_disable_job_and_reenable() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    let job = jenkins.get_job("normal job").await;
    assert!(job.is_ok());
    let job_ok = job.unwrap();
    assert!(job_ok.buildable);

    let disabling = job_ok.disable(&jenkins).await;
    assert!(disabling.is_ok());
    let job_disabled = jenkins.get_job("normal job").await;
    assert!(job_disabled.is_ok());
    let job_disabled_ok = job_disabled.unwrap();
    assert!(!job_disabled_ok.buildable);

    let enabling = job_disabled_ok.enable(&jenkins).await;
    assert!(enabling.is_ok());
    let job_enabled = jenkins.get_job("normal job").await;
    assert!(job_enabled.is_ok());
    let job_enabled_ok = job_enabled.unwrap();
    assert!(job_enabled_ok.buildable);
}

#[tokio::test]
async fn can_add_and_remove_job_from_view_through_view() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    let view = jenkins.get_view("test view").await;
    assert!(view.is_ok());
    let view_ok = view.unwrap();
    assert!(!view_ok
        .jobs
        .iter()
        .map(|job| &job.name)
        .any(|job_name| job_name == "normal job"));

    let job = jenkins.get_job("normal job").await;
    assert!(job.is_ok());
    let job_ok = job.unwrap();

    let adding = view_ok
        .as_variant::<jenkins_api::view::ListView>()
        .unwrap()
        .add_job(&jenkins, &job_ok.name)
        .await;
    assert!(adding.is_ok());

    let view_with = jenkins.get_view("test view").await;
    assert!(view_with.is_ok());
    assert!(view_with
        .unwrap()
        .jobs
        .iter()
        .map(|job| &job.name)
        .any(|job_name| job_name == "normal job"));

    let removing = view_ok
        .as_variant::<jenkins_api::view::ListView>()
        .unwrap()
        .remove_job(&jenkins, &job_ok.name)
        .await;
    assert!(removing.is_ok());

    let view_without = jenkins.get_view("test view").await;
    assert!(view_without.is_ok());
    assert!(!view_without
        .unwrap()
        .jobs
        .iter()
        .map(|job| &job.name)
        .any(|job_name| job_name == "normal job"));
}

#[tokio::test]
async fn can_add_and_remove_job_from_view_through_job() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    let view = jenkins.get_view("test view").await;
    println!("{:#?}", view);
    assert!(view.is_ok());
    let view_ok = view.unwrap();
    assert!(!view_ok
        .jobs
        .iter()
        .map(|job| &job.name)
        .any(|job_name| job_name == "pipeline job"));

    let job = jenkins.get_job("pipeline job").await;
    println!("{:#?}", job);
    assert!(job.is_ok());
    let job_ok = job.unwrap();

    let adding = job_ok.add_to_view(&jenkins, &view_ok.name).await;
    println!("{:#?}", adding);
    assert!(adding.is_ok());

    let view_with = jenkins.get_view("test view").await;
    println!("{:#?}", view_with);
    assert!(view_with.is_ok());
    assert!(view_with
        .unwrap()
        .jobs
        .iter()
        .map(|job| &job.name)
        .any(|job_name| job_name == "pipeline job"));

    let removing = job_ok.remove_from_view(&jenkins, &view_ok.name).await;
    println!("{:#?}", removing);
    assert!(removing.is_ok());

    let view_without = jenkins.get_view("test view").await;
    println!("{:#?}", view_without);
    assert!(view_without.is_ok());
    assert!(!view_without
        .unwrap()
        .jobs
        .iter()
        .map(|job| &job.name)
        .any(|job_name| job_name == "pipeline job"));
}

#[tokio::test]
async fn can_get_queue() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    let job = jenkins.get_job("long job").await;
    assert!(job.is_ok());
    let job_ok = job.unwrap();
    let triggered = job_ok
        .as_variant::<jenkins_api::job::FreeStyleProject>()
        .unwrap()
        .build(&jenkins)
        .await;
    assert!(triggered.is_ok());
    let queue = jenkins.get_queue().await;
    assert!(queue.is_ok());
}

#[tokio::test]
async fn can_get_queue_item() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    let job = jenkins.get_job("job name").await;
    assert!(job.is_ok());
    let triggered = job
        .unwrap()
        .as_variant::<jenkins_api::job::FreeStyleProject>()
        .unwrap()
        .build(&jenkins)
        .await;
    assert_that!(triggered).named("job triggered").is_ok();

    let triggered_ok = triggered.unwrap();

    let few_seconds = time::Duration::from_secs(2);
    for _ in 0..5 {
        assert!(triggered_ok.get_full_queue_item(&jenkins).await.is_ok());
        tokio::time::sleep(few_seconds).await;
    }
}

#[tokio::test]
async fn can_get_console() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    let job = jenkins.get_job("pipeline job").await;
    println!("{:#?}", job);
    assert!(job.is_ok());

    let job_ok = job.unwrap();
    let last_build = &job_ok.last_build;
    let build = last_build.as_ref().unwrap().get_full_build(&jenkins).await;
    println!("{:#?}", build);
    assert!(build.is_ok());

    let build_ok = build.unwrap();
    let console = build_ok.get_console(&jenkins).await;
    println!("{:#?}", console);
    assert!(console.is_ok());
}

#[tokio::test]
async fn can_get_config_xml() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    let job = jenkins.get_job("pipeline job").await;
    println!("{:#?}", job);
    assert!(job.is_ok());

    let job_ok = job.unwrap();
    let config = job_ok.get_config_xml(&jenkins).await;
    println!("{:#?}", config);
    assert!(config.is_ok());
}
#[tokio::test]
async fn can_get_pipeline() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    let job = jenkins.get_job("pipeline job").await;
    println!("{:#?}", job);
    assert!(job.is_ok());

    let build = jenkins.get_build("pipeline job", 1).await;
    println!("{:#?}", build);
    assert!(build.is_ok());
}

#[tokio::test]
async fn can_build_job_with_delay() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    let triggered = jenkins
        .job_builder("delayed job")
        .with_delay(5000)
        .send()
        .await;
    let triggered_ok = triggered.unwrap();

    let queue = jenkins.get_queue().await;
    assert!(queue.is_ok());

    tokio::time::sleep(time::Duration::from_secs(2)).await;
    let queue_item = triggered_ok.get_full_queue_item(&jenkins).await;
    assert!(queue_item.is_ok());
    assert!(queue_item.unwrap().why.is_some());

    tokio::time::sleep(time::Duration::from_secs(10)).await;

    let queue_item = triggered_ok.get_full_queue_item(&jenkins).await;
    assert!(queue_item.is_ok());
    assert!(queue_item.unwrap().why.is_none());
}

#[tokio::test]
async fn can_build_job_remotely() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    let triggered = jenkins
        .job_builder("remote job")
        .remotely_with_token_and_cause("remote_token", None)
        .unwrap()
        .send()
        .await;
    let triggered_ok = triggered.unwrap();

    let queue_item = triggered_ok.get_full_queue_item(&jenkins).await;
    assert!(queue_item.is_ok());
}

#[tokio::test]
async fn can_get_build_with_git() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    let job = jenkins.get_job("git triggered").await;
    assert!(job.is_ok());
    let build = jenkins.get_build("git triggered", 2).await;
    assert!(build.is_ok());
}

#[tokio::test]
async fn can_get_matrix_job() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    let job = jenkins.get_job("matrix job").await;
    assert_that!(job).named("getting a job").is_ok();

    let matrix_project = job.unwrap().as_variant::<jenkins_api::job::MatrixProject>();
    assert_that!(matrix_project)
        .named("was able to get as a MatrixProject")
        .is_ok();
    if let Ok(matrix_project) = matrix_project {
        assert_that!(
            matrix_project
                .last_build
                .unwrap()
                .get_full_build(&jenkins)
                .await
        )
        .named("getting last run of a matrix project")
        .is_ok();
        let config = matrix_project.active_configurations[0]
            .get_full_job(&jenkins)
            .await;
        assert_that!(config)
            .named("getting configuration of a matrix project")
            .is_ok();
        let full_build = config
            .unwrap()
            .last_build
            .unwrap()
            .get_full_build(&jenkins)
            .await;
        assert_that!(full_build)
            .named("getting last run of a matrix configuration")
            .is_ok();
        assert_that!(full_build.unwrap().get_console(&jenkins).await).is_ok();
    }

    let build = jenkins.get_build("matrix job", 1).await;
    assert_that!(build).is_ok();

    let matrix_build = build
        .unwrap()
        .as_variant::<jenkins_api::build::MatrixBuild>();
    assert_that!(matrix_build)
        .named("was abled to get as a MatrixBuild")
        .is_ok();
    if let Ok(matrix_build) = matrix_build {
        let run = matrix_build.runs[0].get_full_build(&jenkins).await;
        assert_that!(run).named("getting build of a run").is_ok();
    }
}

#[tokio::test]
async fn can_build_job_with_parameters() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    #[derive(Serialize)]
    struct Parameters {
        #[serde(rename = "bool-param")]
        bool_param: bool,
        #[serde(rename = "choose between")]
        choose_between: String,
        #[serde(rename = "free string param")]
        free_string_param: String,
    }

    let params = Parameters {
        bool_param: true,
        choose_between: "value2".to_string(),
        free_string_param: "my string param".to_string(),
    };

    let triggered = jenkins
        .job_builder("parameterized job")
        .with_parameters(&params)
        .unwrap()
        .send()
        .await;
    assert_that!(triggered).named("triggered job").is_ok();

    let queue_item = triggered.unwrap().get_full_queue_item(&jenkins).await;
    assert!(queue_item.is_ok());

    let queue_item_ok = queue_item.unwrap();

    let mut found_param1 = false;
    let mut found_param2 = false;
    let mut found_param3 = false;

    // print!("{:?}", queue_item_ok);
    for action in queue_item_ok.actions {
        if let Ok(parameters) = action.as_variant::<jenkins_api::action::ParametersAction>() {
            for param in parameters.parameters {
                print!("{:#?}", param);
                if let Ok(bool_param) =
                    param.as_variant::<jenkins_api::action::parameters::BooleanParameterValue>()
                {
                    found_param1 = bool_param.value == params.bool_param;
                }
                if let Ok(string_param) =
                    param.as_variant::<jenkins_api::action::parameters::StringParameterValue>()
                {
                    if string_param.value == params.choose_between {
                        found_param2 = true;
                    }
                    if string_param.value == params.free_string_param {
                        found_param3 = true;
                    }
                }
            }
        }
    }
    assert!(found_param1);
    assert!(found_param2);
    assert!(found_param3);
}

#[tokio::test]
async fn can_poll_scm() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    let job = jenkins.get_job("git triggered").await;
    assert!(job.is_ok());

    let poll = job
        .unwrap()
        .as_variant::<jenkins_api::job::FreeStyleProject>()
        .unwrap()
        .poll_scm(&jenkins)
        .await;
    assert!(poll.is_ok());

    assert!(jenkins.poll_scm_job("git triggered").await.is_ok());
}

#[tokio::test]
async fn can_get_maven_job() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    let job = jenkins.get_job("maven job").await;
    assert_that!(job).named("getting a job").is_ok();

    let maven_project = job
        .unwrap()
        .as_variant::<jenkins_api::job::MavenModuleSet>();
    assert_that!(maven_project)
        .named("was able to get as a MavenModuleSet")
        .is_ok();
    if let Ok(maven_project) = maven_project {
        assert_that!(
            maven_project
                .last_build
                .unwrap()
                .get_full_build(&jenkins)
                .await
        )
        .named("getting last run of a maven project")
        .is_ok();
        let module = maven_project.modules[0].get_full_job(&jenkins).await;
        assert_that!(module)
            .named("getting module of a maven project")
            .is_ok();
        assert_that!(
            module
                .unwrap()
                .last_build
                .unwrap()
                .get_full_build(&jenkins)
                .await
        )
        .named("getting last run of a maven module")
        .is_ok();
    }

    let build = jenkins.get_build("matrix job", 1).await;
    assert_that!(build).is_ok();

    let matrix_build = build
        .unwrap()
        .as_variant::<jenkins_api::build::MatrixBuild>();
    assert_that!(matrix_build)
        .named("was abled to get as a MatrixBuild")
        .is_ok();
    if let Ok(matrix_build) = matrix_build {
        let run = matrix_build.runs[0].get_full_build(&jenkins).await;
        assert_that!(run).named("getting build of a run").is_ok();
    }
}

#[tokio::test]
async fn can_get_build_with_alias() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    assert_that!(jenkins.get_build("normal job", "lastBuild").await).is_ok();
    assert_that!(jenkins.get_build("normal job", "lastSuccessfulBuild").await).is_ok();
    assert_that!(jenkins.get_build("normal job", "lastCompletedBuild").await).is_ok();
    assert_that!(jenkins.get_build("normal job", "zut").await).is_err();
}

#[tokio::test]
async fn can_get_nodes() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    assert_that!(jenkins.get_nodes().await).is_ok();
}

#[tokio::test]
async fn can_get_master() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    assert_that!(jenkins.get_node("(master)").await).is_ok();
    assert_that!(jenkins.get_master_node().await).is_ok();
}

#[tokio::test]
async fn can_get_master_while_building() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();

    let job: Result<jenkins_api::job::FreeStyleProject, _> =
        jenkins.get_job("long job").await.unwrap().as_variant();
    assert!(job.is_ok());
    assert!(job.unwrap().build(&jenkins).await.is_ok());

    println!("{:#?}", jenkins.get_master_node().await);
    assert_that!(jenkins.get_master_node().await).is_ok();
}

#[tokio::test]
async fn can_get_by_path_with_tree() {
    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct LastBuild {
        number: u8,
        duration: u8,
        result: String,
    }
    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct LasBuildOfJob {
        display_name: String,
        last_build: LastBuild,
    }

    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    let r: Result<LasBuildOfJob, _> = jenkins
        .get_object_as(
            jenkins_api::client::Path::Job {
                name: "normal job",
                configuration: None,
            },
            jenkins_api::client::TreeBuilder::new()
                .with_field("displayName")
                .with_field(
                    jenkins_api::client::TreeBuilder::object("lastBuild")
                        .with_subfield("number")
                        .with_subfield("duration")
                        .with_subfield("result"),
                )
                .build(),
        )
        .await;

    assert!(r.is_ok());
}

#[tokio::test]
async fn can_get_by_path_with_depth() {
    setup();
    let jenkins = JenkinsBuilder::new(JENKINS_URL)
        .with_user("user", Some("password"))
        .build()
        .unwrap();
    let r: Result<serde_json::Value, _> = jenkins
        .get_object_as(
            jenkins_api::client::Path::Job {
                name: "normal job",
                configuration: None,
            },
            jenkins_api::client::AdvancedQuery::Depth(2),
        )
        .await;

    assert!(r.is_ok());
}
