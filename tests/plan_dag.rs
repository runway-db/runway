mod common;
use common::setup_fixture;
use runway::{DatabaseEngine, load_project};
use test_log::test;

#[test]
fn test_plan_as_parent() {
    let tmp_dir = setup_fixture("plan_dag");
    let base_dir = tmp_dir.path();

    let project = load_project(&base_dir).unwrap();
    let engine = DatabaseEngine::Postgres;
    let nodes = project.changes_for_engine(&engine).unwrap();

    // Expected order:
    // 1. create_users (target of v0.1)
    // 2. @v0.1 (the plan node itself)
    // 3. dependent_change (requires @v0.1)

    let names: Vec<String> = nodes.iter().map(|n| n.name().clone()).collect();

    let pos_users = names
        .iter()
        .position(|n| n == "create_users")
        .expect("create_users not found");
    let pos_v01 = names
        .iter()
        .position(|n| n == "v0.1")
        .expect("v0.1 not found");
    let pos_dep = names
        .iter()
        .position(|n| n == "dependent_change")
        .expect("dependent_change not found");

    assert!(pos_users < pos_v01, "create_users must come before v0.1");
    assert!(pos_v01 < pos_dep, "v0.1 must come before dependent_change");
}
