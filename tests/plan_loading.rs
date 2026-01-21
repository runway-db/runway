mod common;
use common::setup_fixture;
use runway::{build::Plan, load_project};
use test_log::test;

#[test]
fn test_load_plan_with_lock() {
    let tmp_dir = setup_fixture("plan_loading");
    let base_dir = tmp_dir.path();
    let project = load_project(base_dir).unwrap();

    let plans = project.plans();
    let plan = plans
        .into_iter()
        .find(|p: &Plan| p.name() == "v0.8")
        .expect("Plan v0.8 not found");
    assert_eq!(plan.name(), "v0.8");
    assert_eq!(plan.targets(), &["create_users", "create_posts"]);

    let lock = plan.lock().expect("Lockfile not found for v0.8");
    assert_eq!(lock.name, "v0.8");
    let pg_hashes = lock
        .engines
        .get(&runway::DatabaseEngine::Postgres)
        .expect("No postgres hashes in lockfile");
    assert_eq!(
        pg_hashes.get("create_users").unwrap(),
        "f9f1e4e1eb52aadc205243aec74fe6c60239ccb6ee006a83f710e3d7d5c23ad5"
    );
    assert_eq!(
        pg_hashes.get("create_posts").unwrap(),
        "c1df13faebfdb78118695cf959f2774b16b97549d147a015e8ba068427789661"
    );
}

#[test]
fn test_load_plan_without_lock() {
    let tmp_dir = setup_fixture("plan_loading");
    let base_dir = tmp_dir.path();

    let project = load_project(&base_dir).unwrap();
    let plans = project.plans();
    let plan = plans
        .into_iter()
        .find(|p: &Plan| p.name() == "wip")
        .expect("Plan wip not found");
    assert!(plan.lock().is_none());
}
