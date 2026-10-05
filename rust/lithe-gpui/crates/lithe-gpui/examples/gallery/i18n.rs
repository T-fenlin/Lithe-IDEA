//! The gallery's own copy in Chinese.
//!
//! The gallery is a development tool, so its copy does not go through the
//! library's catalogue: the library's keys name intent for an application's
//! strings, while the gallery has thousands of one-off demo sentences that no
//! application will ever reuse. Keying this table on the English source keeps
//! those sentences out of the shipped locales.
//!
//! A sentence missing from the table falls through to English, which is what an
//! untranslated page looks like.

use gpui::SharedString;

/// Whether the gallery is showing Chinese.
pub fn is_chinese() -> bool {
    lithe_gpui::i18n::locale().starts_with("zh")
}

/// The gallery's copy for `english`: Chinese when that is the shown language and
/// the table holds it, `english` otherwise.
pub fn gallery(english: &str) -> SharedString {
    if is_chinese() {
        if let Some(chinese) = lookup(english) {
            return chinese.into();
        }
    }
    english.into()
}

fn lookup(english: &str) -> Option<&'static str> {
    COPY.iter()
        .find_map(|(source, chinese)| (*source == english).then_some(*chinese))
}

/// Page titles and page summaries, keyed by the English the gallery is written in.
const COPY: &[(&str, &str)] = &[
    // ---- page titles (the navigation and each page's heading) ----
    ("Primitives", "基础"),
    ("Typography", "排版"),
    ("Layout", "布局"),
    ("Window & Shell", "窗口与外壳"),
    ("Buttons & Actions", "按钮与操作"),
    ("Forms", "表单"),
    ("Navigation", "导航"),
    ("Menus", "菜单"),
    ("Overlays", "浮层"),
    ("Feedback", "反馈"),
    ("Loading & Motion", "加载与动效"),
    ("Data Display", "数据展示"),
    ("Lists & Trees", "列表与树"),
    ("Tables", "表格"),
    ("Charts", "图表"),
    ("Finance", "金融"),
    ("Editor", "编辑器"),
    ("Terminal", "终端"),
    ("Git", "Git"),
    ("Debug", "调试"),
    ("Documents", "文档"),
    ("Collaboration", "协作"),
    ("AI Chat", "AI 对话"),
    ("Agent", "智能体"),
    ("Generative", "生成式"),
    ("Media", "媒体"),
    ("Files", "文件"),
    ("Messaging", "消息"),
    ("Mail", "邮件"),
    ("Calendar", "日历"),
    ("Project", "项目"),
    ("Canvas & Design", "画布与设计"),
    ("DB & Dev Tools", "数据库与开发工具"),
    ("Dashboard & Monitoring", "仪表盘与监控"),
    ("Settings", "设置"),
    ("Account", "账户"),
    ("Onboarding & Help", "上手与帮助"),
    ("Interaction", "交互"),
    ("Theme", "主题"),
    ("i18n & a11y", "i18n 与无障碍"),
    ("Maps", "地图"),
    ("Misc", "杂项"),
    ("Library Tooling", "库工具"),
    ("Rendering", "渲染"),
    // ---- page summaries ----
    (
        "The smallest parts. Most are gpui itself; Ely adds the rest.",
        "最小的零件。大部分是 gpui 本身，其余由 Ely 补上。",
    ),
    (
        "Inter for words, tabular figures for numbers, one ink for everything.",
        "文字用 Inter，数字用等宽数字，一切共用一种墨色。",
    ),
    (
        "Stacks, grids, splits, docks. Structure that stays out of the way.",
        "堆叠、网格、分栏、停靠。不抢戏的结构。",
    ),
    (
        "Title bars, rails, tabs, and the windows around an app.",
        "标题栏、边栏、标签页，以及应用周围的窗口。",
    ),
    (
        "Seven variants, three sizes, one focus ring, and the verbs around them.",
        "七种变体、三档尺寸、一圈焦点环，以及围绕它们的各种动作。",
    ),
    (
        "Text, choices, dates, colors, files: every way to say what you mean.",
        "文本、选项、日期、颜色、文件：表达意图的每一种方式。",
    ),
    (
        "Tabs, paths, places and palettes: where you are, and the ways out.",
        "标签页、路径、位置与面板：你在哪，以及怎么出去。",
    ),
    (
        "Rows of actions that open from a button, a right click or an arrow, and nest.",
        "一行行动作，从按钮、右键或箭头展开，并可嵌套。",
    ),
    (
        "Panels over the page: anchored to a control, over a scrim, or over everything.",
        "盖在页面上的面板：挂在控件旁、盖在遮罩上，或盖住一切。",
    ),
    (
        "What the app tells you: toasts, notifications, alerts and the states a view can be in.",
        "应用告诉你的东西：气泡、通知、提示，以及视图可能处于的各种状态。",
    ),
    (
        "Waiting, shown honestly, and the motion everything else moves with.",
        "老实的等待提示，以及其余一切动效所依据的运动。",
    ),
    (
        "People, numbers and records, shown plainly.",
        "人、数字与记录，平实地呈现。",
    ),
    (
        "Rows to read, pick, reorder and swipe; lists long enough to need care.",
        "可阅读、选择、重排与滑动的行；长到需要认真对待的列表。",
    ),
    (
        "Rows and columns that sort, filter, page and select, and stay quick at a hundred thousand rows.",
        "可排序、筛选、分页与选择的行列，十万行依然跟得上。",
    ),
    (
        "Lines, bars, parts of a whole, spreads, flows and schedules on quiet axes, with legends, tooltips, zoom and export.",
        "安静的坐标轴上的折线、柱、占比、分布、流与日程，配图例、提示、缩放与导出。",
    ),
    (
        "Market charts with the studies traders read, books, quotes, order entry and portfolios.",
        "行情图与交易者常看的研究指标、订单簿、报价、下单与持仓。",
    ),
    (
        "A code editor with the gutter, folds and cursors it needs, the help a language server gives, search, the panels around it and its status row.",
        "一个代码编辑器：行号槽、折叠与光标，语言服务器给的提示，搜索，周围的面板与状态行。",
    ),
    (
        "A real shell drawn cell by cell, with tabs, splits, find and links; command blocks and history; colored output, logs and processes.",
        "一个逐格绘制的真实 shell：标签页、分屏、查找与链接；命令块与历史；彩色输出、日志与进程。",
    ),
    (
        "Diffs and merges, changes and commits, the commit graph, branches, blame and review.",
        "差异与合并、改动与提交、提交图、分支、追责与评审。",
    ),
    (
        "A paused program: its breakpoints, stack, threads and values, memory and machine code; profiles and network.",
        "暂停中的程序：断点、调用栈、线程与变量、内存与机器码；性能剖析与网络。",
    ),
    (
        "Markdown written and read, blocks like a notebook's, and a knowledge base around them.",
        "Markdown 的写作与阅读、类似 notebook 的块，以及围绕它们的知识库。",
    ),
    (
        "People in the same document: who is here and where, comments and reactions, suggested changes, and sharing.",
        "同一份文档里的人：谁在场、在哪，评论与反应、建议修改，以及分享。",
    ),
    (
        "A conversation with an assistant: messages that stream in, code and media inside them, and what can be done with each.",
        "与助手的对话：流式涌入的消息、消息里的代码与媒体，以及每条消息能做什么。",
    ),
    (
        "An assistant at work: the tools it calls, what it asks before it acts, how far it has come, what it would change, what it sees and what it makes.",
        "工作中的助手：它调用了哪些工具、动手前问了什么、进行到哪、打算改什么、看到了什么、做出了什么。",
    ),
    (
        "Making pictures, sound and video with a model: the prompt and its settings, the queue and what comes back, the canvas, the models behind them, and the data and prompts they work from.",
        "用模型生成图像、声音与视频：提示词及其设置、队列与产出、画布、背后的模型，以及它们依据的数据与提示。",
    ),
    (
        "Pictures, video and sound: looking closely, cropping and marking up, playing, and the controls and devices around them.",
        "图像、视频与声音：细看、裁剪与标注、播放，以及周围的控件与设备。",
    ),
    (
        "Files and folders: their icons, rows and tiles, and the listings, searches and transfers around them.",
        "文件与文件夹：图标、行与平铺，以及周围的列表、搜索与传输。",
    ),
    (
        "Channels and people: the lists and heads of chats, the messages in them, the people behind them, and calls.",
        "频道与人：会话列表与头像、其中的消息、消息背后的人，以及通话。",
    ),
    (
        "Mailboxes and the messages in them: reading, writing, sorting and putting off.",
        "邮箱与其中的邮件：阅读、撰写、归档与稍后处理。",
    ),
    (
        "Days, weeks, months and years, and what happens in them.",
        "日、周、月、年，以及其间发生的事。",
    ),
    (
        "Tasks and issues, who has them, where they stand, and when they are due.",
        "任务与工单：谁负责、进行到哪、何时到期。",
    ),
    (
        "An endless plane to lay work on, the tools and panels around it, and graphs of nodes.",
        "一块可以铺开工作的无尽平面，周围的工具与面板，以及节点图。",
    ),
    (
        "Connections, schemas and queries, data to read, requests to send, and the machines that run them.",
        "连接、模式与查询，要读的数据、要发的请求，以及运行它们的机器。",
    ),
    (
        "Widgets on a grid you arrange, what is firing and what is up, and the stream of what happens.",
        "你自己排布的网格上的小组件：哪些在触发、哪些在线，以及事件流。",
    ),
    (
        "A settings window: its sections, a search over them, and the controls each one holds.",
        "设置窗口：分节、跨节搜索，以及每节包含的控件。",
    ),
    (
        "Signing in and up, a link by email, a second factor, and the account once inside.",
        "登录与注册、邮件链接、二次验证，以及进去之后的账户页。",
    ),
    (
        "A first run in steps, what is new called out, first things to do, and help where it is needed.",
        "分步的首次运行、新功能提示、该先做的事，以及在需要处给出的帮助。",
    ),
    (
        "Drag and drop, resizing and turning, band selection, presses and gestures, keys, scrolling and focus.",
        "拖放、缩放与旋转、框选、按压与手势、按键、滚动与焦点。",
    ),
    (
        "Tokens every component reads. Light and dark share one grammar.",
        "每个组件都读的设计令牌。明暗两套共用同一套语法。",
    ),
    (
        "Messages, numbers and dates in the reader's locale. A right-to-left locale mirrors its stacks.",
        "按读者语言呈现的文案、数字与日期。从右到左的语言会把排布镜像过来。",
    ),
    (
        "Tiles the host supplies, pins with a popup, a picker's crosshair, routes, GeoJSON, heat, clusters and choropleths. The land is Natural Earth's.",
        "宿主提供的地图瓦片、带弹窗的图钉、取点准星、路线、GeoJSON、热力、聚合与分级设色。地图数据来自 Natural Earth。",
    ),
    (
        "Small tools that stand alone: clocks, a stopwatch, a calculator, a unit converter, a QR code scanner, a captcha, a poll, a survey, a quiz, flashcards, licenses, terms, a cookie banner, data out and in, and web views.",
        "独立的小工具：时钟、秒表、计算器、单位换算、二维码扫描、验证码、投票、问卷、测验、卡片、许可、条款、Cookie 提示、数据导入导出，以及内嵌网页。",
    ),
    (
        "Tools for building with the library: this gallery, a playground that edits a component live, tools that inspect a running window, its boxes, frames, renders and input, and catalogs of its tokens, icons and contrast.",
        "用这个库做东西的工具：本画廊、实时改组件的演练场、检查运行中窗口的盒子/帧/渲染/输入的工具，以及令牌、图标与对比度目录。",
    ),
    (
        "Pixels the host draws at the box's size in device pixels, drawn again for a new size or content.",
        "宿主按盒子尺寸以设备像素绘制的内容；尺寸或内容变了就重画一次。",
    ),
    // ---- the gallery's own chrome ----
    ("Component gallery", "组件画廊"),
    // ---- the Layout page's Dock demo ----
    (
        "Panels dock on any edge, tab together, and float.",
        "面板可停靠到任意边、成组为标签页，也可浮动。",
    ),
    (
        "Center. Drag a tab onto a zone.",
        "中间区。把标签拖到落点区。",
    ),
    ("Files", "文件"),
    ("Outline", "大纲"),
    ("Search", "搜索"),
    ("Terminal", "终端"),
    ("%{id} panel", "%{id} 面板"),
    (
        "While a tab drags, the drop zones appear and the one below lights.",
        "拖动标签时出现落点区，指针下的那个会高亮。",
    ),
    (
        "Drag a tab across the dock above to see them.",
        "把上面 Dock 里的标签拖一下就能看到。",
    ),
];
