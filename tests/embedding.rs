mod common;
use common::setup_fixture;
use runway::{package_migrations, Package};
use test_log::test;

#[test]
fn test_from_bytes() {
    let tmp_dir = setup_fixture("simple");
    let base_dir = tmp_dir.path();
    let output_path = base_dir.join("output.zip");
    
    package_migrations(base_dir, &output_path, false).unwrap();
    
    let bytes = std::fs::read(&output_path).unwrap();
    // In a real scenario, this would be include_bytes!
    // But since we want to test with dynamic bytes, we'll leak it to get &'static [u8]
    let static_bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    
    let mut package = Package::from_bytes(static_bytes).unwrap();
    assert_eq!(package.engines().len(), 2);
    assert!(package.engines().contains(&runway::DatabaseEngine::Sqlite));
    
    let engine = runway::DatabaseEngine::Sqlite;
    let deploy = package.deploy_script(&engine, "create_users").unwrap();
    assert!(deploy.contains("CREATE TABLE users"));
}

#[test]
fn test_package_project() {
    let tmp_dir = setup_fixture("simple");
    let base_dir = tmp_dir.path();

    // Set OUT_DIR
    let out_dir = tmp_dir.path().join("out");
    std::fs::create_dir(&out_dir).unwrap();
    unsafe {
        std::env::set_var("OUT_DIR", &out_dir);
    }

    runway::package_project(base_dir).unwrap();

    let expected_package = out_dir.join("migrations.runway");
    assert!(expected_package.exists());

    let package = Package::load(&expected_package).unwrap();
    assert_eq!(package.engines().len(), 2);
}

#[test]
fn test_package_named_project() {
    let tmp_dir = setup_fixture("simple");
    let base_dir = tmp_dir.path();

    // Set OUT_DIR
    let out_dir = tmp_dir.path().join("out_named");
    std::fs::create_dir(&out_dir).unwrap();
    unsafe {
        std::env::set_var("OUT_DIR", &out_dir);
    }

    runway::package_named_project(base_dir, "core").unwrap();

    let expected_package = out_dir.join("core.runway");
    assert!(expected_package.exists());

    let package = Package::load(&expected_package).unwrap();
    assert_eq!(package.engines().len(), 2);
}


#[test]
fn test_package_named_project_with_dash() {
    let tmp_dir = setup_fixture("simple");
    let base_dir = tmp_dir.path();

    // Set OUT_DIR
    let out_dir = tmp_dir.path().join("out_dash");
    std::fs::create_dir(&out_dir).unwrap();
    unsafe {
        std::env::set_var("OUT_DIR", &out_dir);
    }

    runway::package_named_project(base_dir, "my-migrations").unwrap();

    let expected_package = out_dir.join("my-migrations.runway");
    assert!(expected_package.exists());

    let package = Package::load(&expected_package).unwrap();
    assert_eq!(package.engines().len(), 2);
}

#[test]
fn test_package_macro() {
    let tmp_dir = setup_fixture("simple");

    // Set OUT_DIR
    let out_dir = tmp_dir.path().join("out_macro");
    std::fs::create_dir(&out_dir).unwrap();
    unsafe {
        std::env::set_var("OUT_DIR", &out_dir);
    }

    // Since we can't easily use dynamic paths with the macro (as it requires literals),
    // and we don't want to change CWD of the whole test process,
    // we'll just check that it compiles and we could potentially use it with a relative literal path
    // if we were in a build script.

    // For testing purposes, we'll use a relative path that we know exists or create it.
    // In many environments, "tests/fixtures/simple" exists relative to project root.
    runway::package!("tests/fixtures/simple", "macro-test");

    let expected_package = out_dir.join("macro-test.runway");
    assert!(expected_package.exists());

    let package = runway::Package::load(&expected_package).unwrap();
    assert_eq!(package.engines().len(), 2);
}

#[test]
fn test_package_macro_single_arg() {
    let tmp_dir = setup_fixture("simple");

    // Set OUT_DIR
    let out_dir = tmp_dir.path().join("out_macro_single");
    std::fs::create_dir_all(out_dir.join("tests/fixtures")).unwrap();
    unsafe {
        std::env::set_var("OUT_DIR", &out_dir);
    }

    // This will package directory "tests/fixtures/simple" and name it "tests/fixtures/simple"
    runway::package!("tests/fixtures/simple");

    let expected_package = out_dir.join("tests/fixtures/simple.runway");
    assert!(expected_package.exists());

    let package = runway::Package::load(&expected_package).unwrap();
    assert_eq!(package.engines().len(), 2);
}
