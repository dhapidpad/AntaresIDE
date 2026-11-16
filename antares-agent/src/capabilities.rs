//! Agent capabilities, specialist role catalog, provider configuration,
//! token budget tracking, and background task management for Antares IDE.

use serde::{Deserialize, Serialize};

/// Specialist role profile for Octomind agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRole {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt_prefix: String,
    pub allowed_tools: Vec<String>,
}

impl AgentRole {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        system_prompt_prefix: impl Into<String>,
        allowed_tools: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            system_prompt_prefix: system_prompt_prefix.into(),
            allowed_tools,
        }
    }

    /// Default builtin specialist roles available in Antares IDE.
    pub fn builtin_roles() -> Vec<Self> {
        vec![
            Self::new(
                "coder",
                "Antares Software Engineer",
                "Full-stack software engineering, patch implementation, and refactoring.",
                "You are an expert software engineer in Antares IDE. Propose safe, minimal diffs.",
                vec![
                    "workspace_read_file".into(),
                    "workspace_list_directory".into(),
                    "workspace_propose_patch".into(),
                    "workspace_request_command".into(),
                ],
            ),
            Self::new(
                "reviewer",
                "Antares Security Auditor & Reviewer",
                "Read-only code auditing, security flaw detection, and style verification.",
                concat!(
                    "You are an expert security auditor and code reviewer. ",
                    "Analyze code for flaws without edits."
                ),
                vec![
                    "workspace_read_file".into(),
                    "workspace_list_directory".into(),
                ],
            ),
            Self::new(
                "architect",
                "Antares System Architect",
                "High-level system design, architecture review, and interface specification.",
                "You are a principal system architect guiding modular boundaries and cohesion.",
                vec![
                    "workspace_read_file".into(),
                    "workspace_list_directory".into(),
                    "workspace_propose_patch".into(),
                ],
            ),
        ]
    }
}

/// Supported model provider families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    Octomind,
    Gemini,
    Anthropic,
    OpenAI,
    Custom,
}

/// Provider and model configuration for Antares AI runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub model: String,
    pub endpoint: Option<String>,
    pub context_window_tokens: usize,
    pub active: bool,
}

impl ProviderConfig {
    pub fn octomind_default() -> Self {
        Self {
            kind: ProviderKind::Octomind,
            model: "octomind-v0.72".to_string(),
            endpoint: None,
            context_window_tokens: 128_000,
            active: true,
        }
    }

    pub fn gemini_flash() -> Self {
        Self {
            kind: ProviderKind::Gemini,
            model: "gemini-3.8-flash".to_string(),
            endpoint: None,
            context_window_tokens: 1_000_000,
            active: false,
        }
    }
}

/// Budget and usage tracking for agent session tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenTracker {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub estimated_cost_usd: f64,
    pub max_session_tokens: Option<usize>,
}

impl TokenTracker {
    pub fn new(max_session_tokens: Option<usize>) -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            estimated_cost_usd: 0.0,
            max_session_tokens,
        }
    }

    pub fn record_usage(
        &mut self,
        input: usize,
        output: usize,
        price_per_million: (f64, f64),
    ) {
        self.input_tokens += input;
        self.output_tokens += output;
        let input_cost = (input as f64 / 1_000_000.0) * price_per_million.0;
        let output_cost = (output as f64 / 1_000_000.0) * price_per_million.1;
        self.estimated_cost_usd += input_cost + output_cost;
    }

    pub fn total_tokens(&self) -> usize {
        self.input_tokens + self.output_tokens
    }

    pub fn is_budget_exceeded(&self) -> bool {
        if let Some(limit) = self.max_session_tokens {
            self.total_tokens() >= limit
        } else {
            false
        }
    }
}

/// Lifecycle status for background tasks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed { error: String },
    Cancelled,
}

/// A background task tracked by the Antares IDE supervisor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundTask {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub created_at_ms: u64,
    pub finished_at_ms: Option<u64>,
    pub progress_percent: Option<u8>,
}

/// Supervisor registry for background agent activities.
#[derive(Default)]
pub struct TaskManager {
    tasks: Vec<BackgroundTask>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_task(
        &mut self,
        id: impl Into<String>,
        title: impl Into<String>,
        created_at_ms: u64,
    ) -> &BackgroundTask {
        let task = BackgroundTask {
            id: id.into(),
            title: title.into(),
            status: TaskStatus::Queued,
            created_at_ms,
            finished_at_ms: None,
            progress_percent: Some(0),
        };
        self.tasks.push(task);
        self.tasks.last().unwrap()
    }

    pub fn update_status(
        &mut self,
        id: &str,
        status: TaskStatus,
        progress: Option<u8>,
        now_ms: u64,
    ) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            let is_finished = matches!(
                status,
                TaskStatus::Completed
                    | TaskStatus::Failed { .. }
                    | TaskStatus::Cancelled
            );
            task.status = status;
            task.progress_percent = progress;
            if is_finished {
                task.finished_at_ms = Some(now_ms);
            }
            true
        } else {
            false
        }
    }

    pub fn cancel_task(&mut self, id: &str, now_ms: u64) -> bool {
        self.update_status(id, TaskStatus::Cancelled, None, now_ms)
    }

    pub fn get_task(&self, id: &str) -> Option<&BackgroundTask> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn list_tasks(&self) -> &[BackgroundTask] {
        &self.tasks
    }
}
