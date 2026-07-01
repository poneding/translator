<div align="center">

# Translator

<img src="ui/public/app-icon.png" alt="Translator Logo" width="120" height="120">

### Ferramenta de tradução multiplataforma

Selecione texto em qualquer lugar → pressione uma tecla de atalho → traduza instantaneamente

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.81+-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/React-18+-61DAFB.svg)](https://reactjs.org)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/poneding/translator)
<a href="https://linux.do" alt="LINUX DO"><img src="https://shorturl.at/ggSqS" /></a>

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Русский](README.ru.md) | [Português](README.pt.md) | [Italiano](README.it.md) | [العربية](README.ar.md)

</div>

---

## ✨ Recursos

- 🌍 **Tecla de atalho global** — Traduza texto selecionado de qualquer aplicativo instantaneamente
- 🔌 **5 serviços de tradução** — Youdao (有道), DeepL, Google, Bing (Azure), compatível com OpenAI
- 🤖 **Detecção automática de idioma** — Reconhecimento inteligente do idioma de origem
- 🎯 **Tradução na janela principal** — Fixar, histórico, reprodução de áudio, repetir por serviço
- 📋 **Fallback da área de transferência** — Traduza a área de transferência quando a seleção não estiver disponível
- 🔄 **Atualizações integradas** — Canais de lançamento estável/beta
- 🎨 **Modo escuro** — Segue as preferências do sistema
- 🌏 **12 idiomas de interface** — Troca de idioma do aplicativo em tempo real
- 🔐 **Armazenamento seguro** — Chaves API armazenadas no Chaveiro do SO
- ⚡ **Leve** — Binário ~6 MB, memória < 50 MB

## 📸 Capturas de tela

<div align="center">

<table>
<tr>
<td width="50%">

### Modo claro
<img src="docs/screenshots/light.png" alt="Modo claro">

</td>
<td width="50%">

### Modo escuro
<img src="docs/screenshots/dark.png" alt="Modo escuro">

</td>
</tr>
</table>

</div>

## 📥 Instalação

Baixe o instalador para sua plataforma do [GitHub Releases](https://github.com/poneding/translator/releases/latest):

| Plataforma | Arquitetura | Download recomendado |
| --- | --- | --- |
| macOS | Intel / Apple Silicon | `.dmg` |
| Windows | x86_64 | `.msi` ou `.exe` |
| Linux | x86_64 / arm64 | `.AppImage`, `.deb` ou `.rpm` |

Após o download, instale normalmente: no macOS, abra o `.dmg` e arraste o app para Aplicativos (na primeira inicialização, clique com o botão direito → "Abrir" para ignorar o Gatekeeper); no Windows, execute o instalador (se o SmartScreen avisar, clique em "Mais informações" → "Executar mesmo assim"); no Linux, execute o `.AppImage` diretamente ou instale o `.deb` / `.rpm` com o gerenciador de pacotes.

Para compilar a partir do código-fonte, consulte a seção [Início rápido](#-início-rápido) abaixo.

## 🚀 Início rápido

### Pré-requisitos

- **Rust** 1.81+ (`rustup install stable`)
- **Node.js** 20+
- **Dependências de plataforma:**
  - **macOS**: `xcode-select --install`
  - **Windows**: Microsoft C++ Build Tools + WebView2 (pré-instalado no Win10+)
  - **Linux**: 
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev \
                     libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```

### Desenvolvimento

```bash
# Instalar dependências JavaScript
cd ui && npm install && cd ..

# Executar servidor de desenvolvimento (recarga automática ativada)
cargo tauri dev
```

### Compilar versão de lançamento

```bash
cargo tauri build
```

**Local de saída:** `target/release/bundle/`

- **macOS**: `.dmg` + `.app`
- **Windows**: `.msi` + `.exe`
- **Linux**: `.AppImage` + `.deb`

## 📚 Documentação

- 📐 [Documento de design](docs/DESIGN.md) — Visão geral da arquitetura v0.2
- 🏛️ [Diagrama de arquitetura](docs/ARCHITECTURE.svg) — Mapa visual de componentes
- 🛠️ [Guia do desenvolvedor](docs/dev-guide.md) — Convenções de codificação, testes, depuração
- 👤 [Guia do usuário](docs/user-guide.md) — Instruções de configuração, chaves API, personalização de atalhos

## 📂 Estrutura do projeto

```txt
translator/
├── crates/
│   ├── core/         # Lógica de negócios Rust pura + 5 serviços de tradução
│   ├── platform/     # Monitor de seleção multiplataforma (macOS/Win/Linux)
│   └── app/          # Shell Tauri (comandos, bandeja, IPC)
├── ui/               # Frontend React + Vite (janela principal + configurações)
├── ui/src/locales/   # Arquivos de i18n Fluent (12 idiomas de aplicativo)
├── docs/             # Design + guias de usuário/desenvolvedor
└── .github/          # Workflows CI + lançamento
```

## 🤝 Contribuindo

Contribuições são bem-vindas! Por favor, leia nosso [Guia do desenvolvedor](docs/dev-guide.md) antes de enviar PRs.

## 🙏 Agradecimentos

- **[EasyDict](https://github.com/tisfeng/EasyDict)** — Translator é fortemente inspirado pela experiência de resultados de tradução do EasyDict. Vários recursos de dicionário e de múltiplos resultados foram portados da sua implementação em Swift, incluindo a análise do dicionário Youdao V4, o dicionário Bing v7 com consulta `tlookupv3`, o endpoint do Google WebApp com assinatura `tk` e o layout dos cartões de resultado. EasyDict é um excelente aplicativo **apenas para macOS**; o Translator busca levar uma experiência comparável para macOS, **Windows e Linux**. Nossos sinceros agradecimentos ao autor do EasyDict e aos seus contribuidores.

## 📄 Licença

GPL-3.0-only. Consulte [LICENSE](LICENSE) para detalhes.

## ⭐ Histórico de estrelas

[![Star History Chart](https://api.star-history.com/svg?repos=poneding/translator&type=Date)](https://star-history.com/#poneding/translator&Date)

---

<div align="center">

**Construído com ❤️ usando Rust + Tauri 2 + React**

</div>
