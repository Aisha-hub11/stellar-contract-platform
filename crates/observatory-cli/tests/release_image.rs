//! Structural checks for the release container image and its publish job.
//!
//! The image is built and pushed only by the release workflow, so these checks
//! validate the committed definition rather than a running container.

const DOCKERFILE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Dockerfile"));
const DOCKERIGNORE: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.dockerignore"));
const RELEASE_WORKFLOW: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../.github/workflows/release.yml"
));
const DOCS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/container-image.md"
));

fn from_stages(contents: &str) -> Vec<&str> {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("FROM "))
        .collect()
}

#[test]
fn image_is_a_non_root_multi_stage_build() {
    let stages = from_stages(DOCKERFILE);
    assert_eq!(stages.len(), 2, "expected a builder and a runtime stage");
    assert!(stages[0].contains("rust:1.88"));
    assert!(stages[1].contains("debian:bookworm-slim"));
    assert!(DOCKERFILE.contains(
        "COPY --from=builder /build/target/release/stellar-contract-platform /usr/local/bin/stellar-contract-platform"
    ));

    let user_lines: Vec<&str> = DOCKERFILE
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("USER "))
        .collect();
    assert_eq!(user_lines, vec!["USER 10001"]);
    assert!(!DOCKERFILE.contains("USER root"));
    assert!(DOCKERIGNORE.lines().any(|line| line.trim() == "target"));
}

#[test]
fn image_runs_the_api_and_exposes_health() {
    assert!(DOCKERFILE.contains("EXPOSE 8080"));
    assert!(DOCKERFILE.contains("ENTRYPOINT [\"stellar-contract-platform\"]"));
    assert!(DOCKERFILE.contains("CMD [\"api\", \"serve\", \"--bind\", \"0.0.0.0:8080\"]"));
    assert!(DOCKERFILE.contains("/api/v1/health"));
    assert!(DOCKERFILE.contains("HEALTHCHECK"));
}

#[test]
fn image_definition_contains_no_secrets() {
    for line in DOCKERFILE.lines().map(str::trim) {
        let upper = line.to_ascii_uppercase();
        for marker in ["SECRET", "TOKEN", "PASSWORD", "API_KEY"] {
            assert!(
                !upper.contains(marker),
                "a secret-looking value is baked into the Dockerfile: `{line}`"
            );
        }
        assert!(
            !upper.starts_with("ARG ") || !upper.contains("KEY"),
            "build arguments must not carry keys: `{line}`"
        );
        assert!(
            !line.contains("--key"),
            "API keys must be supplied at run time, not baked in: `{line}`"
        );
    }
    assert!(!DOCKERFILE.contains("--no-auth"));
}

#[test]
fn release_workflow_publishes_the_image_on_tags() {
    assert!(RELEASE_WORKFLOW.contains("tags: [\"v*.*.*\"]"));
    assert!(RELEASE_WORKFLOW.contains("Publish container image"));
    assert!(RELEASE_WORKFLOW.contains("ghcr.io/${GITHUB_REPOSITORY,,}"));
    assert!(RELEASE_WORKFLOW.contains("docker build"));
    assert!(RELEASE_WORKFLOW.contains("docker push"));
    assert!(RELEASE_WORKFLOW.contains("packages: write"));
    assert!(RELEASE_WORKFLOW.contains("--password-stdin"));

    let push = RELEASE_WORKFLOW.find("docker push").expect("push step");
    let smoke = RELEASE_WORKFLOW.find("/api/v1/health").expect("smoke test");
    assert!(smoke < push, "the smoke test must run before the push");

    assert!(DOCS.contains("has published an image yet"));
}

#[test]
fn release_actions_are_sha_pinned() {
    for line in RELEASE_WORKFLOW.lines() {
        let line = line.trim();
        let reference = line
            .strip_prefix("- uses: ")
            .or_else(|| line.strip_prefix("uses: "));
        let Some(reference) = reference else {
            continue;
        };
        let reference = reference
            .split_whitespace()
            .next()
            .expect("action reference");
        let (action, revision) = reference
            .split_once('@')
            .unwrap_or_else(|| panic!("unpinned action `{reference}`"));
        assert!(
            revision.len() == 40 && revision.chars().all(|c| c.is_ascii_hexdigit()),
            "action `{action}` must be pinned to a 40-character commit SHA"
        );
    }
}
