use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Perfis disponíveis no sistema.
/// Os 3 primeiros são padrões fixos. `Custom` permite perfis criados pelo usuário.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Profile {
    Pessoal,
    Profissional,
    Dev,
    Custom(String),
}

impl Profile {
    pub fn label(&self) -> &str {
        match self {
            Profile::Pessoal => "Pessoal",
            Profile::Profissional => "Profissional",
            Profile::Dev => "Dev",
            Profile::Custom(name) => name.as_str(),
        }
    }

    /// Retorna os perfis padrão (fixos).
    pub fn defaults() -> Vec<Profile> {
        vec![Profile::Pessoal, Profile::Profissional, Profile::Dev]
    }

    /// Determina se arquivos deste perfil devem estar disponíveis na rede.
    /// Perfis customizados compartilham por padrão (como Pessoal/Profissional).
    pub fn is_network_shared(&self) -> bool {
        match self {
            Profile::Dev => false,
            _ => true,
        }
    }
}

/// Registro de perfis customizados criados pelo usuário.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileRegistry {
    /// Nomes dos perfis customizados.
    pub custom_profiles: Vec<String>,
    /// Perfis que NÃO compartilham na rede (como Dev).
    pub no_share_profiles: HashSet<String>,
}

impl ProfileRegistry {
    /// Retorna todos os perfis disponíveis (padrão + custom).
    pub fn all_profiles(&self) -> Vec<Profile> {
        let mut profiles = Profile::defaults();
        for name in &self.custom_profiles {
            profiles.push(Profile::Custom(name.clone()));
        }
        profiles
    }

    /// Adiciona um perfil customizado.
    pub fn add_custom(&mut self, name: String) {
        if !self.custom_profiles.contains(&name) {
            self.custom_profiles.push(name);
        }
    }

    /// Remove um perfil customizado.
    pub fn remove_custom(&mut self, name: &str) {
        self.custom_profiles.retain(|n| n != name);
        self.no_share_profiles.remove(name);
    }

    /// Define se um perfil customizado compartilha na rede.
    pub fn set_network_shared(&mut self, name: &str, shared: bool) {
        if shared {
            self.no_share_profiles.remove(name);
        } else {
            self.no_share_profiles.insert(name.to_string());
        }
    }

    /// Verifica se um perfil compartilha na rede (considerando custom overrides).
    pub fn is_profile_network_shared(&self, profile: &Profile) -> bool {
        match profile {
            Profile::Dev => false,
            Profile::Custom(name) => !self.no_share_profiles.contains(name),
            _ => true,
        }
    }
}

/// Filtro de perfis ativos. Quando vazio, todos os arquivos são visíveis.
#[derive(Debug, Clone, Default)]
pub struct ProfileFilter {
    pub active: HashSet<Profile>,
}

impl ProfileFilter {
    pub fn none() -> Self {
        Self { active: HashSet::new() }
    }

    pub fn is_active(&self, profile: &Profile) -> bool {
        self.active.contains(profile)
    }

    pub fn toggle(&mut self, profile: Profile) {
        if self.active.contains(&profile) {
            self.active.remove(&profile);
        } else {
            self.active.insert(profile);
        }
    }

    pub fn show_all(&self) -> bool {
        self.active.is_empty()
    }

    pub fn clear(&mut self) {
        self.active.clear();
    }
}

/// Armazena o mapeamento de perfis para diretórios e arquivos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    /// Mapeamento de caminhos para lista de perfis atribuídos.
    pub assignments: HashMap<PathBuf, Vec<Profile>>,
    pub secure_directories: Vec<PathBuf>,
    #[serde(default)]
    pub registry: ProfileRegistry,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            assignments: HashMap::new(),
            secure_directories: Vec::new(),
            registry: ProfileRegistry::default(),
        }
    }
}

impl ProfileConfig {
    /// Resolve os perfis de um caminho com herança de diretórios.
    pub fn resolve_profiles(&self, path: &Path) -> Vec<&Profile> {
        if let Some(profiles) = self.assignments.get(path) {
            return profiles.iter().collect();
        }
        let mut current = path.parent();
        while let Some(parent) = current {
            if let Some(profiles) = self.assignments.get(parent) {
                return profiles.iter().collect();
            }
            current = parent.parent();
        }
        Vec::new()
    }

    /// Adiciona um perfil a um caminho (não duplica).
    pub fn assign_profile(&mut self, path: PathBuf, profile: Profile) {
        let profiles = self.assignments.entry(path).or_default();
        if !profiles.contains(&profile) {
            profiles.push(profile);
        }
    }

    /// Remove um perfil específico de um caminho.
    pub fn unassign_profile(&mut self, path: &Path, profile: &Profile) {
        if let Some(profiles) = self.assignments.get_mut(path) {
            profiles.retain(|p| p != profile);
            if profiles.is_empty() {
                self.assignments.remove(path);
            }
        }
    }

    /// Remove todos os perfis de um caminho.
    pub fn remove_profile(&mut self, path: &Path) {
        self.assignments.remove(path);
    }

    pub fn is_in_secure_directory(&self, path: &Path) -> bool {
        self.secure_directories.iter().any(|secure| path.starts_with(secure))
    }

    /// Visibilidade com filtro multi-perfil.
    /// Visível se: filtro vazio (show_all), sem perfil atribuído, ou qualquer
    /// perfil do arquivo está no filtro.
    pub fn is_visible_filtered(&self, path: &Path, filter: &ProfileFilter) -> bool {
        if filter.show_all() {
            return true;
        }
        let profiles = self.resolve_profiles(path);
        if profiles.is_empty() {
            return true; // Sem perfil = sempre visível
        }
        // Visível se pelo menos um dos perfis do arquivo está ativo no filtro
        profiles.iter().any(|p| filter.is_active(p))
    }

    /// Verifica se um arquivo deve estar disponível na rede.
    pub fn is_network_available(&self, path: &Path) -> bool {
        if self.is_in_secure_directory(path) {
            return false;
        }
        let profiles = self.resolve_profiles(path);
        if profiles.is_empty() {
            return false;
        }
        // Disponível se pelo menos um dos perfis compartilha na rede
        profiles.iter().any(|p| self.registry.is_profile_network_shared(p))
    }

    pub fn needs_profile_assignment(&self, path: &Path) -> bool {
        self.resolve_profiles(path).is_empty()
    }

    /// Todos os perfis disponíveis (padrão + custom).
    pub fn all_profiles(&self) -> Vec<Profile> {
        self.registry.all_profiles()
    }

    /// Verifica se um caminho tem um perfil específico atribuído.
    pub fn has_profile(&self, path: &Path, profile: &Profile) -> bool {
        self.resolve_profiles(path).contains(&profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_inheritance() {
        let mut config = ProfileConfig::default();
        config.assign_profile(PathBuf::from("/home/user/docs"), Profile::Pessoal);
        let file = PathBuf::from("/home/user/docs/notas/arquivo.txt");
        assert_eq!(config.resolve_profiles(&file), vec![&Profile::Pessoal]);
    }

    #[test]
    fn test_multiple_profiles() {
        let mut config = ProfileConfig::default();
        config.assign_profile(PathBuf::from("/shared"), Profile::Pessoal);
        config.assign_profile(PathBuf::from("/shared"), Profile::Profissional);
        let profiles = config.resolve_profiles(Path::new("/shared/file.txt"));
        assert_eq!(profiles.len(), 2);
        assert!(profiles.contains(&&Profile::Pessoal));
        assert!(profiles.contains(&&Profile::Profissional));
    }

    #[test]
    fn test_visibility_filter() {
        let mut config = ProfileConfig::default();
        config.assign_profile(PathBuf::from("/home/user/docs"), Profile::Pessoal);

        let file = PathBuf::from("/home/user/docs/notas.txt");

        let mut filter = ProfileFilter::none();
        filter.toggle(Profile::Pessoal);
        assert!(config.is_visible_filtered(&file, &filter));

        let mut filter2 = ProfileFilter::none();
        filter2.toggle(Profile::Dev);
        assert!(!config.is_visible_filtered(&file, &filter2));
    }

    #[test]
    fn test_multi_profile_visibility() {
        let mut config = ProfileConfig::default();
        config.assign_profile(PathBuf::from("/shared"), Profile::Pessoal);
        config.assign_profile(PathBuf::from("/shared"), Profile::Dev);

        let file = PathBuf::from("/shared/file.txt");

        // Visible if ANY of the file's profiles is in the filter
        let mut filter = ProfileFilter::none();
        filter.toggle(Profile::Dev);
        assert!(config.is_visible_filtered(&file, &filter));
    }

    #[test]
    fn test_custom_profile() {
        let mut config = ProfileConfig::default();
        config.registry.add_custom("Faculdade".to_string());
        let profiles = config.all_profiles();
        assert_eq!(profiles.len(), 4);
        assert!(profiles.contains(&Profile::Custom("Faculdade".to_string())));
    }
}
