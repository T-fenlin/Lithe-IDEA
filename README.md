<div align="center">
  <img src="./gpui/assets/images/logo.png" width="112" alt="Lithe app icon">

  <h1>Lithe</h1>

  <p><strong>A low-memory IntelliJ IDEA alternative for Java development and AI collaboration</strong></p>

  <p>
    <a href="./README.zh-CN.md"><strong>简体中文</strong></a> ·
    <a href="#core-features">Core features</a> ·
    <a href="#product-tour">Product tour</a> ·
    <a href="#download-and-install">Download</a> ·
    <a href="https://1lck.github.io/Lithe-IDEA/">Decision board</a> ·
    <a href="#develop-lithe">Develop Lithe</a>
  </p>

  <p>
    <a href="https://hellogithub.com/repository/1lck/Lithe-IDEA" target="_blank"><img src="https://abroad.hellogithub.com/v1/widgets/recommend.svg?rid=633af067f07d4d16af010b9dc16c0b8d&amp;claim_uid=7NYd4yvRlGtqfgr" alt="Featured | HelloGitHub" style="width: 250px; height: 54px;" width="250" height="54"></a>
    <a href="https://atomgit.com/lithe_IDEA/Lithe-IDEA"><img src="https://atomgit.com/lithe_IDEA/Lithe-IDEA/star/new_badge.svg" alt="AtomGit G-Star" style="width: 250px; height: 54px;" width="250" height="54"></a>
  </p>

  <p>
    <a href="https://github.com/1lck/Lithe-IDEA/releases/latest"><img src="https://img.shields.io/github/v/release/1lck/Lithe-IDEA?style=flat&label=release&logo=github&logoColor=white" alt="Latest release"></a>
    <a href="https://github.com/1lck/Lithe-IDEA/releases"><img src="https://img.shields.io/github/downloads/1lck/Lithe-IDEA/total?style=flat&label=downloads&logo=github&logoColor=white" alt="Total downloads"></a>
    <img src="https://img.shields.io/badge/status-no%20release%20yet-lightgrey?style=flat" alt="No release build yet">
    <img src="https://img.shields.io/badge/memory-300--400%20MB-159957?style=flat" alt="300 to 400 MB baseline memory">
    <a href="./LICENSE"><img src="https://img.shields.io/github/license/1lck/Lithe-IDEA?style=flat&label=license" alt="Apache License 2.0"></a>
  </p>
</div>

## Join the community

Welcome to join the Lithe community, share your experience, ask questions, and follow the latest updates.

<div align="center">
<table>
  <tr>
    <th>QQ group</th>
    <th>WeChat group</th>
  </tr>
  <tr>
    <td align="center"><a href="https://qm.qq.com/cgi-bin/qm/qr?k=&amp;group_code=163027877"><img src="./docs/assets/contact/qq-group-qr.png" width="280" alt="QR code for the Lithe QQ group"></a></td>
    <td align="center"><a href="https://gcnctzuuwe9u.feishu.cn/wiki/HJFbwZ0hZirAPnkWF3xcPWkCnid?from=from_copylink"><img src="./docs/assets/contact/wechat-group-qr.png" width="280" alt="QR code for the Lithe WeChat group"></a></td>
  </tr>
</table>
</div>

If you cannot scan the WeChat QR code, [click here to join through Feishu](https://gcnctzuuwe9u.feishu.cn/wiki/HJFbwZ0hZirAPnkWF3xcPWkCnid?from=from_copylink).

## Why Lithe

Codex, Claude Code, and other AI coding tools can now handle much of the implementation work. Developers still need an IDE to understand the generated code, follow symbols, run and debug the project, and review every diff. Keeping a heavyweight development environment open for those tasks can mean several gigabytes of resident memory.

## Meet Lithe

Lithe is a lightweight IntelliJ IDEA alternative built first for Java and Spring Boot developers. It keeps the core workflows for browsing, editing, navigation, search, Maven, run and debug, Git diff review, local history, and databases.

The Lithe application typically uses about **300–400 MB of baseline memory** after opening a regular project. Language servers, terminals, build tools, debuggers, and database helpers start on demand. Actual usage varies with the project and active services.

> **AI writes the code. Lithe helps you understand it, run it, and review it.**

## Repository status

The macOS (SwiftUI/AppKit) and Windows (React/Tauri 2) frontends have been
removed from this repository. They were replaced by a single GPUI Kit host in
`gpui/`, which is under active development and has no release pipeline yet.
Installers on [GitHub Releases](https://github.com/1lck/Lithe-IDEA/releases)
come from the removed source and are kept for reference.

If you are installing a previously published macOS build and Gatekeeper blocks
it, the recovery steps are unchanged: Control-click the app in **Applications**,
or run `sudo xattr -dr com.apple.quarantine /Applications/Lithe.app`. Use those
only for an app whose source you trust.

## Core features

<details>
<summary>Expand core features</summary>

1. Built for Spring Boot projects and Java development.
2. Maven management, breakpoint debugging, and custom run configurations.
3. Git management and side-by-side diff review.
4. Double-Shift search and `Command + Shift + F` project-wide search.
5. Process-free lightweight completion and current-file navigation, with on-demand language servers for richer completion, hover, and semantic navigation.
6. Local snapshot history.
7. Multiple projects open within the app.
8. Multiple files open independently in the same window.
9. AI-generated commit messages with customizable formats.
10. Rich Markdown rendering consistent with Yuque syntax.
11. Local application memory usage monitoring.
12. One-command installation and updates through Homebrew.
13. One-click in-app updates and installation.
14. Automatic project entry-point detection with one-click run support for Spring Boot, Java, Maven, Gradle, npm, Cargo, Go, Python, Make, Docker Compose, Procfile, and shell projects.
15. Per-language service switches so language servers can be enabled or disabled independently to match the machine's resources.
16. Multi-line editor tabs for keeping more files visible in the same workspace.
17. Database connection workspace with multiple database types, connection management, SQL history, table browsing, and database operations.
18. Ongoing bug fixes and user experience improvements.

</details>

## Product tour

<p align="center">
  <img src="./docs/assets/screenshots/search-everywhere.png" width="49%" alt="Double-Shift Search Everywhere">
  <img src="./docs/assets/screenshots/global-search.png" width="49%" alt="Command Shift F project-wide search and replace">
</p>

<p align="center">
  <img src="./docs/assets/screenshots/git-diff-review.png" width="96%" alt="Side-by-side Git diff review">
</p>

<p align="center">
  <img src="./docs/assets/screenshots/ai-provider-import.png" width="49%" alt="Import AI provider settings from local tools">
  <img src="./docs/assets/screenshots/ai-commit-format.png" width="49%" alt="Customize AI commit message formats">
</p>

<p align="center">
  <img src="./docs/assets/screenshots/ai-commit-message.png" width="96%" alt="Generate commit messages with AI">
</p>

<p align="center">
  <img src="./docs/assets/screenshots/markdown-mermaid-preview.png" width="49%" alt="Markdown Mermaid rendering and live preview">
  <img src="./docs/assets/screenshots/markdown-rich-preview.png" width="49%" alt="Rich Markdown rendering and live preview">
</p>

<p align="center">
  <img src="./docs/assets/screenshots/memory-monitor-annotated.png" width="49%" alt="Low memory footprint and in-app memory monitoring">
  <img src="./docs/assets/screenshots/memory-monitor.png" width="49%" alt="Application memory usage details">
</p>

<p align="center">
  <img src="./docs/assets/screenshots/project-auto-detection-run.png" width="96%" alt="Automatic project detection and one-click run configuration">
</p>

<p align="center">
  <img src="./docs/assets/screenshots/language-services-settings.png" width="49%" alt="Per-language service settings">
  <img src="./docs/assets/screenshots/multi-line-editor-tabs.png" width="49%" alt="Multi-line editor tabs">
</p>

<p align="center">
  <img src="./docs/assets/screenshots/database-workspace-overview.png" width="49%" alt="Database connection workspace">
  <img src="./docs/assets/screenshots/database-sql-operation.png" width="49%" alt="Database SQL operation and table structure">
</p>

## Download and install

**There is no downloadable build yet.** The previous macOS and Windows
products were removed from this repository, and the GPUI host that replaces
them has no packaging or release pipeline. Published installers still exist on
[GitHub Releases](https://github.com/1lck/Lithe-IDEA/releases) but they were
built from the removed source.

## Architecture Overview

Lithe is a pure Rust repository with one host: a GPUI Kit application in
`gpui/` that drives the deterministic command surface in `rust/lithe-core`.

```mermaid
flowchart LR
    subgraph Host["gpui/ — GPUI Kit host"]
        App["app — composition root"] --> Workbench["workbench — shell"]
        Workbench --> Editor["editor / explorer / git / terminal"]
        App --> Settings["settings"]
        Editor --> Shared["shared — icons, i18n, core client"]
    end

    subgraph Core["rust/lithe-core"]
        Commands["Deterministic commands, models, validation"]
    end

    Contracts["shared/ — contracts and fixtures"] --> Commands
    Shared -->|"Rust crate, direct link"| Commands
```

The gpui crates depend strictly downward:
`app -> workbench -> {editor, explorer, git, terminal} -> shared`, plus
`app -> settings -> shared`. `rust/lithe-core` stays free of GPUI and of any
UI framework.

<details>
<summary><strong>Develop Lithe</strong></summary>


Build and run the host from the repository root:

```bash
cd gpui
cargo build --bin Lithe
./target/debug/Lithe <workspace-root>
```

`--theme`, `--locale`, and `--open-settings` override settings for a single
launch without writing them back. Set `LITHE_GPUI_SETTINGS_FILE` to point the
settings file at a temporary path during testing.

Before submitting a change, run:

```bash
cargo fmt --manifest-path rust/Cargo.toml -p lithe-core -- --check
cargo test --manifest-path rust/Cargo.toml -p lithe-core
cargo test --manifest-path gpui/Cargo.toml
./scripts/verify-rust-core.sh
./scripts/verify-shared-contracts.sh
node scripts/verify-agent-notes.mjs
node scripts/test-classify-ci-changes.mjs
```

The gpui host has no CI lane yet, so those checks are the gate. Directory
ownership and the required Rust Core comment standard are described in
[Repository ownership and sharing boundaries](./.agents/notes/implemented/architecture/2026-09-13-repository-ownership-and-sharing-boundaries.md).
Include your verification steps and known limitations when submitting a change.

</details>

## Project support

### ❤️ Sponsors

<p align="center">
  <a href="https://www.fastaitoken.com/">
    <img src="./docs/assets/sponsors/fastai-qrcode.png" width="480" alt="FastAI Token community chat group QR code">
  </a>
</p>
<p align="center">
  <a href="https://www.fastaitoken.com/"><strong>FastAI</strong></a><br>
  FastAI provides convenient relay access to a range of leading large language models, making it easier to connect AI capabilities to everyday development workflows. Its support helps Lithe continue improving its AI-assisted experience. Thank you to FastAI for supporting this project! Scan the QR code above to join the FastAI Token community chat group #2 (group ID: 1073874006).
</p>

<a href="mailto:2188718831@qq.com">Want to be featured below?</a>

<table>
  <tr>
    <td width="112" align="center">
      <a href="https://www.fastaitoken.com/">
        <img src="./docs/assets/sponsors/fastai.png" width="64" alt="FastAI">
      </a>
    </td>
    <td>
      <a href="https://www.fastaitoken.com/"><strong>FastAI</strong></a> provides convenient relay access to a range of leading large language models, making it easier to connect AI capabilities to everyday development workflows. Its support helps Lithe continue improving its AI-assisted experience. Thank you to FastAI for supporting this project!
    </td>
  </tr>
  <tr>
    <td width="112" align="center">
      <a href="https://torchai.ai/sign-up?aff=FdkE">
        <img src="./docs/assets/sponsors/torchai.jpg" width="64" alt="TorchAI">
      </a>
    </td>
    <td>
      <a href="https://torchai.ai/sign-up?aff=FdkE"><strong>TorchAI</strong></a> provides large language model relay services for developers who need convenient API access across coding, content, and automation scenarios. Its support helps sustain Lithe's development and exploration of practical AI-powered tools. Thank you to TorchAI for supporting this project!
    </td>
  </tr>
  <tr>
    <td width="112" align="center">
      <a href="https://www.atlascloud.ai/zh">
        <img src="./docs/assets/sponsors/atlas-cloud.png" width="112" alt="Atlas Cloud">
      </a>
    </td>
    <td>
      <a href="https://www.atlascloud.ai/zh"><strong>Atlas Cloud</strong></a> provides AI model relay services with access to <strong>over 400 models</strong>. Its broad model selection gives developers more options to explore, compare, and choose models for their applications through one platform. Thank you to Atlas Cloud for supporting Lithe and the continued improvement of its AI-assisted development experience!
    </td>
  </tr>
  <tr>
    <td width="112" align="center">
      <a href="https://codezsy.com/register?aff=fDyA">
        <img src="https://raw.githubusercontent.com/zwenooo/z/main/logo/codez-logo-bq.png" width="64" alt="CodeZ relay service">
      </a>
    </td>
    <td>
      <a href="https://codezsy.com/register?aff=fDyA"><strong>CodeZ</strong></a> focuses on stable relay access to GPT-family models, offering developers a straightforward way to integrate model APIs into their tools and projects. Its sponsorship contributes to the ongoing development and maintenance of Lithe. Thank you to CodeZ for supporting this project!
    </td>
  </tr>
  <tr>
    <td width="112" align="center">
      <a href="https://app.rainyun.com/apps/rcs/392785/detail?view=info">
        <img src="./docs/assets/sponsors/rainyun.png" width="112" alt="RainYun (雨云)">
      </a>
    </td>
    <td>
      <a href="https://app.rainyun.com/apps/rcs/392785/detail?view=info"><strong>RainYun (雨云)</strong></a> offers cloud servers for deploying applications, hosting websites, and running development and test environments. Thank you to RainYun for supporting Lithe and its continued development!
    </td>
  </tr>
</table>

### ⭐ Special thanks

<p align="center">
  <a href="https://linux.do/">
    <img src="./docs/assets/special-thanks/linux-do.png" width="78%" alt="LINUX DO">
  </a>
</p>

<p align="center">
  <strong>For all things AI, head to <a href="https://linux.do/">LINUX DO</a>. Wishing the community ever greater success.</strong>
</p>

### Contributors

Thank you to everyone who contributes to and improves Lithe.

<a href="https://github.com/1lck/Lithe-IDEA/graphs/contributors">
  <img src="https://raw.githubusercontent.com/1lck/Lithe-IDEA/chart-assets/contributors.svg" alt="Contributors">
</a>

### License

Lithe is licensed under the [Apache License 2.0](./LICENSE).

## Star History

<a href="https://www.star-history.com/#1lck/Lithe-IDEA&Date">
  <img alt="Star History Chart" src="https://raw.githubusercontent.com/1lck/Lithe-IDEA/chart-assets/star-history-light.svg" />
</a>
