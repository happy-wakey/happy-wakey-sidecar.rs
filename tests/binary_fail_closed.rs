use std::process::Command;

#[test]
fn missing_configuration_exits_nonzero_without_stdout_or_values() {
    let output = Command::new(env!("CARGO_BIN_EXE_happy-wakey-sidecar"))
        .env_clear()
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("\"error_class\":\"configuration\""));
    assert!(!stderr.contains("HAPPY_WAKEY_SHARED_AUTH_BASE_URL"));
}

#[test]
fn public_diagnostic_bind_is_rejected_even_with_valid_authorities() {
    let output = Command::new(env!("CARGO_BIN_EXE_happy-wakey-sidecar"))
        .env_clear()
        .env("HAPPY_WAKEY_SIDECAR_BIND", "0.0.0.0:9090")
        .env("HAPPY_WAKEY_SIDECAR_PRODUCT_KIND", "api")
        .env(
            "HAPPY_WAKEY_SIDECAR_PRODUCT_PROBE_URL",
            "http://127.0.0.1:8080/healthz",
        )
        .env(
            "HAPPY_WAKEY_SHARED_AUTH_BASE_URL",
            "https://auth.example.test",
        )
        .env(
            "HAPPY_WAKEY_OPTO_SYNC_BASE_URL",
            "http://opto-sync.sync.svc",
        )
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("\"error_class\":\"bind_policy\""));
    assert!(!stderr.contains("auth.example.test"));
}
