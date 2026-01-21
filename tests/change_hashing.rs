mod common;
use common::setup_fixture;
use runway::DatabaseEngine;
use runway::load_project;
use std::fs;

#[test]
fn test_change_hash_includes_requirements() {
    let tmp_dir = setup_fixture("change_hashing");
    let path = tmp_dir.path();

    let change_dir = path.join("changes/c1");

    let project = load_project(path).unwrap();
    let change = project
        .all_changes()
        .into_iter()
        .find(|c| c.name() == "c1")
        .unwrap();
    let hash1 = change.hash(&DatabaseEngine::Postgres).unwrap();

    // Update change.toml to add a requirement
    fs::write(
        change_dir.join("change.toml"),
        "description = \"c1 desc\"\nrequires = [\"other\"]",
    )
    .unwrap();

    // We need to reload the project to pick up the change.toml modification
    let project2 = load_project(path).unwrap();
    let change2 = project2
        .all_changes()
        .into_iter()
        .find(|c| c.name() == "c1")
        .unwrap();
    let hash2 = change2.hash(&DatabaseEngine::Postgres).unwrap();

    assert_ne!(hash1, hash2, "Hash should change when requirements change");

    // Update change.toml to add a rework
    fs::write(
        change_dir.join("change.toml"),
        "description = \"c1 desc\"\nreworks = \"other_old\"",
    )
    .unwrap();

    let project3 = load_project(path).unwrap();
    let change3 = project3
        .all_changes()
        .into_iter()
        .find(|c| c.name() == "c1")
        .unwrap();
    let hash3 = change3.hash(&DatabaseEngine::Postgres).unwrap();

    assert_ne!(hash1, hash3, "Hash should change when reworks changes");
    assert_ne!(
        hash2, hash3,
        "Hash should change when reworks changes (compared to requirements only)"
    );
}
