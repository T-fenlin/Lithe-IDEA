mod common;

use std::path::PathBuf;

use gpui::{App, AppContext as _, TestAppContext, Window, px, size};
use gpui_kit::test::TestWindowExt as _;

struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn new(name: &str, contents: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("lithe-ui-test-{}-{name}", std::process::id()));
        std::fs::write(&path, contents).expect("写临时文件");
        Self { path }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[gpui::test]
fn editor_pane_renders_a_real_file_and_reports_its_indent(cx: &mut TestAppContext) {
    let file = TempFile::new("indent.md", "# 标题\n\n正文\n");

    cx.update(gpui_kit::init);

    let (handle, pane) = common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
        cx.new(|cx| lithe_gpui_editor::EditorPane::new(None, window, cx))
    });

    let path = file.path.clone();
    common::update_content(handle, &pane, cx, |pane, window, cx| {
        pane.open(&path, window, cx);
    })
    .unwrap();

    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
    })
    .unwrap();

    cx.update(|cx| {
        let pane = pane.read(cx);
        assert_eq!(
            pane.tab_size(),
            2,
            "新视图的缩进宽度应当是默认档 2（设置页 tabSize 的默认值）"
        );
        assert!(
            !pane.active_buffer_is_readonly(),
            "刚打开的普通 markdown 文件必须是可写的（只读格的判据取 Buffer::writable）"
        );
        assert_eq!(
            pane.active_buffer_name(),
            "lithe-ui-test-".to_string() + &std::process::id().to_string() + "-indent.md",
            "活动标签应当是刚打开的那个文件"
        );
    });

    common::update_content(handle, &pane, cx, |pane, _window, cx| {
        pane.set_tab_size(8, cx);
    })
    .unwrap();
    cx.update(|cx| {
        assert_eq!(
            pane.read(cx).tab_size(),
            8,
            "set_tab_size 之后访问器必须返回新值（状态栏缩进格读的就是它）"
        );
    });
}

#[gpui::test]
fn editor_body_host_fills_the_pane_below_the_tab_bar(cx: &mut TestAppContext) {
    let file = TempFile::new("host.md", "# 标题\n\n正文\n");

    cx.update(gpui_kit::init);

    let (handle, pane) = common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
        cx.new(|cx| lithe_gpui_editor::EditorPane::new(None, window, cx))
    });

    let path = file.path.clone();
    common::update_content(handle, &pane, cx, |pane, window, cx| {
        pane.open(&path, window, cx);
    })
    .unwrap();

    // 正文容器的尺寸改从 gpui 自己的 debug selector 读（`.debug_selector` 记进
    // `rendered_frame.debug_bounds`）：编辑器整片迁到 `lithe-ui` 之后，上游 gpui-kit 那套
    // `TestSupportExt::test_support()` 观察包装不再属于本 crate。
    let mut window = gpui::VisualTestContext::from_window(handle.into(), cx);
    window.update(|window, cx| {
        window.render_frame(cx);
    });
    let host = window
        .debug_bounds(lithe_gpui_editor::EDITOR_HOST_ELEMENT_ID)
        .expect("编辑器正文容器必须带 debug selector（EDITOR_HOST_ELEMENT_ID），UI 测试就是靠它量高度的");
    println!("EDITOR_HOST bounds={:?} size={:?}", host.origin, host.size);
    assert!(
        host.size.height >= px(100.),
        "编辑器正文容器必须在标签栏下面拿到真实高度（实测 {:?}）：高度为 0 时编辑器什么都不画，用户看到的就是打开文件后一片黑",
        host.size.height
    );
}

#[gpui::test]
fn welcome_view_lists_recent_projects_and_a_row_click_calls_the_opener(cx: &mut TestAppContext) {
    use std::sync::{Arc, Mutex};

    use lithe_gpui_settings::RecentProjects;
    use lithe_gpui_workbench::welcome::{ProjectOpener, WelcomeView, welcome_row_element_id};

    struct RecordingOpener {
        opened: Arc<Mutex<Vec<PathBuf>>>,
    }

    impl ProjectOpener for RecordingOpener {
        fn open_project(&mut self, target: PathBuf, _window: &mut Window, _cx: &mut App) {
            self.opened.lock().expect("记录锁没中毒").push(target);
        }
    }

    cx.update(gpui_kit::init);

    let mut projects = RecentProjects::new();
    projects.record_open(r"D:\proj\alpha", 1_000, None);
    projects.record_open(r"D:\proj\beta", 2_000, None);
    assert_eq!(projects.entries()[0].path, r"D:\proj\beta");

    let opened = Arc::new(Mutex::new(Vec::new()));
    let recorder = RecordingOpener {
        opened: opened.clone(),
    };
    let (handle, welcome) =
        common::open_window(cx, Some(size(px(800.), px(650.))), |window, cx| {
            cx.new(|cx| WelcomeView::with_opener(projects, None, Box::new(recorder), window, cx))
        });

    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);

        let beta = welcome_row_element_id(r"D:\proj\beta");
        let alpha = welcome_row_element_id(r"D:\proj\alpha");
        assert!(
            window.find(beta.clone()).bounds().size.height > px(0.),
            "最近项目 beta 那一行必须真的画出来（高度 > 0）"
        );
        assert!(
            window.find(alpha.clone()).bounds().size.height > px(0.),
            "最近项目 alpha 那一行必须真的画出来"
        );
        assert!(
            window.find(beta.clone()).bounds().top() <= window.find(alpha.clone()).bounds().top(),
            "行顺序必须与数据层一致（最近的项目在前，真源 `filteredProjects` 不重排）"
        );

        window.click(beta, cx);
    })
    .unwrap();

    let recorded = opened.lock().expect("记录锁没中毒").clone();
    assert_eq!(
        recorded,
        vec![PathBuf::from(r"D:\proj\beta")],
        "点第二条（最近的那条）必须把**数据层那条原始路径**交给打开链路"
    );

    cx.update(|cx| {
        let view = welcome.read(cx);
        assert_eq!(view.rows().len(), 2);
        assert_eq!(view.empty_state(), None);
    });
}

#[gpui::test]
fn welcome_window_renders_the_title_bar_above_its_content(cx: &mut TestAppContext) {
    use gpui_kit::component::Theme;
    use lithe_gpui_settings::RecentProjects;
    use lithe_gpui_shared::rem_px;
    use lithe_gpui_workbench::welcome::{
        WELCOME_TITLE_BAR_HEIGHT_SPEC, WelcomeView, welcome_row_element_id,
    };

    cx.update(gpui_kit::init);

    let mut projects = RecentProjects::new();
    projects.record_open(r"D:\proj\alpha", 1_000, None);

    let (handle, _welcome) =
        common::open_window(cx, Some(size(px(800.), px(650.))), |window, cx| {
            cx.new(|cx| WelcomeView::new(projects, None, window, cx))
        });

    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);

        let bar = window.try_find("lithe-welcome-title-bar").expect(
            "欢迎窗口必须自己画一条标题栏 —— gpui 的窗口在 Windows 上没有原生标题栏，\
             少了这一条就没有窗口键（min/max/close）也没有标题栏拖拽区，用户只能靠 Alt+F4 或任务栏代替",
        );
        assert_eq!(
            bar.bounds().size.height,
            rem_px(Theme::global(cx).font_size, WELCOME_TITLE_BAR_HEIGHT_SPEC),
            "标题栏必须占住规格高度 {WELCOME_TITLE_BAR_HEIGHT_SPEC}pt（工作区窗口的标题栏同高）"
        );
        assert_eq!(
            bar.bounds().top(),
            px(0.),
            "标题栏必须贴在窗口最顶上（实测 top={:?}）：它是窗口壳，不是内容列里的一条工具栏",
            bar.bounds().top()
        );

        let row = window.find(welcome_row_element_id(r"D:\proj\alpha"));
        assert!(
            row.bounds().top() >= bar.bounds().bottom(),
            "最近项目列表必须落在标题栏下面（行 top={:?} < 标题栏 bottom={:?}）：\
             说明标题栏真的占了布局高度，而不是浮在内容上",
            row.bounds().top(),
            bar.bounds().bottom()
        );
    })
    .unwrap();
}
