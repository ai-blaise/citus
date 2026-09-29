use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn serve_without_kubernetes_credentials_exits_instead_of_advertising_ready() {
    let missing_config = std::env::temp_dir().join(format!(
        "ai-blaise-missing-kubeconfig-{}",
        std::process::id()
    ));
    let mut child = Command::new(env!("CARGO_BIN_EXE_ai_blaise_citus_operator"))
        .arg("serve")
        .env("KUBECONFIG", &missing_config)
        .env("AI_BLAISE_LISTEN_ADDR", "127.0.0.1:0")
        .env("AI_BLAISE_OPERATOR_CONTROLLERS", "sidecar")
        .env_remove("KUBERNETES_SERVICE_HOST")
        .env_remove("KUBERNETES_SERVICE_PORT")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn operator serve");

    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll operator serve") {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill stuck probe-only operator");
            child.wait().expect("reap stuck operator");
            panic!("operator stayed alive without Kubernetes credentials");
        }
        thread::sleep(Duration::from_millis(20));
    };

    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("operator stderr")
        .read_to_string(&mut stderr)
        .expect("read operator stderr");
    assert!(!status.success());
    assert!(
        stderr.contains("Kubernetes client initialization failed"),
        "unexpected stderr: {stderr}"
    );
    assert!(!stderr.contains("probe server listening"));
}
