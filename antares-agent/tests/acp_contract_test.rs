use std::{path::PathBuf, process::Command, sync::Arc, time::Duration};

use antares_agent::{
    codec::decode_line,
    protocol::{ContentBlock, SessionEvent},
    AgentError, AgentSession, ProcessLaunchOptions, ProcessSupervisor,
};

fn find_python() -> Option<&'static str> {
    if Command::new("python3").arg("--version").output().is_ok() {
        Some("python3")
    } else if Command::new("python").arg("--version").output().is_ok() {
        Some("python")
    } else {
        None
    }
}

fn fake_agent_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("fake-agent")
        .join("fake_agent.py")
}

#[test]
fn test_acp_fixture_frames_parsing() {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("acp")
        .join("session_flow.jsonl");

    let content = std::fs::read_to_string(fixture_path).expect("fixture file should exist");
    let mut parsed_count = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let msg = decode_line(trimmed).expect("fixture line should decode cleanly");
        parsed_count += 1;
        match msg {
            antares_agent::codec::JsonRpcMessage::Request(r) => {
                assert!(!r.method.is_empty());
            }
            antares_agent::codec::JsonRpcMessage::Response(_) => {}
            antares_agent::codec::JsonRpcMessage::Notification(n) => {
                assert_eq!(n.method, "session/update");
            }
            antares_agent::codec::JsonRpcMessage::Error(_) => {}
        }
    }
    assert_eq!(parsed_count, 6);
}

#[test]
fn test_mock_agent_happy_path_session_flow() {
    let python = match find_python() {
        Some(p) => p,
        None => {
            eprintln!("Python not available; skipping live child test");
            return;
        }
    };

    let script = fake_agent_script();
    let options = ProcessLaunchOptions::new(python)
        .with_arg(script.to_str().unwrap())
        .with_arg("--scenario")
        .with_arg("echo");

    let supervisor = Arc::new(ProcessSupervisor::spawn(options).expect("spawn supervisor"));
    let session = AgentSession::new(supervisor.clone());

    // 1. Initialize
    let init_res = session
        .initialize(Duration::from_secs(3))
        .expect("initialize handshake");
    assert_eq!(init_res.protocol_version, "2024-11-05");
    assert_eq!(
        init_res.agent_info.name.as_deref(),
        Some("mock-octomind")
    );

    // 2. New session
    let sess_res = session
        .new_session("/mock/workspace", None, Duration::from_secs(3))
        .expect("create session");
    assert_eq!(sess_res.session_id, "mock-session-001");
    assert_eq!(sess_res.state.as_deref(), Some("ready"));

    // 3. Prompt turn
    let prompt_handle = std::thread::spawn({
        let session = AgentSession::new(supervisor.clone());
        move || {
            session.prompt(
                vec![ContentBlock::text("test prompt")],
                Duration::from_secs(3),
            )
        }
    });

    // 4. Consume streamed token events
    let mut tokens = Vec::new();
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        if let Ok(Some(SessionEvent::TextDelta(delta))) =
            session.next_event(Duration::from_millis(200))
        {
            tokens.push(delta);
            if tokens.len() >= 2 {
                break;
            }
        }
    }

    assert_eq!(tokens, vec!["Hello ", "from mock agent!"]);
    let prompt_res = prompt_handle.join().unwrap().expect("prompt result");
    assert_eq!(prompt_res.get("status").and_then(|s| s.as_str()), Some("completed"));

    // 5. Cancel
    session.cancel().expect("cancel notification");

    // 6. Graceful shutdown
    session.shutdown(Duration::from_secs(2)).expect("shutdown");
}

#[test]
fn test_mock_agent_crash_detection() {
    let python = match find_python() {
        Some(p) => p,
        None => return,
    };

    let script = fake_agent_script();
    let options = ProcessLaunchOptions::new(python)
        .with_arg(script.to_str().unwrap())
        .with_arg("--scenario")
        .with_arg("crash");

    let supervisor = ProcessSupervisor::spawn(options).expect("spawn supervisor");
    // Wait slightly for child to exit with 42
    std::thread::sleep(Duration::from_millis(200));

    let res = supervisor.send_request("initialize", None, Duration::from_millis(500));
    assert!(res.is_err());
    match res {
        Err(AgentError::ProcessExited(code)) => {
            assert!(code == Some(42) || code.is_none());
        }
        other => panic!("expected ProcessExited, got {other:?}"),
    }
}

#[test]
fn test_mock_agent_timeout_handling() {
    let python = match find_python() {
        Some(p) => p,
        None => return,
    };

    let script = fake_agent_script();
    let options = ProcessLaunchOptions::new(python)
        .with_arg(script.to_str().unwrap())
        .with_arg("--scenario")
        .with_arg("delay");

    let supervisor = ProcessSupervisor::spawn(options).expect("spawn supervisor");
    // Request with small timeout
    let res = supervisor.send_request("initialize", None, Duration::from_millis(100));
    assert!(matches!(res, Err(AgentError::Timeout(_))));

    let _ = supervisor.shutdown(Duration::from_millis(500));
}

#[test]
fn test_mock_agent_stderr_flood_is_bounded() {
    let python = match find_python() {
        Some(p) => p,
        None => return,
    };

    let script = fake_agent_script();
    let options = ProcessLaunchOptions::new(python)
        .with_arg(script.to_str().unwrap())
        .with_arg("--scenario")
        .with_arg("stderr-flood");

    let supervisor = Arc::new(ProcessSupervisor::spawn(options).expect("spawn supervisor"));
    let session = AgentSession::new(supervisor.clone());

    // Even with 1,000 stderr lines flooding, stdout requests must still succeed
    let init_res = session
        .initialize(Duration::from_secs(4))
        .expect("initialize handshake despite stderr flood");
    assert_eq!(init_res.protocol_version, "2024-11-05");

    let stderr = supervisor.stderr_tail(20);
    assert!(!stderr.is_empty());
    assert!(stderr.len() <= 20);

    let _ = session.shutdown(Duration::from_secs(1));
}
