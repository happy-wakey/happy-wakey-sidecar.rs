use std::collections::BTreeSet;

use happy_wakey_sidecar::config::{
    BIND_ENV, FAILURE_THRESHOLD_ENV, INTERVAL_MS_ENV, OPTO_SYNC_BASE_URL_ENV, PRODUCT_KIND_ENV,
    PRODUCT_PROBE_URL_ENV, SHARED_AUTH_BASE_URL_ENV, SUCCESS_THRESHOLD_ENV,
};

#[test]
fn generated_schema_and_rust_config_cover_the_same_environment_surface() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../generated/json-schema/env.schema.json")).unwrap();
    let actual = schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::from([
        BIND_ENV,
        PRODUCT_KIND_ENV,
        PRODUCT_PROBE_URL_ENV,
        SHARED_AUTH_BASE_URL_ENV,
        OPTO_SYNC_BASE_URL_ENV,
        INTERVAL_MS_ENV,
        SUCCESS_THRESHOLD_ENV,
        FAILURE_THRESHOLD_ENV,
    ]);
    assert_eq!(actual, expected);
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["x-flags2env-source"], ".cli-flags.toml");
}

#[test]
fn k8s_fragment_preserves_loopback_and_exec_probe_policy() {
    let manifest = include_str!("../k8s/container.yaml");
    assert!(manifest.contains("value: 127.0.0.1:9090"));
    assert!(manifest.contains("probe-healthz"));
    assert!(manifest.contains("allowPrivilegeEscalation: false"));
    assert!(manifest.contains("readOnlyRootFilesystem: true"));
    assert!(!manifest.contains("readinessProbe:"));
    assert!(!manifest.contains("secretKeyRef:"));
}

#[test]
fn formal_and_runtime_phase_names_remain_aligned() {
    let model = include_str!("../formal/sidecar_lifecycle.qnt");
    for phase in ["Booting", "Probing", "Ready", "Degraded", "Draining"] {
        assert!(model.contains(phase));
    }
}
