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
    // ---- 1 Primitives: section notes ----
    (
        "gpui's div() is the box. Ely adds nothing.",
        "gpui 的 div() 就是盒子，Ely 没有加任何东西。",
    ),
    (
        "A string is an element. StyledText adds runs.",
        "字符串本身就是元素，StyledText 再加样式段。",
    ),
    (
        "gpui's svg(). Any path from the asset source, tinted.",
        "gpui 的 svg()。资产源里的任意路径，按主题着色。",
    ),
    (
        "gpui's canvas(). Paint with paths.",
        "gpui 的 canvas()。用路径作画。",
    ),
    (
        "flex_1() takes the slack. No component needed.",
        "flex_1() 吃掉剩余空间，不需要组件。",
    ),
    (
        "deferred(anchored()) escapes clipping and paints on top.",
        "deferred(anchored()) 能逃出裁剪、画在最上层。",
    ),
    (
        "Take impl IntoElement. The caller fills the hole.",
        "接收 impl IntoElement，由调用方填这块空位。",
    ),
    (
        ".when() and .when_some() branch inside a builder.",
        ".when() 与 .when_some() 在 builder 里做分支。",
    ),
    (
        ".children() takes any iterator.",
        ".children() 接受任意迭代器。",
    ),
    (
        "Return several elements; the parent adopts them.",
        "返回多个元素，由父级收养。",
    ),
    (
        "on_mouse_down_out closes what the pointer left.",
        "on_mouse_down_out 关掉指针离开的那个东西。",
    ),
    (
        "on_hover reports enter and leave. .hover() restyles.",
        "on_hover 报告进入与离开；.hover() 只改样式。",
    ),
    (
        "on_key_down on a focused element. Click, then type.",
        "在获得焦点的元素上挂 on_key_down。先点击，再打字。",
    ),
    (
        "write_to_clipboard and read_from_clipboard_async on App; a browser asks before it hands the clipboard over.",
        "App 上的 write_to_clipboard 与 read_from_clipboard_async；浏览器会先询问再交出剪贴板。",
    ),
    (
        "Bundled Lucide or an application's SVG asset, drawn as a themed mask.",
        "内置 Lucide 或应用自己的 SVG 资产，按主题色做成蒙版绘制。",
    ),
    (
        "IconName lists every bundled icon. Hover for its name.",
        "IconName 列出全部内置图标，悬停可看名字。",
    ),
    (
        "Loading pulses. Failure and emptiness show a quiet mark.",
        "加载时脉动；失败与空态给一个安静的记号。",
    ),
    (
        "One hairline. Across, down, or holding a word.",
        "一条发丝线。横着、竖着，或中间夹个词。",
    ),
    (
        "A scrim that fades in, blocks the page, and hears outside clicks.",
        "淡入的遮罩，挡住页面，并监听外面的点击。",
    ),
    ("Text for screen readers only.", "只给读屏器看的文字。"),
    (
        "Any focusable element. Press Tab to walk these.",
        "任何可聚焦的元素。按 Tab 在它们之间走。",
    ),
    (
        "Tab into the box. Tab then circles inside it.",
        "Tab 进盒子，再按 Tab 就在盒内转圈。",
    ),
    (
        "Knows when it is held. Hold half a second for a long press.",
        "知道自己是被按住的。按住半秒即长按。",
    ),
    (
        "Reports bounds when they change. This is also the ResizeObserver.",
        "边界变化时上报，同时也是 ResizeObserver。",
    ),
    (
        "Scroll the box. The marker reports entering and leaving.",
        "滚动这个盒子，标记会报告进入与离开。",
    ),
    (
        "gpui's trigger with Ely's body, or Ely's own trigger for delay and tracking.",
        "gpui 的触发器配 Ely 的内容体，或用 Ely 自己的触发器获得延迟与跟随。",
    ),
    // ---- 2 Typography: section notes ----
    (
        "Digits roll by place; or the value counts up.",
        "数字按位滚动，或从零数上来。",
    ),
    (
        "Reveals by grapheme, so emoji and CJK stay whole.",
        "按字素揭示，emoji 与 CJK 不会被拆碎。",
    ),
    (
        "A tonal ramp, glyph by glyph. No neon.",
        "逐字渐变的色调坡道，不做霓虹灯。",
    ),
    (
        "A slow band of light while something loads.",
        "加载时缓缓扫过的一条光带。",
    ),
    (
        "Six steps, one weight. Size carries the order.",
        "六级字号、一种字重，层级靠大小表达。",
    ),
    (
        "A pair for page and panel headers.",
        "页面与面板标题用的一对。",
    ),
    (
        "Reading text at 14 with loose leading.",
        "14 号正文，行距宽松。",
    ),
    ("Names a control or a value.", "给控件或值起名字。"),
    ("The quiet line beneath.", "下面那行安静的小字。"),
    ("An eyebrow in small capitals.", "小型大写字母的眉题。"),
    ("Monospace on a quiet fill.", "安静底色上的等宽字。"),
    (
        "gpui key syntax. secondary means ⌘ on macOS and Ctrl elsewhere.",
        "gpui 的按键语法；secondary 在 macOS 上是 ⌘，别处是 Ctrl。",
    ),
    ("Sequences read left to right.", "组合键从左往右读。"),
    ("A rule and a softer ink.", "一条分隔线加一种更柔和的墨色。"),
    (
        "Every match of a query, case-insensitive.",
        "查询的每一处命中，不分大小写。",
    ),
    (
        "One line, an ellipsis at the end, in the text style around it, cut by the shaped line at the width its box gets.",
        "单行、末尾省略号，沿用周围文本样式，按盒子宽度由排版线截断。",
    ),
    (
        "gpui's .line_clamp(n): n lines, then a hard cut.",
        "gpui 的 .line_clamp(n)：留 n 行，然后硬切。",
    ),
    (
        "Cuts the middle so both ends stay legible.",
        "截掉中间，两端保持可读。",
    ),
    (
        "Hover shows the whole line, only when it was cut.",
        "悬停显示整行，且只在真的被截断时。",
    ),
    (
        "Blue marks where a click goes. Underline on hover.",
        "蓝色标记可点击处，悬停加下划线。",
    ),
    (
        "Leaves the app. The arrow says so.",
        "会离开本应用，箭头就是这么说的。",
    ),
    (
        "Refreshes every thirty seconds while on screen.",
        "在屏幕上时每三十秒刷新一次。",
    ),
    (
        "strftime patterns in the system zone.",
        "strftime 格式，系统时区。",
    ),
    (
        "Grouped, fixed precision, tabular figures.",
        "千分位分组、固定精度、等宽数字。",
    ),
    (
        "ISO 4217 codes. Yen and won have no minor units.",
        "ISO 4217 代码；日元与韩元没有小数位。",
    ),
    (
        "Signed values carry direction in word and color.",
        "带符号的值，正负同时写在文字与颜色里。",
    ),
    (
        "Decimal units by default, binary on request.",
        "默认十进制单位，可切二进制。",
    ),
    (
        "A clock for media, a compact span for logs.",
        "媒体用时钟，日志用紧凑时长。",
    ),
    ("The noun follows the count.", "名词跟着数量变单复数。"),
    (
        "One click copies. A check confirms, then leaves.",
        "点一下即复制，对勾确认后自动消失。",
    ),
    (
        "Drag to select. ⌘C copies, ⌘A takes it all.",
        "拖动选择；⌘C 复制，⌘A 全选。",
    ),
    (
        "Shortcodes resolve to the system color font.",
        "短代码解析为系统彩色字符。",
    ),
    (
        "A TeX subset: scripts, fractions, roots, Greek.",
        "TeX 的一个子集：上下标、分式、根号与希腊字母。",
    ),
    // ---- 3 Layout: section notes ----
    (
        "h_stack() and v_stack(): flex rows and columns.",
        "h_stack() 与 v_stack()：横向与纵向的 flex 排布。",
    ),
    (
        "gpui's .flex(), with grow and justify.",
        "gpui 的 .flex()，带 grow 与 justify。",
    ),
    (
        "gpui's .grid() and .col_span().",
        "gpui 的 .grid() 与 .col_span()。",
    ),
    (
        "As many equal columns as fit at a minimum width.",
        "按最小宽度能放下多少等宽列就放多少。",
    ),
    (
        "Each item joins the shortest column.",
        "每个元素放进当前最矮的列。",
    ),
    (
        "flex, items_center, justify_center.",
        "flex、items_center、justify_center。",
    ),
    (
        "A centered column capped by a width token.",
        "居中的一列，宽度由宽度令牌封顶。",
    ),
    (
        "Width fills; height follows the ratio.",
        "宽度占满，高度按比例跟随。",
    ),
    (
        "gpui's .p_*() on the rem scale.",
        "gpui 的 .p_*()，按 rem 刻度。",
    ),
    (
        "gpui's .flex_wrap() moves overflow to the next line.",
        "gpui 的 .flex_wrap() 把溢出挪到下一行。",
    ),
    (
        "gpui's .absolute() with edge offsets.",
        "gpui 的 .absolute() 加边缘偏移。",
    ),
    (
        "The first child sets the size; the rest cover it.",
        "第一个子元素定尺寸，其余盖在上面。",
    ),
    (
        "Drag a divider. Panes nest across and down.",
        "拖动分隔条；面板可横向纵向嵌套。",
    ),
    (
        "A hairline with a wide grab. Hover lights it.",
        "发丝线配很宽的抓取区，悬停点亮。",
    ),
    (
        "Height animates to the measured content.",
        "高度以动画过渡到量出的内容高度。",
    ),
    (
        "One open at a time. The chevron turns with it.",
        "一次只开一个，箭头随之转动。",
    ),
    ("Flat: a hairline, no shadow.", "扁平：一条发丝线，无阴影。"),
    (
        "An app region: a quiet header bar and a body.",
        "应用的一个区域：安静的头栏加内容体。",
    ),
    (
        "A page block: title, note, action.",
        "页面块：标题、说明、操作。",
    ),
    (
        "Related fields under one name.",
        "相关的字段归在一个名字下。",
    ),
    ("A bordered region with no fill.", "有边框、无填充的区域。"),
    (
        "A recessed region on the sunken tone.",
        "凹下去的区域，用下陷色。",
    ),
    (
        "Overlay scrollbars appear while scrolling or hovered.",
        "滚动或悬停时浮现的覆盖式滚动条。",
    ),
    (
        "Any ScrollHandle can wear one. Drag the thumb.",
        "任何 ScrollHandle 都能挂一条，滑块可以拖。",
    ),
    (
        "A soft edge wherever more content waits.",
        "哪里还有内容，哪里就有柔和的边缘提示。",
    ),
    (
        "Past half a screen, a button eases back up.",
        "滚过半屏后，一个按钮缓缓回到顶部。",
    ),
    (
        "Each group's header pins until the next pushes it off.",
        "每组的标题钉住，直到下一组把它顶走。",
    ),
    (
        "Folds to an icon rail; the width eases.",
        "收成一条图标栏，宽度平滑过渡。",
    ),
    (
        "Rises from the bottom. Pull the handle down to dismiss.",
        "从底部升起，往下拉把手即可关掉。",
    ),
    (
        "Slides in from an edge over a scrim.",
        "从边上滑入，底下带遮罩。",
    ),
    (
        "Title, note and actions over the body.",
        "内容体之上是标题、说明与操作。",
    ),
    (
        "Title bar, sidebar, content, status bar.",
        "标题栏、侧栏、内容、状态栏。",
    ),
    (
        "A list beside the chosen item. The divider drags.",
        "列表挨着选中项，分隔条可拖。",
    ),
    (
        "Drag the title bar. It stays inside the window.",
        "拖标题栏，不会跑出窗口。",
    ),
    (
        "Split right or down from any pane; close all but one.",
        "任意面板可向右或向下分屏；其余可全部关掉只留一个。",
    ),
    (
        "Layout to versioned JSON and back, all or nothing.",
        "布局与带版本号的 JSON 互相转换，要么全成要么不动。",
    ),
    (
        "Drag to pan. Hold ⌘ or Ctrl and scroll to zoom at the pointer.",
        "拖动平移；按住 ⌘ 或 Ctrl 并滚动，以指针为中心缩放。",
    ),
    // ---- 4 Window & Shell: section notes ----
    (
        "Tools in groups, hairlines between, one action apart.",
        "工具分组排布，组间发丝线，动作之间留出一档。",
    ),
    (
        "Small type at the foot. Items that act show it on hover.",
        "页脚的小字；可点的条目悬停时显示。",
    ),
    (
        "An icon rail. The mark grows in beside the chosen view.",
        "一条图标栏，选中视图旁会浮现记号。",
    ),
    (
        "Window tabs. New ones rise in; drag one onto another to reorder.",
        "窗口标签页，新标签浮现；把一个拖到另一个上即可重排。",
    ),
    (
        "Title centered, system buttons in their place, empty space drags.",
        "标题居中，系统按钮各就各位，空白处可拖动窗口。",
    ),
    (
        "Three manners. Hover the lights for their glyphs.",
        "三种状态；悬停红绿灯可看各自的图标。",
    ),
    (
        "Windows hit-tests it natively, Linux moves by hand, double-click zooms.",
        "Windows 原生命中测试，Linux 手动移动，双击缩放。",
    ),
    (
        "A margin to resize from, a hairline, a shadow, for client-drawn windows.",
        "自绘窗口用的：可拖动调整的边缘、一条发丝线、一层阴影。",
    ),
    (
        "One menu model: the system bar, the Dock, and later the window.",
        "同一份菜单模型：系统菜单栏、Dock，之后还有窗口内菜单。",
    ),
    (
        "The sidebar runs to the window's top; each column brings a header that drags.",
        "侧栏一直顶到窗口上缘，每列带一个可拖动的头。",
    ),
    (
        "Offer, download, restart. One slim bar, three states.",
        "提供更新、下载、重启：一条窄条，三种状态。",
    ),
    (
        "What changed, then one clear action.",
        "先说改了什么，再给一个明确的动作。",
    ),
    (
        "Plain words first. The report waits behind a disclosure.",
        "先说人话，详细报告收在折叠里。",
    ),
    (
        "Quiet while it matters, gone soon after the network returns.",
        "断网期间安静存在，网络恢复后很快消失。",
    ),
    (
        "Scales the whole window through its rem size. Click the number to reset.",
        "通过 rem 尺寸缩放整个窗口，点数字即复位。",
    ),
    (
        "An SF Symbol in the menu bar with a menu. macOS only.",
        "菜单栏里的 SF Symbol 图标加菜单，仅 macOS。",
    ),
    (
        "A count on the app's Dock icon. macOS only.",
        "应用 Dock 图标上的数字角标，仅 macOS。",
    ),
    (
        "Notices from the system's own center.",
        "来自系统通知中心的通知。",
    ),
    (
        "Mark and name rise in; the bar reports progress or sweeps.",
        "记号与名字浮现，进度条报进度或来回扫动。",
    ),
    (
        "Name, version to copy, links, fine print.",
        "名字、可复制的版本号、链接与小字。",
    ),
    (
        "Each new window cascades from the last; the manager knows them by name.",
        "每个新窗口从上一个层叠而出，管理器按名字记住它们。",
    ),
    (
        "Managed windows over a scrim. Arrows or Tab move, Enter goes.",
        "遮罩之上的受管窗口，方向键或 Tab 移动，Enter 进入。",
    ),
    (
        "A search palette in a window above all others.",
        "浮在一切之上的搜索面板窗口。",
    ),
    (
        "Small windows above the rest. Hover for the bar; expand returns to the app.",
        "浮在其余窗口上的小窗，悬停出标题栏，展开即回到应用。",
    ),
    // ---- 5 Buttons & Actions: section notes ----
    (
        "Built from parts in later chapters.",
        "由后面章节的零件拼成。",
    ),
    (
        "Round and raised for the screen's one main action. It rises in.",
        "屏幕唯一主操作的圆形凸起按钮，出现时有入场动画。",
    ),
    (
        "Hold until the fill completes, or click twice within three seconds.",
        "按住直到填满，或三秒内点两次。",
    ),
    (
        "Small, familiar verbs. Each names itself on hover.",
        "小而熟悉的动作，悬停时自报名字。",
    ),
    (
        "Docked along a panel's foot, or floating as a pill.",
        "停靠在面板底部，或浮成一颗胶囊。",
    ),
    (
        "Rises once rows are selected. The count rolls as it changes.",
        "选中若干行后升起，数量变化时滚动更新。",
    ),
    (
        "A few choices from the bottom, and a separate way out.",
        "从底部弹出的几个选择，出口另设。",
    ),
    (
        "Common moves with their keys. Arrows choose, Enter runs.",
        "常见动作带各自键位，方向键选择，Enter 执行。",
    ),
    (
        "Primary is ink. Color is saved for meaning.",
        "主按钮就是墨色，颜色留给有含义的地方。",
    ),
    (
        "Heights follow density. Tab moves the focus ring.",
        "高度随密度，Tab 移动焦点环。",
    ),
    (
        "Leading icon for verbs. Trailing for menus.",
        "动词类放前导图标，菜单类放尾随图标。",
    ),
    ("Square. Ghost by default.", "方形，默认 Ghost。"),
    (
        "The keystroke rides along, quiet, in the platform's spelling.",
        "按键提示安静地跟在旁边，按平台习惯拼写。",
    ),
    ("Buttons joined edge to edge.", "按钮边贴边相连。"),
    (
        "Stays pressed until pressed again.",
        "保持按下状态，再按一次才弹起。",
    ),
    (
        "Several at once for marks; one at a time for alignment.",
        "标记类可多选，对齐类一次一个。",
    ),
    (
        "One of a few. The thumb springs to the choice.",
        "几个里选一个，滑块弹到所选。",
    ),
];
