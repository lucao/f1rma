use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Extensão do arquivo sidecar de anotações.
const SIDECAR_EXT: &str = ".f1rma";

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

/// Dados do sidecar (o que é persistido no arquivo .f1rma).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SidecarData {
    pub annotations: Vec<Annotation>,
}

/// Gerenciador de anotações com persistência em arquivos sidecar.
/// Cada arquivo/pasta `X` tem suas anotações em `X.f1rma` ao lado dele.
#[derive(Debug, Clone, Default)]
pub struct AnnotationStore {
    /// Cache em memória para evitar leitura de disco a cada frame.
    cache: std::collections::HashMap<PathBuf, Vec<Annotation>>,
}

impl AnnotationStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Retorna o caminho do arquivo sidecar para um dado path.
    fn sidecar_path(path: &Path) -> PathBuf {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(SIDECAR_EXT);
        PathBuf::from(sidecar)
    }

    /// Carrega anotações de um arquivo (do cache ou disco).
    pub fn get_annotations(&mut self, path: &Path) -> &Vec<Annotation> {
        if !self.cache.contains_key(path) {
            let annotations = self.load_from_disk(path);
            self.cache.insert(path.to_path_buf(), annotations);
        }
        self.cache.get(path).unwrap()
    }

    /// Adiciona uma anotação e persiste imediatamente.
    pub fn add_annotation(&mut self, path: PathBuf, annotation: Annotation) {
        if !self.cache.contains_key(&path) {
            let loaded = self.load_from_disk(&path);
            self.cache.insert(path.clone(), loaded);
        }
        self.cache.get_mut(&path).unwrap().push(annotation);
        self.save_to_disk(&path);
    }

    /// Remove uma anotação por ID e persiste.
    pub fn remove_annotation(&mut self, path: &Path, annotation_id: Uuid) -> bool {
        if let Some(annotations) = self.cache.get_mut(path) {
            let len_before = annotations.len();
            annotations.retain(|a| a.id != annotation_id);
            if annotations.len() < len_before {
                self.save_to_disk(path);
                return true;
            }
        }
        false
    }

    /// Atualiza uma anotação e persiste.
    pub fn update_annotation(&mut self, path: &Path, annotation_id: Uuid, new_content: String) -> bool {
        if let Some(annotations) = self.cache.get_mut(path) {
            if let Some(ann) = annotations.iter_mut().find(|a| a.id == annotation_id) {
                ann.update(new_content);
                self.save_to_disk(path);
                return true;
            }
        }
        false
    }

    /// Carrega do disco (arquivo sidecar).
    fn load_from_disk(&self, path: &Path) -> Vec<Annotation> {
        let sidecar = Self::sidecar_path(path);
        if !sidecar.exists() {
            return Vec::new();
        }
        match std::fs::read_to_string(&sidecar) {
            Ok(content) => {
                serde_json::from_str::<SidecarData>(&content)
                    .map(|d| d.annotations)
                    .unwrap_or_default()
            }
            Err(_) => Vec::new(),
        }
    }

    /// Salva no disco (arquivo sidecar).
    fn save_to_disk(&self, path: &Path) {
        let sidecar = Self::sidecar_path(path);
        if let Some(annotations) = self.cache.get(path) {
            if annotations.is_empty() {
                // Remove o sidecar se não há anotações
                let _ = std::fs::remove_file(&sidecar);
                return;
            }
            let data = SidecarData { annotations: annotations.clone() };
            if let Ok(json) = serde_json::to_string_pretty(&data) {
                let _ = std::fs::write(&sidecar, json);
            }
        }
    }

    /// Invalida o cache para um path (forçar releitura do disco).
    pub fn invalidate(&mut self, path: &Path) {
        self.cache.remove(path);
    }

    /// Método de compatibilidade (não faz nada — persistência agora é por arquivo).
    pub fn save(&self, _config_path: &Path) -> std::io::Result<()> {
        Ok(())
    }

    /// Método de compatibilidade (retorna store vazio — dados carregam sob demanda).
    pub fn load(_config_path: &Path) -> std::io::Result<Self> {
        Ok(Self::default())
    }
}
