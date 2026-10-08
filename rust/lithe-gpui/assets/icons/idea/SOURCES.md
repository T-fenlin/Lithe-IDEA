# idea 图标包来源

本目录由 `node rust/lithe-gpui/tools/sync-macos-idea-icons.mjs --apply` 生成，**不要手改**。

## 一、macOS 图标文件夹的副本（主）

`fileTypes/ expui/ general/ nodes/ vcs/ debugger/ terminal/ toolwindows/ testState/ gutter/ maven/ actions/`
逐字节复制自 `macos/Resources/IDEAIcons/**`（176 个 SVG + 2 个附件，含 Apache-2.0 归属文件）。
macOS 侧仍是那份真源，本目录是 gpui 用的副本 —— 两边各自独立，改图标请改 macOS 那份后重跑本工具。

## 二、并集补充：macOS 文件夹没有、但界面仍在用的旧图标（次）

放在 `legacy/` 下，保留旧包的相对路径（去掉 `icons/` 前缀）。它们来自 Windows 前端
`windows/tauri/src/ui/icons/**`（本分支仍在），与 macOS 那份同属 intellij-community expui，
只是这一版更全（macOS 侧缺 `ui/` 那批通用字形与部分文件类型）。
共 122 个：

- `legacy/files/font.svg`
- `legacy/files/font_dark.svg`
- `legacy/files/graphql.svg`
- `legacy/files/graphql_dark.svg`
- `legacy/files/http.svg`
- `legacy/files/jsp.svg`
- `legacy/files/jsp_dark.svg`
- `legacy/files/maven.svg`
- `legacy/files/maven_dark.svg`
- `legacy/files/patch.svg`
- `legacy/files/patch_dark.svg`
- `legacy/files/perl.svg`
- `legacy/files/perl_dark.svg`
- `legacy/files/rst.svg`
- `legacy/files/rst_dark.svg`
- `legacy/files/terraform.svg`
- `legacy/files/terraform_dark.svg`
- `legacy/files/typeScript.svg`
- `legacy/files/typeScript_dark.svg`
- `legacy/files/vue.svg`
- `legacy/files/wsdl.svg`
- `legacy/files/wsdl_dark.svg`
- `legacy/files/xhtml.svg`
- `legacy/files/xhtml_dark.svg`
- `legacy/files/xsd.svg`
- `legacy/files/xsd_dark.svg`
- `legacy/folders/testResourcesRoot.svg`
- `legacy/folders/testResourcesRoot_dark.svg`
- `legacy/folders/testRoot.svg`
- `legacy/folders/testRoot_dark.svg`
- `legacy/ui/add.svg`
- `legacy/ui/addFile.svg`
- `legacy/ui/addFile_dark.svg`
- `legacy/ui/add_dark.svg`
- `legacy/ui/chevronDown.svg`
- `legacy/ui/chevronDown_dark.svg`
- `legacy/ui/chevronLeft.svg`
- `legacy/ui/chevronLeft_dark.svg`
- `legacy/ui/close.svg`
- `legacy/ui/close_dark.svg`
- `legacy/ui/copy.svg`
- `legacy/ui/copy_dark.svg`
- `legacy/ui/cut.svg`
- `legacy/ui/cut_dark.svg`
- `legacy/ui/delete.svg`
- `legacy/ui/delete_dark.svg`
- `legacy/ui/desktop.svg`
- `legacy/ui/desktop_dark.svg`
- `legacy/ui/documentation.svg`
- `legacy/ui/documentation_dark.svg`
- `legacy/ui/down.svg`
- `legacy/ui/down_dark.svg`
- `legacy/ui/download.svg`
- `legacy/ui/download_dark.svg`
- `legacy/ui/errorDialog.svg`
- `legacy/ui/errorDialog_dark.svg`
- `legacy/ui/filter.svg`
- `legacy/ui/filter_dark.svg`
- `legacy/ui/greenCheckmark.svg`
- `legacy/ui/greenCheckmark_dark.svg`
- `legacy/ui/hashtag.svg`
- `legacy/ui/hashtag_dark.svg`
- `legacy/ui/hide.svg`
- `legacy/ui/hide_dark.svg`
- `legacy/ui/history.svg`
- `legacy/ui/history_dark.svg`
- `legacy/ui/informationDialog.svg`
- `legacy/ui/informationDialog_dark.svg`
- `legacy/ui/keyboard.svg`
- `legacy/ui/keyboard_dark.svg`
- `legacy/ui/left.svg`
- `legacy/ui/left_dark.svg`
- `legacy/ui/lightning.svg`
- `legacy/ui/lightning_dark.svg`
- `legacy/ui/listFiles.svg`
- `legacy/ui/listFiles_dark.svg`
- `legacy/ui/locked.svg`
- `legacy/ui/locked_dark.svg`
- `legacy/ui/moreHorizontal.svg`
- `legacy/ui/moreHorizontal_dark.svg`
- `legacy/ui/newFolder.svg`
- `legacy/ui/newFolder_dark.svg`
- `legacy/ui/notifications.svg`
- `legacy/ui/notifications_dark.svg`
- `legacy/ui/palette.svg`
- `legacy/ui/palette_dark.svg`
- `legacy/ui/realIntentionBulb.svg`
- `legacy/ui/realIntentionBulb_dark.svg`
- `legacy/ui/redo.svg`
- `legacy/ui/redo_dark.svg`
- `legacy/ui/remove.svg`
- `legacy/ui/remove_dark.svg`
- `legacy/ui/right.svg`
- `legacy/ui/right_dark.svg`
- `legacy/ui/save.svg`
- `legacy/ui/save_dark.svg`
- `legacy/ui/settings.svg`
- `legacy/ui/settings_dark.svg`
- `legacy/ui/show.svg`
- `legacy/ui/show_dark.svg`
- `legacy/ui/splitHorizontally.svg`
- `legacy/ui/splitHorizontally_dark.svg`
- `legacy/ui/splitVertically.svg`
- `legacy/ui/splitVertically_dark.svg`
- `legacy/ui/successDialog.svg`
- `legacy/ui/successDialog_dark.svg`
- `legacy/ui/toolWindowDataView.svg`
- `legacy/ui/toolWindowDataView_dark.svg`
- `legacy/ui/undo.svg`
- `legacy/ui/undo_dark.svg`
- `legacy/ui/unlocked.svg`
- `legacy/ui/unlocked_dark.svg`
- `legacy/ui/up.svg`
- `legacy/ui/up_dark.svg`
- `legacy/ui/warningDialog.svg`
- `legacy/ui/warningDialog_dark.svg`
- `legacy/ui/web.svg`
- `legacy/ui/web_dark.svg`
- `legacy/ui/zoomIn.svg`
- `legacy/ui/zoomIn_dark.svg`
- `legacy/ui/zoomOut.svg`
- `legacy/ui/zoomOut_dark.svg`

## 三、这次删掉的旧图标

共 57 个：清单（`fileExtensions` / `folders` / 默认值）与宿主代码（`idea::` 常量）都没有引用。

## 四、同时删除的其它图标主题文件夹

`assets/icons/` 下的 `lithe/`、`symbols/`、`material/`、`pierre/` 一并删除：
它们都是从 Windows 前端搬来的副本，现在只保留本目录这一套（来源见上）。
