use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Uma anotação feita por um usuário sobre um arquivo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: Uuid,
    pub content: String,
    pub author: String,
    pub machine_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Annotation {
    pub fn new(content: String, author: String, machine_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            content,
            author,
            machine_id,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update(&mut self, content: String) {
        self.content = content;
        self.updated_at = Utc::now();
    }
}

/// Armazena todas as anotações do sistema, indexadas por caminho de arquivo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnnotationStore {
    /// Mapa de caminho do arquivo -> lista de anotações.
    pub annotations: HashMap<PathBuf, Vec<Annotation>>,
}

impl AnnotationStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adiciona uma anotação para um arquivo.
    pub fn add_annotation(&mut self, path: PathBuf, annotation: Annotation) {
        self.annotations.entry(path).or_default().push(annotation);
    }

    /// Retorna as anotações de um arquivo.
    pub fn get_annotations(&self, path: &Path) -> Option<&Vec<Annotation>> {
        self.annotations.get(path)
    }

    /// Remove uma anotação por ID.
    pub fn remove_annotation(&mut self, path: &Path, annotation_id: Uuid) -> bool {
        if let Some(annotations) = self.annotations.get_mut(path) {
            let len_before = annotations.len();
            annotations.retain(|a| a.id != annotation_id);
            return annotations.len() < len_before;
        }
        false
    }

    /// Atualiza o conteúdo de uma anotação.
    pub fn update_annotation(
        &mut self,
        path: &Path,
        annotation_id: Uuid,
        new_content: String,
    ) -> bool {
        if let Some(annotations) = self.annotations.get_mut(path) {
            if let Some(annotation) = annotations.iter_mut().find(|a| a.id == annotation_id) {
                annotation.update(new_content);
                return true;
            }
        }
        false
    }

    /// Persiste as anotações em disco.
    pub fn save(&self, config_path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(config_path, json)
    }

    /// Carrega as anotações do disco.
    pub fn load(config_path: &Path) -> std::io::Result<Self> {
        if !config_path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(config_path)?;
        let store: Self = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(store)
    }
}
