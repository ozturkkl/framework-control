use super::*;

#[test]
fn versions_compare_numerically_not_lexically() {
    assert!(is_newer("0.10.0", "0.9.0").unwrap());
    assert!(!is_newer("0.9.0", "0.10.0").unwrap());
    assert!(!is_newer("0.5.5", "0.5.5").unwrap());
}

#[test]
fn update_repo_accepts_owner_name_or_github_url() {
    let expected = Some(("ozturkkl".to_string(), "framework-control".to_string()));
    assert_eq!(parse_github_repo("ozturkkl/framework-control"), expected);
    assert_eq!(
        parse_github_repo("https://github.com/ozturkkl/framework-control"),
        expected
    );
    assert_eq!(
        parse_github_repo("https://github.com/ozturkkl/framework-control/"),
        expected
    );
}

#[test]
fn update_repo_without_owner_disables_updates() {
    assert_eq!(parse_github_repo(""), None);
    assert_eq!(parse_github_repo("framework-control"), None);
    assert_eq!(parse_github_repo("ozturkkl/"), None);
}
