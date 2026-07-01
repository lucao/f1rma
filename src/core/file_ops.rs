use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Representa uma operação de arquivo em andamento ou concluída.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperation {
    pub id: uuid::Uuid,
    pub kind: OperationKind,
    pub source: PathBuf,
    pub destination: Option<PathBuf>,
    pub status: OperationStatus,
    pub progress: f32, // 0.0 a 1.0
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationKind {
    Copy,
    Move,
    Delete,
    Rename,
    Compress,
    Extract,
}

impl OperationKind {
    pub fn label(&self) -> &'static str {
        match self {
            OperationKind::Copy => "Copiar",
            OperationKind::Move => "Mover",
            OperationKind::Delete => "Excluir",
            OperationKind::Rename => "Renomear",
            OperationKind::Compress => "Comprimir",
            OperationKind::Extract => "Extrair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl FileOperation {
    pub fn new(kind: OperationKind, source: PathBuf, destination: Option<PathBuf>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            kind,
            source,
            destination,
            status: OperationStatus::Pending,
            progress: 0.0,
            started_at: Utc::now(),
            finished_at: None,
            error: None,
        }
    }

    pub fn mark_in_progress(&mut self) {
        self.status = OperationStatus::InProgress;
    }

    pub fn update_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    pub fn mark_completed(&mut self) {
        self.status = OperationStatus::Completed;
        self.progress = 1.0;
        self.finished_at = Some(Utc::now());
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = OperationStatus::Failed;
        self.finished_at = Some(Utc::now());
        self.error = Some(error);
    }
}

/// Histórico de operações recentes.
#[derive(Debug, Clone, Default)]
pub struct OperationHistory {
    pub operations: Vec<FileOperation>,
    pub max_history: usize,
}

impl OperationHistory {
    pub fn new(max_history: usize) -> Self {
        Self {
            operations: Vec::new(),
            max_history,
        }
    }

    pub fn add(&mut self, op: FileOperation) {
        self.operations.push(op);
        if self.operations.len() > self.max_history {
            self.operations.remove(0);
        }
    }

    pub fn active_operations(&self) -> Vec<&FileOperation> {
        self.operations
            .iter()
            .filter(|op| {
                op.status == OperationStatus::Pending || op.status == OperationStatus::InProgress
            })
            .collect()
    }

    pub fn recent_completed(&self, count: usize) -> Vec<&FileOperation> {
        self.operations
            .iter()
            .filter(|op| op.status == OperationStatus::Completed)
            .rev()
            .take(count)
            .collect()
    }
}
