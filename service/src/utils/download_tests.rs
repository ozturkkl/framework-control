use super::*;

#[test]
fn split_archive_name_strips_only_archive_extensions() {
    assert_eq!(
        split_archive_name("ryzenadj-win64.zip"),
        Some((ArchiveKind::Zip, "ryzenadj-win64"))
    );
    assert_eq!(
        split_archive_name("framework-control-linux.tar.gz"),
        Some((ArchiveKind::TarGz, "framework-control-linux"))
    );
    assert_eq!(split_archive_name("Tool.TGZ"), Some((ArchiveKind::TarGz, "Tool")));
    assert_eq!(split_archive_name("framework_tool.exe"), None);
}
