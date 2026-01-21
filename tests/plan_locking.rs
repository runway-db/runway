mod common;
use common::setup_fixture;
use runway::load_project;
use test_log::test;

#[test]
fn test_lock_plan() {
    let tmp_dir = setup_fixture("plan_locking");
    let base_dir = tmp_dir.path();

    let plan_dir = base_dir.join("plans/v0.1");

    let mut project = load_project(&base_dir).unwrap();
    project.lock_plan("v0.1").unwrap();

    // Verify lockfile exists
    let lock_path = plan_dir.join("plan.lock");
    assert!(lock_path.exists());

    // Reload project and verify lock
    let project = load_project(&base_dir).unwrap();
    let plan = project.get_plan("v0.1").unwrap();
    let lock = plan.lock().expect("Lockfile not found after locking");

    assert_eq!(lock.name, "v0.1");
    assert!(
        lock.engines
            .get(&runway::DatabaseEngine::Postgres)
            .unwrap()
            .contains_key("create_users")
    );

    let expected_hash = project
        .all_changes()
        .iter()
        .find(|c| c.name() == "create_users")
        .unwrap()
        .hash(&runway::DatabaseEngine::Postgres)
        .unwrap()
        .clone();
    assert_eq!(
        lock.engines
            .get(&runway::DatabaseEngine::Postgres)
            .unwrap()
            .get("create_users")
            .unwrap(),
        &expected_hash
    );
}
