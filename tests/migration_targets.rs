mod common;
use common::setup_fixture;
use runway::{Migrator, build::Project, adapters};
use test_log::test;

#[test]
fn test_apply_to_and_revert() {
    let tmp_dir = setup_fixture("migration_targets");
    let base_dir = tmp_dir.path();
    
    let mut project = Project::load(base_dir).unwrap();
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let adapter = adapters::Rusqlite::new(&conn);
    let mut migrator = Migrator::new(adapter, &mut project);

    // Apply up to 'create_users'
    migrator.apply_to(Some("create_users")).expect("Failed to apply to create_users");
    
    // Verify tables
    let tables: Vec<String> = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    
    assert!(tables.contains(&"users".to_string()));
    
    migrator.apply_to(Some("create_profiles")).expect("Failed to apply to create_profiles");
    
    let tables: Vec<String> = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(tables.contains(&"profiles".to_string()));
    
    // Revert to 'create_users'
    migrator.revert(Some("create_users")).expect("Failed to revert to create_users");
    
    let tables: Vec<String> = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
        
    assert!(tables.contains(&"users".to_string()));
    assert!(!tables.contains(&"profiles".to_string()));
    
    // Test reverting EVERYTHING
    migrator.revert(None).expect("Failed to revert everything");
    let tables: Vec<String> = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(!tables.contains(&"users".to_string()));
    
    // Check history table for reverted_at
    let reverted_at: Option<String> = conn.query_row(
        "SELECT reverted_at FROM _runway_history WHERE name = 'create_profiles'",
        [],
        |r| r.get(0)
    ).unwrap();
    assert!(reverted_at.is_some());

    // Re-apply migrations successfully
    migrator.apply().expect("Failed to re-apply migrations after revert");

    let tables: Vec<String> = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    
    assert!(tables.contains(&"users".to_string()));
    assert!(tables.contains(&"profiles".to_string()));

    // Verify history table has NEW successful entries (since get_applied_migration excludes reverted ones)
    let history_count: i32 = conn.query_row("SELECT count(*) FROM _runway_history WHERE success = 1 AND reverted_at IS NULL", [], |r| r.get(0)).unwrap();
    // In migration_targets fixture, how many changes?
    // Let's check the fixture's engine metadata if possible, but we know it applied at least users and profiles.
    assert!(history_count >= 2);
}
