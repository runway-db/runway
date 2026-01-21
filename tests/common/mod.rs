use std::fs;
use std::path::Path;
use tempdir::TempDir;

pub fn setup_fixture(fixture_name: &str) -> TempDir {
    let tmp_dir = TempDir::new(&format!("runway_test_{}", fixture_name)).unwrap();
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(fixture_name);

    copy_dir_all(&fixture_path, tmp_dir.path()).expect("Failed to copy fixture");

    tmp_dir
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
