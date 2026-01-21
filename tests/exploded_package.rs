mod common;
use common::setup_fixture;
use runway::{package_migrations, Migrator, Package, adapters};
use test_log::test;

#[test]
fn exploded_package_migrations() {
    let tmp_dir = setup_fixture("simple");
    let base_dir = tmp_dir.path();
    let output_path = base_dir.join("exploded_package");
    
    // Create exploded package
    package_migrations(base_dir, &output_path, true).unwrap();

    // Verify it is a directory
    assert!(output_path.is_dir());
    assert!(output_path.join("package.json").exists());
    assert!(output_path.join("engines").is_dir());

    let mut package = Package::load(&output_path).unwrap();
    assert_eq!(package.engines().len(), 2);
    assert!(package.engines().contains(&runway::DatabaseEngine::Sqlite));
    assert!(package.engines().contains(&runway::DatabaseEngine::Postgres));

    let engine = runway::DatabaseEngine::Sqlite;
    let metadata = package.engine_metadata(&engine).unwrap();
    assert_eq!(metadata.sequence().len(), 8);

    let deploy = package.deploy_script(&engine, "create_users").unwrap();
    assert!(deploy.contains("CREATE TABLE users"));

    // Now test migration using the exploded package
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let adapter = adapters::Rusqlite::new(&conn);
    let mut migrator = Migrator::new(adapter, &mut package);

    migrator.apply().expect("Failed to apply migrations from exploded package");

    // Verify it worked
    let user_count: i32 = conn.query_row("SELECT count(*) FROM users", [], |r| r.get(0)).unwrap();
    assert_eq!(user_count, 0);

    // Verify history table
    let history_count: i32 = conn.query_row("SELECT count(*) FROM _runway_history WHERE success = 1", [], |r| r.get(0)).unwrap();
    assert_eq!(history_count, 8);
}
