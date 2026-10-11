use super::*;

#[test]
fn zip_extraction_stays_inside_target_dir() {
    let root = std::env::temp_dir().join(format!("fc-extract-test-{}", std::process::id()));
    let zip_path = root.join("archive.zip");
    let target_dir = root.join("out");
    std::fs::create_dir_all(&root).unwrap();

    let mut writer = zip::ZipWriter::new(File::create(&zip_path).unwrap());
    writer.start_file("bin/tool", Default::default()).unwrap();
    writer.write_all(b"tool").unwrap();
    writer.start_file("../escape.txt", Default::default()).unwrap();
    writer.write_all(b"escape").unwrap();
    writer.finish().unwrap();

    let extracted = extract_zip_to(&zip_path, &target_dir).unwrap();

    assert_eq!(
        extracted,
        vec![target_dir.join("bin/tool"), target_dir.join("escape.txt")]
    );
    assert!(!root.join("escape.txt").exists());
    std::fs::remove_dir_all(&root).unwrap();
}
