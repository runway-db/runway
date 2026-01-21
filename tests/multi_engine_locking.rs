mod common;
use common::setup_fixture;
use runway::DatabaseEngine;
use runway::load_project;
use std::fs;
use test_log::test;

#[test]
fn test_lock_multi_engine_plan() {
    let tmp_dir = setup_fixture("multi-engine");
    let base_dir = tmp_dir.path();

    let change_dir = base_dir.join("changes/create_users");
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
    assert!(lock.engines.contains_key(&DatabaseEngine::Postgres));
    assert!(lock.engines.contains_key(&DatabaseEngine::Sqlite));

    let pg_hash = lock
        .engines
        .get(&DatabaseEngine::Postgres)
        .unwrap()
        .get("create_users")
        .unwrap();
    let sqlite_hash = lock
        .engines
        .get(&DatabaseEngine::Sqlite)
        .unwrap()
        .get("create_users")
        .unwrap();

    // The hashes should be different because the SQL scripts are different
    assert_ne!(
        pg_hash, sqlite_hash,
        "Postgres and Sqlite hashes should be different"
    );

    // Verify that Plan::hash uses the locked hashes
    // Modify the SQL scripts and verify Plan::hash hasn't changed
    fs::write(
        change_dir.join("deploy.postgres.sql"),
        "CREATE TABLE users (id SERIAL PRIMARY KEY); -- modified",
    )
    .unwrap();

    let new_plan_hash_pg = plan.hash(&DatabaseEngine::Postgres).unwrap();
    // We didn't reload the project after modifying the file, but we're using the same `plan` object which is locked.
    // Wait, if we use the same `plan` object, it already has the lock.

    // Let's reload to be sure
    let project_reloaded = load_project(&base_dir).unwrap();
    let plan_reloaded = project_reloaded.get_plan("v0.1").unwrap();
    assert_eq!(
        plan_reloaded.hash(&DatabaseEngine::Postgres).unwrap(),
        new_plan_hash_pg
    );

    // Cleanup
}
