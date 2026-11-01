# Dock JM

Dock para Windows feita com Tauri 2, Rust e TypeScript.

## Comandos

Instalar as dependências:

```bash
npm install
```

Rodar em desenvolvimento:

```bash
npm run tauri dev
```

Gerar o executável em `src-tauri/target/release/dock-jm.exe`:

```bash
npm run tauri build -- --no-bundle
```

## Configuração

Para abrir a tela de personalização, clique com o botão direito na dock, use o ícone na bandeja do sistema ou abra o programa de novo, o que também funciona com a dock desativada.

As mudanças valem na hora e ficam salvas em `%APPDATA%\com.jmatheus.dockjm\dock.json`.

## Estrutura

- `src/main.ts`, `src/styles.css`: dock, efeito de ampliação e cliques.
- `src/settings.ts`, `src/settings.css`, `settings.html`: tela de personalização.
- `src/config.ts`: tipos da configuração e temas prontos.
- `src-tauri/src/lib.rs`: janelas, bandeja e monitoramento do cursor e das janelas.
- `src-tauri/src/win.rs`: chamadas à API do Windows.
- `src-tauri/src/config.rs`: leitura do `dock.json`.
