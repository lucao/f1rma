use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Perfis disponíveis no sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Profile {
    Pessoal,
    Profissional,
    Dev,
}

/// Filtro de perfis ativos. Quando vazio, todos os arquivos são visíveis.
#[derive(Debug, Clone, Default)]
pub struct ProfileFilter {
    pub active: HashSet<Profile>,
}

impl ProfileFilter {
    /// Cria um filtro sem nenhum perfil selecionado (mostra tudo).
    pub fn none() -> Self {
        Self { active: HashSet::new() }
    }

    /// Cria um filtro com um único perfil.
    pub fn single(profile: Profile) -> Self {
        let mut active = HashSet::new();
        active.insert(profile);
        Self { active }
    }

    /// Verifica se um perfil está ativo no filtro.
    pub fn is_active(&self, profile: Profile) -> bool {
        self.active.contains(&profile)
    }

    /// Ativa ou desativa um perfil no filtro (toggle).
    pub fn toggle(&mut self, profile: Profile) {
        if self.active.contains(&profile) {
            self.active.remove(&profile);
        } else {
            self.active.insert(profile);
        }
    }

    /// Se o filtro está vazio (sem perfis selecionados = mostrar tudo).
    pub fn show_all(&self) -> bool {
        self.active.is_empty()
    }

    /// Limpa o filtro (mostra tudo).
    pub fn clear(&mut self) {
        self.active.clear();
    }
}

impl Profile {
    pub fn label(&self) -> &'static str {
        match self {
            Profile::Pessoal => "Pessoal",
            Profile::Profissional => "Profissional",
            Profile::Dev => "Dev",
        }
    }

    pub fn all() -> &'static [Profile] {
        &[Profile::Pessoal, Profile::Profissional, Profile::Dev]
    }

    /// Determina se arquivos deste perfil devem estar disponíveis na rede.
    pub fn is_network_shared(&self) -> bool {
        match self {
            Profile::Pessoal | Profile::Profissional => true,
            Profile::Dev => false,
        }
    }
}

/// Armazena o mapeamento de perfis para diretórios e arquivos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    /// Mapeamento de caminhos absolutos para perfis definidos explicitamente.
    pub assignments: HashMap<PathBuf, Profile>,
    /// Diretório seguro/criptografado que NÃO é compartilhado na rede.
    pub secure_directories: Vec<PathBuf>,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            assignments: HashMap::new(),
            secure_directories: Vec::new(),
        }
    }
}

impl ProfileConfig {
    /// Resolve o perfil de um caminho com base na herança de diretórios.
    /// Percorre o caminho de baixo para cima até encontrar um perfil definido.
    pub fn resolve_profile(&self, path: &Path) -> Option<Profile> {
        // Verifica se o próprio arquivo/diretório tem perfil explícito
        if let Some(profile) = self.assignments.get(path) {
            return Some(*profile);
        }

        // Percorre os ancestrais para herdar o perfil
        let mut current = path.parent();
        while let Some(parent) = current {
            if let Some(profile) = self.assignments.get(parent) {
                return Some(*profile);
            }
            current = parent.parent();
        }

        None
    }

    /// Define o perfil de um caminho.
    pub fn assign_profile(&mut self, path: PathBuf, profile: Profile) {
        self.assignments.insert(path, profile);
    }

    /// Remove a atribuição de perfil de um caminho.
    pub fn remove_profile(&mut self, path: &Path) {
        self.assignments.remove(path);
    }

    /// Verifica se um caminho está dentro de um diretório seguro.
    pub fn is_in_secure_directory(&self, path: &Path) -> bool {
        self.secure_directories
            .iter()
            .any(|secure| path.starts_with(secure))
    }

    /// Determina se um arquivo deve ser visível dado o perfil ativo.
    /// Arquivos sem perfil são sempre visíveis. Arquivos de outro perfil ficam ocultos.
    pub fn is_visible(&self, path: &Path, active_profile: Profile) -> bool {
        match self.resolve_profile(path) {
            Some(profile) => profile == active_profile,
            None => true, // Sem perfil definido = sempre visível
        }
    }

    /// Determina visibilidade com filtro multi-perfil.
    /// Se o filtro está vazio (show_all), mostra tudo.
    /// Se há perfis selecionados, mostra apenas arquivos desses perfis + sem perfil.
    pub fn is_visible_filtered(&self, path: &Path, filter: &ProfileFilter) -> bool {
        if filter.show_all() {
            return true;
        }
        match self.resolve_profile(path) {
            Some(profile) => filter.is_active(profile),
            None => true, // Sem perfil definido = sempre visível
        }
    }

    /// Verifica se um arquivo deve estar disponível na rede.
    pub fn is_network_available(&self, path: &Path) -> bool {
        if self.is_in_secure_directory(path) {
            return false;
        }

        match self.resolve_profile(path) {
            Some(profile) => profile.is_network_shared(),
            None => false, // Sem perfil = não compartilha por padrão
        }
    }

    /// Retorna true se o caminho não tem perfil definido (nem herdado).
    pub fn needs_profile_assignment(&self, path: &Path) -> bool {
        self.resolve_profile(path).is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_inheritance() {
        let mut config = ProfileConfig::default();
        let docs = PathBuf::from("/home/user/docs");
        config.assign_profile(docs.clone(), Profile::Pessoal);

        let file = PathBuf::from("/home/user/docs/notas/arquivo.txt");
        assert_eq!(config.resolve_profile(&file), Some(Profile::Pessoal));
    }

    #[test]
    fn test_visibility() {
        let mut config = ProfileConfig::default();
        let docs = PathBuf::from("/home/user/docs");
        config.assign_profile(docs, Profile::Pessoal);

        let file = PathBuf::from("/home/user/docs/notas.txt");
        assert!(config.is_visible(&file, Profile::Pessoal));
        assert!(!config.is_visible(&file, Profile::Dev));
    }

    #[test]
    fn test_secure_directory_not_shared() {
        let mut config = ProfileConfig::default();
        let secure = PathBuf::from("/home/user/docs/.seguro");
        config.secure_directories.push(secure.clone());

        let docs = PathBuf::from("/home/user/docs");
        config.assign_profile(docs, Profile::Pessoal);

        let file = PathBuf::from("/home/user/docs/.seguro/senha.txt");
        assert!(!config.is_network_available(&file));
    }
}
