#[macro_use]
extern crate proptest;

extern crate env_logger;

extern crate jenkins_api;

use jenkins_api::JenkinsBuilder;

use std::sync::Once;

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        env_logger::init();
    });
}

static JENKINS_URL: &str = "http://localhost:8080";
// FIXME: Waiting on https://github.com/proptest-rs/proptest/issues/442 to be fixed before these can be reenabled
/*
proptest! {
    #[tokio::test]
    async fn doesnt_crash_user(ref s in "\\PC*") {
        setup();
        let jenkins = JenkinsBuilder::new(JENKINS_URL)
            .with_user(&s, Some("password"))
            .build()
            .unwrap();
        jenkins.get_home().await.ok();
    }
}

proptest! {
    #[tokio::test]
    async fn doesnt_crash_url(ref s in "\\PC*") {
        setup();
        if let Ok(jenkins) = JenkinsBuilder::new(&s)
            .with_user("user", Some("password"))
            .build()
        {
            jenkins.get_home().unwrap();
        }
    }
}

proptest! {
    #[tokio::test]
    async fn doesnt_crash_job_name(ref s in "\\PC*") {
        setup();
        let jenkins = JenkinsBuilder::new(JENKINS_URL)
            .with_user("user", Some("password"))
            .build()
            .unwrap();
        jenkins.get_job(s).await.ok();
    }
}
 */
