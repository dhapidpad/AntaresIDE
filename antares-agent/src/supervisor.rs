use std::{
    collections::{HashMap, VecDeque},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crossbeam_channel::{Receiver, Sender, bounded};
use parking_lot::Mutex;
use serde_json::Value;

use crate::{
    codec::{
        JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, RequestId, decode_line,
        encode_line,
    },
    error::AgentError,
    protocol::SessionUpdateParams,
};

/// Default capacity for the bounded stderr ring buffer.
const MAX_STDERR_LINES: usize = 500;

/// Default capacity for internal bounded channels.
const DEFAULT_CHANNEL_CAPACITY: usize = 256;

/// Configuration for launching an agent process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessLaunchOptions {
    pub binary: PathBuf,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub envs: HashMap<String, String>,
    pub sandbox: bool,
}

impl ProcessLaunchOptions {
    pub fn new(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            args: Vec::new(),
            cwd: None,
            envs: HashMap::new(),
            sandbox: false,
        }
    }

    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_env(
        mut self,
        key: impl Into<String>,
        val: impl Into<String>,
    ) -> Self {
        self.envs.insert(key.into(), val.into());
        self
    }

    pub fn with_sandbox(mut self, sandbox: bool) -> Self {
        self.sandbox = sandbox;
        self
    }
}

/// Events emitted asynchronously by the agent process.
#[derive(Debug, Clone, PartialEq)]
pub enum SupervisorEvent {
    /// Server sent a session update notification.
    SessionUpdate(SessionUpdateParams),
    /// Server sent an arbitrary notification.
    Notification(JsonRpcNotification),
    /// Child process exited.
    ProcessExited { status: Option<i32> },
}

type PendingRequests =
    Arc<Mutex<HashMap<RequestId, Sender<Result<Value, AgentError>>>>>;
type StderrBuffer = Arc<Mutex<VecDeque<String>>>;

/// Low-level process supervisor managing child stdio and JSON-RPC message correlation.
pub struct ProcessSupervisor {
    child: Arc<Mutex<Option<Child>>>,
    request_counter: AtomicI64,
    writer_tx: Sender<String>,
    pending_requests: PendingRequests,
    stderr_buffer: StderrBuffer,
    event_rx: Receiver<SupervisorEvent>,
    is_running: Arc<AtomicBool>,
    _reader_handle: JoinHandle<()>,
    _stderr_handle: JoinHandle<()>,
    _writer_handle: JoinHandle<()>,
}

impl ProcessSupervisor {
    /// Spawn a child agent process and attach I/O readers/writers.
    pub fn spawn(options: ProcessLaunchOptions) -> Result<Self, AgentError> {
        let mut cmd = Command::new(&options.binary);
        cmd.args(&options.args);

        if options.sandbox {
            cmd.arg("--sandbox");
        }

        if let Some(ref cwd) = options.cwd {
            cmd.current_dir(cwd);
        }

        for (k, v) in &options.envs {
            cmd.env(k, v);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            AgentError::ProcessSpawnFailed(format!(
                "{}: {e}",
                options.binary.display()
            ))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            AgentError::IoError("failed to capture child stdin".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            AgentError::IoError("failed to capture child stdout".to_string())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            AgentError::IoError("failed to capture child stderr".to_string())
        })?;

        let (writer_tx, writer_rx) = bounded::<String>(DEFAULT_CHANNEL_CAPACITY);
        let (event_tx, event_rx) =
            bounded::<SupervisorEvent>(DEFAULT_CHANNEL_CAPACITY);

        let pending_requests: PendingRequests = Arc::new(Mutex::new(HashMap::new()));
        let stderr_buffer: StderrBuffer =
            Arc::new(Mutex::new(VecDeque::with_capacity(MAX_STDERR_LINES)));
        let is_running = Arc::new(AtomicBool::new(true));

        // Writer Thread
        let writer_handle = thread::Builder::new()
            .name("antares-agent-writer".to_string())
            .spawn(move || {
                let mut writer = stdin;
                while let Ok(line) = writer_rx.recv() {
                    if writer.write_all(line.as_bytes()).is_err()
                        || writer.flush().is_err()
                    {
                        break;
                    }
                }
            })
            .map_err(|e| {
                AgentError::ProcessSpawnFailed(format!(
                    "failed to spawn writer thread: {e}"
                ))
            })?;

        // Stdout Reader Thread
        let pending_clone = pending_requests.clone();
        let event_tx_clone = event_tx.clone();
        let running_clone = is_running.clone();
        let reader_handle = thread::Builder::new()
            .name("antares-agent-stdout".to_string())
            .spawn(move || {
                let reader = BufReader::new(stdout);
                for line_res in reader.lines() {
                    let line = match line_res {
                        Ok(l) => l,
                        Err(_) => break,
                    };

                    match decode_line(&line) {
                        Ok(JsonRpcMessage::Success(res)) => {
                            if let Some(sender) =
                                pending_clone.lock().remove(&res.id)
                            {
                                let _ = sender.send(Ok(res.result));
                            }
                        }
                        Ok(JsonRpcMessage::Error(err_res)) => {
                            if let Some(sender) =
                                pending_clone.lock().remove(&err_res.id)
                            {
                                let _ = sender.send(Err(AgentError::RpcError {
                                    code: err_res.error.code,
                                    message: err_res.error.message,
                                    data: err_res.error.data,
                                }));
                            }
                        }
                        Ok(JsonRpcMessage::Notification(notif)) => {
                            if notif.method == "session/update" {
                                if let Some(params_val) = notif.params {
                                    if let Ok(params) = serde_json::from_value::<
                                        SessionUpdateParams,
                                    >(
                                        params_val
                                    ) {
                                        let _ = event_tx_clone.send(
                                            SupervisorEvent::SessionUpdate(params),
                                        );
                                        continue;
                                    }
                                }
                            }
                            let _ = event_tx_clone
                                .send(SupervisorEvent::Notification(notif));
                        }
                        Ok(JsonRpcMessage::Request(_)) => {
                            // Inbound requests from the server (e.g. MCP tool execution request)
                        }
                        Err(e) => {
                            // Ignore or log malformed lines without panicking or crashing reader
                            tracing::warn!(
                                "ignoring malformed agent stdout line: {e}"
                            );
                        }
                    }
                }

                running_clone.store(false, Ordering::SeqCst);
                // Drain any pending requests on EOF
                let mut pending = pending_clone.lock();
                for (_, sender) in pending.drain() {
                    let _ = sender.send(Err(AgentError::ProcessExited(None)));
                }
                let _ = event_tx_clone
                    .send(SupervisorEvent::ProcessExited { status: None });
            })
            .map_err(|e| {
                AgentError::ProcessSpawnFailed(format!(
                    "failed to spawn reader thread: {e}"
                ))
            })?;

        // Stderr Reader Thread
        let stderr_buf_clone = stderr_buffer.clone();
        let stderr_handle = thread::Builder::new()
            .name("antares-agent-stderr".to_string())
            .spawn(move || {
                let reader = BufReader::new(stderr);
                for line_res in reader.lines() {
                    let line = match line_res {
                        Ok(l) => l,
                        Err(_) => break,
                    };
                    let mut buf = stderr_buf_clone.lock();
                    if buf.len() >= MAX_STDERR_LINES {
                        buf.pop_front();
                    }
                    buf.push_back(line);
                }
            })
            .map_err(|e| {
                AgentError::ProcessSpawnFailed(format!(
                    "failed to spawn stderr thread: {e}"
                ))
            })?;

        Ok(Self {
            child: Arc::new(Mutex::new(Some(child))),
            request_counter: AtomicI64::new(1),
            writer_tx,
            pending_requests,
            stderr_buffer,
            event_rx,
            is_running,
            _reader_handle: reader_handle,
            _stderr_handle: stderr_handle,
            _writer_handle: writer_handle,
        })
    }

    /// Check if the child process is still running.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Send a request and wait synchronously for the response with a timeout.
    pub fn send_request(
        &self,
        method: impl Into<String>,
        params: Option<Value>,
        timeout: Duration,
    ) -> Result<Value, AgentError> {
        if !self.is_running() {
            return Err(AgentError::ProcessExited(self.exit_code()));
        }

        let id =
            RequestId::Number(self.request_counter.fetch_add(1, Ordering::SeqCst));
        let request = JsonRpcRequest::new(id.clone(), method, params);
        let encoded = encode_line(&request)?;

        let (resp_tx, resp_rx) = bounded::<Result<Value, AgentError>>(1);
        self.pending_requests.lock().insert(id.clone(), resp_tx);

        if self.writer_tx.send(encoded).is_err() {
            self.pending_requests.lock().remove(&id);
            return Err(AgentError::ChannelClosed);
        }

        match resp_rx.recv_timeout(timeout) {
            Ok(res) => res,
            Err(_) => {
                self.pending_requests.lock().remove(&id);
                Err(AgentError::Timeout(format!(
                    "request timed out after {timeout:?}"
                )))
            }
        }
    }

    /// Send a notification (no response expected).
    pub fn send_notification(
        &self,
        method: impl Into<String>,
        params: Option<Value>,
    ) -> Result<(), AgentError> {
        if !self.is_running() {
            return Err(AgentError::ProcessExited(self.exit_code()));
        }

        let notif = JsonRpcNotification::new(method, params);
        let encoded = encode_line(&notif)?;
        self.writer_tx
            .send(encoded)
            .map_err(|_| AgentError::ChannelClosed)
    }

    /// Receive the next asynchronous event from the agent.
    pub fn recv_event(&self) -> Result<SupervisorEvent, AgentError> {
        self.event_rx.recv().map_err(|_| AgentError::ChannelClosed)
    }

    /// Receive an event with timeout.
    pub fn recv_event_timeout(
        &self,
        timeout: Duration,
    ) -> Result<SupervisorEvent, AgentError> {
        self.event_rx.recv_timeout(timeout).map_err(|_| {
            AgentError::Timeout("no event received in time".to_string())
        })
    }

    /// Retrieve the recent stderr diagnostic output lines.
    pub fn stderr_tail(&self, max_lines: usize) -> Vec<String> {
        let buf = self.stderr_buffer.lock();
        let start = buf.len().saturating_sub(max_lines);
        buf.iter().skip(start).cloned().collect()
    }

    /// Read the child's exit code if it has exited.
    pub fn exit_code(&self) -> Option<i32> {
        let mut child_guard = self.child.lock();
        if let Some(child) = child_guard.as_mut() {
            if let Ok(Some(status)) = child.try_wait() {
                return status.code();
            }
        }
        None
    }

    /// Graceful shutdown: close writer (stdin) and wait for child exit up to timeout.
    pub fn shutdown(&self, timeout: Duration) -> Result<(), AgentError> {
        // Drop writer_tx to close stdin
        let _ = self.writer_tx.send(String::new());

        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Some(_code) = self.exit_code() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(50));
        }

        // Force kill if it refused to exit cleanly
        let mut child_guard = self.child.lock();
        if let Some(child) = child_guard.as_mut() {
            let _ = child.kill();
        }
        Ok(())
    }
}
