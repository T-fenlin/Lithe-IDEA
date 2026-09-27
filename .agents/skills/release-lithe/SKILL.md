---
name: release-lithe
description: Prepare and validate stable Lithe release notes and publishing workflows. Use when drafting docs/releases files, cutting a stable version, publishing a GitHub Release, or changing stable release automation.
---

# Release Lithe

Apply this Skill after `develop-lithe` for stable releases.

> **当前状态：没有可发布的产物。** 旧前端（macOS 的 `.app`/DMG 与 Windows 的
> NSIS 安装包）连同它们的发布工作流已删除，`gpui/` 宿主**还没有**发布流水线。
> 因此本 Skill 现在只约束两件事：写发布说明的**文风与校验**，以及"恢复发布前必须
> 先补齐什么"。不要照着旧章节去引用已经不存在的安装包、Gatekeeper 提示或
> Homebrew 配方；`docs/releases/v*.md` 里的下载链接属于**历史发布记录**，保留原样。

恢复发布前必须先补齐（缺任何一项都不要发版）：

- gpui 的打包与产物命名（安装包格式、图标、版本号注入）。
- 签名与公证方式，以及"未签名时用户会看到什么"的说明。
- 一个真正的 CI lane 跑 `cargo build --release` 与 `cargo test`，而不是只跑 Core。
- 更新 `docs/releases/TEMPLATE.md`：它当前的下载/升级段落写的是 macOS 与 Windows
  产物，已与仓库不符。

## Prepare the release notes first

- Create `docs/releases/v<version>.md` and commit it before creating the tag or
  manually dispatching the stable release workflow.
- Derive the content from the commits, pull requests, tests, packaging changes,
  and known limitations between the previous stable tag and the target commit.
  Do not invent features, compatibility claims, download assets, or fixes.
- Treat the release notes as a required release artifact. A missing or empty
  file blocks the release; never fall back to GitHub-generated notes.
- Keep the release description short, plain-language, and focused on what users
  can do or notice after updating. Write from the user's point of view: explain
  the problem solved or workflow improved before naming the implementation.
- Include all meaningful user-facing changes, grouping related changes when that
  makes the release easier to scan. Do not impose a fixed number of highlights.
  Each highlight should answer “What does this change mean for me?” in one
  sentence. Avoid internal names and jargon such as
  implementation class names, protocol phases, rendering backends, or test
  terminology. Mention a technical name only when a user must recognize it in
  the UI or follow a setup step, and explain it in plain language.
- Do not publish a raw commit or pull-request list as the release description.

## Use the bilingual structure

Write Simplified Chinese first and English second, separated by `---`. Keep the
two sections equivalent in meaning. Use these literal required headings and
this order:

1. `## 中文`, a short release summary, `### 下载`, and `### 重点更新`.
2. A brief overview sentence under `### 重点更新`, then `#### ✨ 新功能`,
   `#### ⚡ 改进`, and `#### 🛠 修复`; omit categories with no changes.
3. `### 升级说明` and `### 兼容性与已知问题`.
4. A comparison link from the previous stable tag to the new tag.
5. `## English`, a short release summary, `### Downloads`, and `### Highlights`.
6. A matching overview under `### Highlights`, then `#### ✨ New features`,
   `#### ⚡ Improvements`, and `#### 🛠 Fixes`; omit the same empty categories.
7. `### Upgrade instructions` and `### Compatibility and known issues`.
8. The equivalent English comparison link.
9. At the very bottom, `### 🙌 感谢贡献者` followed by `### 🙌 Contributors`, with
   equivalent contributor names or GitHub profile links in both languages.

The download sections must list **exactly the artifacts the release actually
produces**, with versioned URLs that match the packaging workflow. Do not copy
the asset list from an older release: those files are gone. Upgrade
instructions must cover the real installation method for that version, and the
compatibility section must state the real platform requirements, signing status,
and known issues.

## User-facing writing checklist

- Lead with the result: “启动项目更快”“终端输出更流畅”“打开文件不再反复报错”.
- Replace implementation descriptions with the visible effect on editing,
  running, debugging, Git, databases, or updates.
- Keep each bullet to one idea and one sentence; combine related fixes when the
  user impact is the same.
- Use everyday words. If a technical term is unavoidable, add a short
  explanation the first time it appears.
- Do not claim performance numbers, compatibility, security, or fixes unless
  the target commit and release checks verify them.
- If the artifact is unsigned, say so plainly and give the recovery step users
  will actually need. Do not carry over the macOS Gatekeeper paragraph unless
  that platform ships again.

Use [the reusable release template](../../../docs/releases/TEMPLATE.md) as the
formatting source of truth. Copy it to `docs/releases/v<version>.md`, replace
all placeholders, and remove its authoring comment. Previous releases are only
factual references; do not inherit their structure or stale claims.

Keep change bullets as single Markdown source lines with no nested lists. Aim
for roughly 40 Chinese characters or 20 English words when practical; this is
a writing target, not a display-width guarantee. Put the platform first only
when a change is platform-specific. Group by new capability, improvement, or
fix, not by implementation layer. Credit verified contributors together at
the bottom; optional per-item attribution must be verified and stay concise.
Do not copy another project's features, identities, or compatibility claims.
Run `node scripts/validate-stable-release-notes.mjs docs/releases/v<version>.md`
before committing the notes.

## Contributors

- Add a contributor list at the bottom of every stable release description,
  after the English comparison link.
- Build the list from the commits and merged pull requests between the
  previous stable tag and the new tag. Verify names and profile links against
  GitHub before publishing; do not infer identities from an email address.
- Include human contributors who made code, documentation, design, testing, or
  release work relevant to the version. Exclude automation accounts such as
  `github-actions[bot]` unless the release explicitly needs to credit them.
- Keep the list short and readable. Do not include a raw commit log or every
  incidental merge author.
- Use the same people and links in the Chinese and English sections. If no
  human contribution can be verified for a release, write a brief equivalent
  sentence instead of leaving the section empty.

## Validate before publishing

- Confirm the version uses `MAJOR.MINOR.PATCH`, the filename is exactly
  `docs/releases/v<version>.md`, and the file is non-empty.
- Check that Chinese and English describe the same release and that every
  download filename matches an artifact the release workflow actually produces.
- Check the comparison range, current platform requirements, bundled tools,
  signing status, updater availability, and known issues against the target
  commit and release configuration.
- Run `actionlint` after changing a GitHub Actions workflow. Run any focused
  packaging or manifest checks required by `develop-lithe` for other release
  changes.

Creating tags, pushing commits, dispatching workflows, or editing a live GitHub
Release changes external state. Perform those actions only when the user has
explicitly requested them. Preparing and validating the local release artifact
does not authorize publication.
