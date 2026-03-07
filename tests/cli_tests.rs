use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_new_project_generation() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let out_dir = temp.path().join("my-service");

    let mut cmd = Command::cargo_bin("springboot-gen")?;
    cmd.arg("new")
        .arg("my-service")
        .arg("--group")
        .arg("com.acme")
        .arg("--features")
        .arg("postgres,redis,jwt,docker")
        .arg("--output")
        .arg(out_dir.as_os_str());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Project my-service is ready!"));

    // Verify key files were created
    assert!(out_dir.exists());
    assert!(out_dir.join("pom.xml").exists());
    assert!(out_dir.join("springboot-gen.toml").exists());
    assert!(out_dir.join("src/main/resources/application.yml").exists());
    assert!(out_dir.join("docker/postgres/docker-compose.yml").exists());
    assert!(out_dir.join("Dockerfile").exists());
    assert!(out_dir.join(".env.example").exists());

    // Verify Java structure exists
    assert!(out_dir.join("src/main/java/com/acme/my_service").exists());

    Ok(())
}

#[test]
fn test_features_list() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("springboot-gen")?;
    cmd.arg("features");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Redis cache"));

    Ok(())
}
