use forgeos_lib::{infrastructure::github::GitHubRestClient, ports::GitHubClient};

#[tokio::test]
#[ignore = "requires live access to the public GitHub REST API"]
async fn reads_public_repository_through_read_only_api() {
    let client = GitHubRestClient::new().expect("GitHub client should initialize");
    let token = std::env::var("GITHUB_TOKEN").ok();
    let facts = client
        .inspect_repository("octocat", "Hello-World", token.as_deref())
        .await
        .expect("public repository should be readable without a token");

    assert_eq!(facts.owner, "octocat");
    assert_eq!(facts.name, "Hello-World");
    assert!(facts
        .url
        .starts_with("https://github.com/octocat/Hello-World"));
    assert!(!facts.default_branch.is_empty());
    assert!(!facts.recent_commits.is_empty());
}
