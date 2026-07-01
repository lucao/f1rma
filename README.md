# F1RMA - Gerenciador de Arquivos

Gerenciador de arquivos com sistema de perfis, compartilhamento em rede seguro e anotações colaborativas.

## Funcionalidades

### Interface (5 painéis)
- **Cabeçalho**: Seletor de perfil (Pessoal / Profissional / Dev) + caminho atual + info da máquina
- **Painel esquerdo**: Árvore de diretórios + busca por nome
- **Painel central**: Exibição de arquivos (Lista, Ícones, Detalhes, Compacto)
- **Painel direito**: Pré-visualização de arquivo + anotações do usuário
- **Rodapé**: Operações em progresso, barra de transferência, histórico de ações

### Perfis
- **Pessoal** e **Profissional**: Arquivos compartilhados na rede (exceto diretórios seguros/criptografados)
- **Dev**: Arquivos nunca compartilhados na rede
- Herança automática: perfil de uma pasta se propaga para tudo abaixo
- Arquivos sem perfil ficam indicados com ⚠ para rápida atribuição (botão direito)
- Arquivos de outro perfil ficam ocultos quando o perfil está ativo

### Rede
- Compartilhamento HTTP local com autenticação
- Toda modificação remota requer autorização da máquina hospedeira
- Anotações visíveis na rede com identificação do autor e máquina

### Segurança
- Diretórios seguros criptografados com AES-256-GCM
- Arquivos em diretórios seguros nunca são compartilhados na rede

## Executar

```bash
cargo run
```

## Build otimizado

```bash
cargo build --release
```

## Estrutura

```
src/
├── main.rs              # Entry point
├── app.rs               # Estado da aplicação e loop principal
├── core/
│   ├── annotations.rs   # Sistema de anotações
│   ├── crypto.rs        # Criptografia AES-256-GCM
│   ├── file_ops.rs      # Operações de arquivo com progresso
│   ├── profile.rs       # Sistema de perfis e visibilidade
│   └── search.rs        # Busca e árvore de diretórios
├── network/
│   ├── client.rs        # Cliente para acesso a máquinas remotas
│   ├── permissions.rs   # Controle de permissões de rede
│   ├── server.rs        # Servidor HTTP de compartilhamento
│   └── share.rs         # Lógica de compartilhamento
└── ui/
    ├── footer.rs        # Rodapé (operações + progresso)
    ├── header.rs        # Cabeçalho (perfil + navegação)
    ├── main_panel.rs    # Painel central (exibição de arquivos)
    ├── preview_panel.rs # Painel direito (preview + anotações)
    └── tree_panel.rs    # Painel esquerdo (árvore + busca)
```
