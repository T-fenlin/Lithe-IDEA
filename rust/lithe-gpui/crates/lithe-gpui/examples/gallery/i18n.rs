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
    // ---- 6 Forms: section notes ----
    (
        "On, off, or mixed when a parent's children disagree. The check pops in.",
        "开、关，或父项的子项不一致时的混合态；对勾弹入。",
    ),
    (
        "One Tab stop per group; the arrow keys move the choice.",
        "每组一个 Tab 停留点，方向键移动选择。",
    ),
    (
        "Larger targets that explain a choice. The chosen border turns to ink.",
        "更大的命中区，顺带解释选项；选中项的边框变成墨色。",
    ),
    (
        "Takes effect at once. The thumb slides with a little overshoot.",
        "改动立即生效，滑块带一点回弹地滑过去。",
    ),
    (
        "Saturation and value on the plane, hue and alpha on the rails. Type it as hex, RGB or HSV; drag a letter to scrub.",
        "平面上调饱和度与明度，轨道上调色相与透明度；可用 hex、RGB 或 HSV 输入，拖字母可微调。",
    ),
    (
        "Named patches; see-through colors sit on a checkerboard. The dropper reads the screen.",
        "命名的色板；半透明色放在棋盘格上。取色器可读屏幕。",
    ),
    (
        "Click the bar to add a stop, drag stops to move them, Delete removes the chosen one.",
        "点色条加节点、拖节点移动，Delete 删除所选节点。",
    ),
    (
        "Arrows move a cursor, Page keys turn the month, Enter picks. Today wears a ring.",
        "方向键移动光标，Page 翻月，Enter 选中；今天带一圈描边。",
    ),
    (
        "Fields that open a calendar or time columns. Two clicks set a span.",
        "字段点开日历或时间列，两次点击定出一段区间。",
    ),
    (
        "Pages of periods; arrows move, Page keys turn the page. Weeks follow ISO 8601.",
        "按页显示时间段；方向键移动，Page 翻页，周次遵循 ISO 8601。",
    ),
    (
        "Spans named from today, every zone the system knows, and hours and minutes.",
        "从今天起算的时间段、系统认识的全部时区，以及小时与分钟。",
    ),
    (
        "Five fields, read back in words with the next runs. Presets fill it in.",
        "五个输入位，回读成文字并附下一次运行；预设可一键填入。",
    ),
    (
        "Click a field or Browse for the system's dialog, or drop files from Finder. A folder, or two files on a one-file target, turns it red.",
        "点输入框或 Browse 走系统对话框，也可从访达拖入；选了文件夹、或在单文件限制下选了两个文件，输入框会变红。",
    ),
    (
        "Checked once you leave the field; a quiet note says what fits.",
        "离开输入框时校验，一行安静的小字说明什么样才合规。",
    ),
    (
        "Typing fits the pattern; literals appear on their own.",
        "边打边套格式，固定的字面字符自动出现。",
    ),
    (
        "Six boxes, one field underneath: type, paste, or backspace through them.",
        "六个格子共用下面那一个输入域：打字、粘贴或退格都会在格间移动。",
    ),
    (
        "Click the text to edit. Enter or leaving keeps it; Escape puts it back.",
        "点击文字开始编辑；Enter 或失焦保存，Escape 还原。",
    ),
    (
        "Arrow keys or the stepper move it; drag the grip sideways to scrub.",
        "方向键或步进器增减，抓住侧边的把手横向拖动可快速扫值。",
    ),
    (
        "Drag the label sideways, as in a design tool. The field still types.",
        "像设计工具那样横向拖标签；输入框照样可以打字。",
    ),
    (
        "The same number field with a symbol, a percent, or a unit you click through.",
        "同一个数字输入框，带符号、百分号或可点选的单位。",
    ),
    (
        "Sign with the pointer. A stroke lands when the pen lifts; Clear starts over.",
        "用指针签名，笔抬起即落笔；Clear 重新来过。",
    ),
    (
        "One line of code, colored the way most languages read. JSON is checked as you type.",
        "单行代码，按多数语言的惯例着色；JSON 边打边校验。",
    ),
    (
        "Every family the system knows, and a line set in the chosen one.",
        "系统认识的全部字体，以及一行按所选字体排的文字。",
    ),
    (
        "Type to narrow. Down moves into the grid, arrows walk it, Enter picks.",
        "输入即筛选；Down 进入列表，方向键走格，Enter 选中。",
    ),
    (
        "Languages under their own names, countries with flags, currencies by ISO 4217. Type a name or a code.",
        "语言按自称列出，国家带旗帜，货币按 ISO 4217；输入名字或代码即可筛选。",
    ),
    (
        "A button that opens its list. Arrows move, Enter picks, Escape closes.",
        "点按钮展开列表；方向键移动，Enter 选中，Escape 关闭。",
    ),
    (
        "A list chosen from in place. Focus it and use the arrows, Space or Enter.",
        "就地展开的列表；聚焦后用方向键、Space 或 Enter。",
    ),
    (
        "Type to filter. A combobox keeps a listed value; an autocomplete keeps any text.",
        "输入即筛选；combobox 只保留列表里的值，autocomplete 保留任意文本。",
    ),
    (
        "Several values as chips. The list stays open while you pick; Backspace drops the last.",
        "多个值排成小片；选择时列表保持展开，Backspace 删最后一个。",
    ),
    (
        "Each column opens the next. Right and Left walk the levels; a leaf picks the path.",
        "每选一列展开下一列；Right 与 Left 走层级，叶子节点确定路径。",
    ),
    (
        "Mark rows on one side, then move them across.",
        "在一侧勾选行，然后整体搬去另一侧。",
    ),
    (
        "@ and # open suggestions at the caret. Arrows choose, Enter picks.",
        "@ 与 # 在光标处弹建议；方向键选择，Enter 采纳。",
    ),
    (
        "Enter or a comma adds a chip; Backspace on an empty field drops the last.",
        "Enter 或逗号生成小片；空输入框按 Backspace 删最后一个。",
    ),
    (
        "Rows you add and remove. Each row reports as you type.",
        "可增删的行，每行边打边上报。",
    ),
    (
        "Type a path, or Browse with the system's dialog.",
        "输入路径，或 Browse 走系统对话框。",
    ),
    (
        "Colored by part as you type; a broken pattern says why.",
        "边打边按分段着色；格式错了会说明原因。",
    ),
    (
        "A formula, colored and evaluated as you type.",
        "公式边打边着色并即时求值。",
    ),
    (
        "Focus it and press a shortcut; it keeps that one and lets go.",
        "聚焦后按一个快捷键，它会记住这个组合然后松手。",
    ),
    (
        "Sections with room between them. Errors take the description's place; Cmd-Enter saves from any field.",
        "分节之间留空；错误信息占用说明的位置；任意字段里按 Cmd-Enter 都能保存。",
    ),
    (
        "Fields and actions in one row, their bottoms aligned.",
        "字段与动作排成一行，底边对齐。",
    ),
    (
        "One line. Undo, word jumps, IME and the clipboard come built in.",
        "单行输入；撤销、按词跳转、输入法与剪贴板都是内置的。",
    ),
    (
        "Several lines. It grows to its content, then scrolls.",
        "多行输入；先随内容长高，长满后再滚动。",
    ),
    (
        "Bullets by default; the eye shows the text. Masked text never reaches the clipboard.",
        "默认圆点遮蔽，眼睛图标可显文；被遮蔽的内容永远进不了剪贴板。",
    ),
    (
        "A magnifier, a clear button, and recent searches while it is empty.",
        "放大镜、清空按钮，空着时展示最近搜索。",
    ),
    (
        "Addons join the field's frame: a scheme before, a unit or a control after.",
        "附加件嵌进输入框边框：前面是协议头，后面是单位或控件。",
    ),
    (
        "Pills for a few options. Filter chips toggle and show a check.",
        "少量选项排成胶囊；过滤片可切换并带对勾。",
    ),
    (
        "Hover previews, a click sets it, and clicking the same star clears it.",
        "悬停预览、点击设定，再点同一颗星即清空。",
    ),
    (
        "Drag a thumb, click the track, or use the arrows; Page keys jump ten steps.",
        "拖滑块、点轨道或用方向键；Page 键一次跳十步。",
    ),
    (
        "Drag a knob up or down, or use the arrows. The stepper counts without typing.",
        "上下拖旋钮或用方向键；步进器不用打字就能增减。",
    ),
    // ---- 6 Forms: section notes ----
    (
        "On, off, or mixed when a parent's children disagree. The check pops in.",
        "开、关，或父项的子项不一致时的混合态；对勾弹入。",
    ),
    (
        "One Tab stop per group; the arrow keys move the choice.",
        "每组一个 Tab 停留点，方向键移动选择。",
    ),
    (
        "Larger targets that explain a choice. The chosen border turns to ink.",
        "更大的命中区，顺带解释选项；选中项的边框变成墨色。",
    ),
    (
        "Takes effect at once. The thumb slides with a little overshoot.",
        "改动立即生效，滑块带一点回弹地滑过去。",
    ),
    (
        "Saturation and value on the plane, hue and alpha on the rails. Type it as hex, RGB or HSV; drag a letter to scrub.",
        "平面上调饱和度与明度，轨道上调色相与透明度；可用 hex、RGB 或 HSV 输入，拖字母可微调。",
    ),
    (
        "Named patches; see-through colors sit on a checkerboard. The dropper reads the screen.",
        "命名的色板；半透明色放在棋盘格上。取色器可读屏幕。",
    ),
    (
        "Click the bar to add a stop, drag stops to move them, Delete removes the chosen one.",
        "点色条加节点、拖节点移动，Delete 删除所选节点。",
    ),
    (
        "Arrows move a cursor, Page keys turn the month, Enter picks. Today wears a ring.",
        "方向键移动光标，Page 翻月，Enter 选中；今天带一圈描边。",
    ),
    (
        "Fields that open a calendar or time columns. Two clicks set a span.",
        "字段点开日历或时间列，两次点击定出一段区间。",
    ),
    (
        "Pages of periods; arrows move, Page keys turn the page. Weeks follow ISO 8601.",
        "按页显示时间段；方向键移动，Page 翻页，周次遵循 ISO 8601。",
    ),
    (
        "Spans named from today, every zone the system knows, and hours and minutes.",
        "从今天起算的时间段、系统认识的全部时区，以及小时与分钟。",
    ),
    (
        "Five fields, read back in words with the next runs. Presets fill it in.",
        "五个输入位，回读成文字并附下一次运行；预设可一键填入。",
    ),
    (
        "Click a field or Browse for the system's dialog, or drop files from Finder. A folder, or two files on a one-file target, turns it red.",
        "点输入框或 Browse 走系统对话框，也可从访达拖入；选了文件夹、或在单文件限制下选了两个文件，输入框会变红。",
    ),
    (
        "Checked once you leave the field; a quiet note says what fits.",
        "离开输入框时校验，一行安静的小字说明什么样才合规。",
    ),
    (
        "Typing fits the pattern; literals appear on their own.",
        "边打边套格式，固定的字面字符自动出现。",
    ),
    (
        "Six boxes, one field underneath: type, paste, or backspace through them.",
        "六个格子共用下面那一个输入域：打字、粘贴或退格都会在格间移动。",
    ),
    (
        "Click the text to edit. Enter or leaving keeps it; Escape puts it back.",
        "点击文字开始编辑；Enter 或失焦保存，Escape 还原。",
    ),
    (
        "Arrow keys or the stepper move it; drag the grip sideways to scrub.",
        "方向键或步进器增减，抓住侧边的把手横向拖动可快速扫值。",
    ),
    (
        "Drag the label sideways, as in a design tool. The field still types.",
        "像设计工具那样横向拖标签；输入框照样可以打字。",
    ),
    (
        "The same number field with a symbol, a percent, or a unit you click through.",
        "同一个数字输入框，带符号、百分号或可点选的单位。",
    ),
    (
        "Sign with the pointer. A stroke lands when the pen lifts; Clear starts over.",
        "用指针签名，笔抬起即落笔；Clear 重新来过。",
    ),
    (
        "One line of code, colored the way most languages read. JSON is checked as you type.",
        "单行代码，按多数语言的惯例着色；JSON 边打边校验。",
    ),
    (
        "Every family the system knows, and a line set in the chosen one.",
        "系统认识的全部字体，以及一行按所选字体排的文字。",
    ),
    (
        "Type to narrow. Down moves into the grid, arrows walk it, Enter picks.",
        "输入即筛选；Down 进入列表，方向键走格，Enter 选中。",
    ),
    (
        "Languages under their own names, countries with flags, currencies by ISO 4217. Type a name or a code.",
        "语言按自称列出，国家带旗帜，货币按 ISO 4217；输入名字或代码即可筛选。",
    ),
    (
        "A button that opens its list. Arrows move, Enter picks, Escape closes.",
        "点按钮展开列表；方向键移动，Enter 选中，Escape 关闭。",
    ),
    (
        "A list chosen from in place. Focus it and use the arrows, Space or Enter.",
        "就地展开的列表；聚焦后用方向键、Space 或 Enter。",
    ),
    (
        "Type to filter. A combobox keeps a listed value; an autocomplete keeps any text.",
        "输入即筛选；combobox 只保留列表里的值，autocomplete 保留任意文本。",
    ),
    (
        "Several values as chips. The list stays open while you pick; Backspace drops the last.",
        "多个值排成小片；选择时列表保持展开，Backspace 删最后一个。",
    ),
    (
        "Each column opens the next. Right and Left walk the levels; a leaf picks the path.",
        "每选一列展开下一列；Right 与 Left 走层级，叶子节点确定路径。",
    ),
    (
        "Mark rows on one side, then move them across.",
        "在一侧勾选行，然后整体搬去另一侧。",
    ),
    (
        "@ and # open suggestions at the caret. Arrows choose, Enter picks.",
        "@ 与 # 在光标处弹建议；方向键选择，Enter 采纳。",
    ),
    (
        "Enter or a comma adds a chip; Backspace on an empty field drops the last.",
        "Enter 或逗号生成小片；空输入框按 Backspace 删最后一个。",
    ),
    (
        "Rows you add and remove. Each row reports as you type.",
        "可增删的行，每行边打边上报。",
    ),
    (
        "Type a path, or Browse with the system's dialog.",
        "输入路径，或 Browse 走系统对话框。",
    ),
    (
        "Colored by part as you type; a broken pattern says why.",
        "边打边按分段着色；格式错了会说明原因。",
    ),
    (
        "A formula, colored and evaluated as you type.",
        "公式边打边着色并即时求值。",
    ),
    (
        "Focus it and press a shortcut; it keeps that one and lets go.",
        "聚焦后按一个快捷键，它会记住这个组合然后松手。",
    ),
    (
        "Sections with room between them. Errors take the description's place; Cmd-Enter saves from any field.",
        "分节之间留空；错误信息占用说明的位置；任意字段里按 Cmd-Enter 都能保存。",
    ),
    (
        "Fields and actions in one row, their bottoms aligned.",
        "字段与动作排成一行，底边对齐。",
    ),
    (
        "One line. Undo, word jumps, IME and the clipboard come built in.",
        "单行输入；撤销、按词跳转、输入法与剪贴板都是内置的。",
    ),
    (
        "Several lines. It grows to its content, then scrolls.",
        "多行输入；先随内容长高，长满后再滚动。",
    ),
    (
        "Bullets by default; the eye shows the text. Masked text never reaches the clipboard.",
        "默认圆点遮蔽，眼睛图标可显文；被遮蔽的内容永远进不了剪贴板。",
    ),
    (
        "A magnifier, a clear button, and recent searches while it is empty.",
        "放大镜、清空按钮，空着时展示最近搜索。",
    ),
    (
        "Addons join the field's frame: a scheme before, a unit or a control after.",
        "附加件嵌进输入框边框：前面是协议头，后面是单位或控件。",
    ),
    (
        "Pills for a few options. Filter chips toggle and show a check.",
        "少量选项排成胶囊；过滤片可切换并带对勾。",
    ),
    (
        "Hover previews, a click sets it, and clicking the same star clears it.",
        "悬停预览、点击设定，再点同一颗星即清空。",
    ),
    (
        "Drag a thumb, click the track, or use the arrows; Page keys jump ten steps.",
        "拖滑块、点轨道或用方向键；Page 键一次跳十步。",
    ),
    (
        "Drag a knob up or down, or use the arrows. The stepper counts without typing.",
        "上下拖旋钮或用方向键；步进器不用打字就能增减。",
    ),
    // ---- 7 Data Display: section notes ----
    (
        "Codes for cameras and scanners: dark on a light tile in both themes, so they read anywhere. Code 128 takes printable ASCII; EAN-13 adds its own check digit.",
        "给摄像头与扫码器读的条码：两种主题下都是浅底深码，任何地方都扫得出来。Code 128 收可打印 ASCII；EAN-13 自带校验位。",
    ),
    (
        "How full things are. A meter turns amber past 80% and red past 95%; a gauge rolls its number and sweeps its arc; a usage bar splits one whole into parts.",
        "表示有多满。仪表过了 80% 变黄、过 95% 变红；环形表滚动数字并扫过弧线；用量条把一个整体拆成几份。",
    ),
    (
        "A score in stars, read only. Parts of a star fill too; to set a score, use Rating in Forms.",
        "只读的星级评分，星星可以只填一部分；要打分请用 Forms 里的 Rating。",
    ),
    (
        "Options side by side, a row per feature. One column can stand out.",
        "选项并排，每个特性行对一行；可以让其中一列突出。",
    ),
    (
        "Slides one at a time, gliding between them; past the last comes the first. Arrows show on hover, the dots stretch toward where it goes, and Left and Right step it. With autoplay it turns on its own and holds while pointed at.",
        "一次一张地滑动切换，到最后一张再回到第一张。悬停显示箭头，圆点朝去向拉伸，Left 与 Right 逐张切换。开自动播放会自己翻页，指针停在它上面时暂停。",
    ),
    (
        "Pictures in square tiles. Press one, or Enter on a focused tile, to open the lightbox there.",
        "方格里的图片；点击一张，或在聚焦的格上按 Enter，从那里打开灯箱。",
    ),
    (
        "One picture twice: flat as it came, and graded. Drag the divider, press anywhere on the picture, or use Left and Right.",
        "同一张图看两遍：原片与调色后。拖分隔线、在图上任意点按，或用 Left 与 Right。",
    ),
    (
        "Faint text over a page, for drafts and shared screens. The rows run level, and presses reach the page below.",
        "页面上的淡色文字，用于草稿与共享屏幕；行列保持水平，点击会穿到下面的页面。",
    ),
    (
        "Small labels tinted by what they mean, counts whose digits roll, and a dot for something new.",
        "按含义着色的小标签、数字滚动变化的计数，以及表示有新东西的小圆点。",
    ),
    (
        "Short labels for a kind, a topic or a filter; an x takes one away.",
        "标记种类、主题或过滤器的短标签；点 x 移除一条。",
    ),
    (
        "A picture, or initials on a tone the name keeps, or an icon. Presence sits on the corner. Hover the last one, then press it to choose a picture, or drop one on it.",
        "图片、按名字配色写首字母，或一个图标；在线状态挂在角上。悬停最后那颗再点按可换头像，也能直接拖一张进去。",
    ),
    (
        "People side by side, each tucked under the next, with a count of the rest; someone named inline.",
        "人们并排堆叠、彼此压边，附剩余人数；也可在行内点名某人。",
    ),
    (
        "Figures that matter, rolling to each new value, with how they moved. For latency, down is the good way.",
        "关键数字滚动到每个新值，并标出变化方向。对延迟而言，下降才是好事。",
    ),
    (
        "Labels and their values: in a quiet column beside them, or stacked where room is narrow.",
        "标签与值的组合：宽处排在旁边的安静一列，窄处上下堆叠。",
    ),
    (
        "An inspector: groups fold, each row a name and its editor. The owner keeps the values; the frame on the right follows them.",
        "检查器：分组可折叠，每行一个名字配一个编辑器。值由所有者持有，右侧的框跟着变。",
    ),
    (
        "Events down a rail, marked by an icon in a tinted ring or by a dot, with detail below.",
        "沿一条轨展开的事件，用着色圆环里的图标或圆点标记，下面是详情。",
    ),
    (
        "Who did what, and when, newest first. A new entry folds in at the top; the times stay fresh.",
        "谁在何时做了什么，最新的在前；新条目从顶部折入，时间保持新鲜。",
    ),
    (
        "Releases, newest first, each change under its kind.",
        "发布版本，最新在前，每条变更归在它的类型下。",
    ),
    // ---- 8 Lists & Trees: section notes ----
    (
        "A project's files: folders first, an icon for each kind, git status by letter and tint, and a dot on folders with changes. Type to filter; matches keep their folders, which open.",
        "项目的文件：文件夹在前，每类一个图标，git 状态用字母加颜色表示，有改动的文件夹带点。输入即筛选；命中的条目保留所在文件夹并自动展开。",
    ),
    (
        "A folder's contents in columns. A header sorts by its column and a second press reverses it; folders stay first. Open a folder with a double press or Enter; the path above climbs back.",
        "文件夹内容分列显示；点表头按该列排序，再点一次反向，文件夹始终在前。双击或 Enter 打开文件夹，上方的路径可以逐级返回。",
    ),
    (
        "Ten thousand rows, each as tall as it needs; only those near the view are built.",
        "一万行、各行自取所需的高度；只构建视口附近的那些。",
    ),
    (
        "Scroll to the end and the next page comes, a spinner holding its place while it loads. After five pages, the list ends quietly.",
        "滚到末尾就会来下一页，加载时转圈占位；翻过五页后列表安静地结束。",
    ),
    (
        "Rows under their letter; each letter pins to the top while its group scrolls past.",
        "按首字母归组的行；某组滚过时，它的字母钉在顶部。",
    ),
    (
        "Rows with something to lead with, a title and a quieter line, and something at the end. Rows that can be pressed light on hover.",
        "有前导元素的行：标题加一行更安静的说明，尾部还有点缀；可点击的行悬停时会亮。",
    ),
    (
        "A press picks one; Cmd-press adds or drops one, Shift-press takes a range. With focus, Up and Down move, Shift extends, Space toggles and Cmd-A takes all.",
        "点击选中一个；Cmd 点击加选或减选，Shift 点击选区间。聚焦后 Up/Down 移动、Shift 扩选、Space 切换、Cmd-A 全选。",
    ),
    (
        "Drag a row to its new place, or focus the list and press Alt with Up or Down. The others glide aside.",
        "把行拖到新位置，或聚焦列表后按 Alt+Up/Down；其余的行会滑开让位。",
    ),
    (
        "Swipe a row left with two fingers to show what can be done to it; it snaps open or shut as the fingers lift.",
        "双指把行向左划，露出可做的操作；抬手时吸附到开或关。",
    ),
    (
        "Folders open and close; Right, Left, Up and Down walk them, F2 renames, and a drag lands before, after or into a folder. Shift and Cmd widen the pick. Assets loads its children the first time it opens. Rows draw only in view, so large trees stay quick.",
        "文件夹可开合；Right/Left/Up/Down 走树，F2 重命名，拖动可落在某文件夹之前、之后或之中，Shift 与 Cmd 扩选。Assets 首次展开时才加载子节点；只绘制视口内的行，大树也快。",
    ),
    (
        "A box on every row. A branch's box checks all it holds, and shows a dash when only some are.",
        "每行一个勾选框；分支的框会勾选它包含的全部，只选中一部分时显示横线。",
    ),
    (
        "A select whose choices are a tree, each indented under its parent.",
        "选项是一棵树的 Select，每个选项缩进在父项之下。",
    ),
    (
        "A document's headings, each level stepping in. The marker glides to the section you pick.",
        "文档的标题列表，逐级缩进；标记会滑到你选的章节。",
    ),
    // ---- 9 Navigation: section notes ----
    (
        "Every command in one place. Recent ones lead; typing ranks fuzzy matches and marks the letters that matched.",
        "所有命令集中一处，最近用过的在前；输入即模糊排序，并标出命中的字母。",
    ),
    (
        "Files by path. The name leads and its folder follows; recent files show before you type.",
        "按路径列文件：文件名在前、所在目录在后；还没输入就先显示最近文件。",
    ),
    (
        "Documents, sessions and projects, most recent first. It opens on the one before, so Enter goes back.",
        "文档、会话与项目，最近在前；默认停在上一个，按 Enter 即回去。",
    ),
    (
        "One field searches everywhere and marks each match. As a launcher it opens alone, above every window, and closes when you look away.",
        "一个输入框搜遍所有地方并标记每处命中；作为启动器时独立浮在所有窗口之上，失焦即关。",
    ),
    (
        "Each level is a place. A level with siblings opens them, the way an editor's path bar does.",
        "每级都是一个位置；有兄弟节点的层级会展开它们，就像编辑器的路径条。",
    ),
    (
        "The first, the last, and the pages around this one. A gap stands for two or more pages.",
        "首页、末页与当前页附近的几页；省略号代表隔了两页以上。",
    ),
    (
        "Finished steps carry a check and take you back. The line into the current step fills as you advance.",
        "完成的步骤带对勾、可点回去；通向当前步骤的线随推进逐渐填满。",
    ),
    (
        "Places in a side column. Groups fold; folded to a rail, items keep their icons and name themselves on hover.",
        "侧栏一列的位置；分组可折叠，收成图标栏后仍保留图标，悬停自报名字。",
    ),
    (
        "A click opens an entry's panel; while one is open, hovering another switches. Arrows walk it, Escape closes.",
        "点击展开某条目的面板；已有时悬停另一条即切换。方向键走位，Escape 关闭。",
    ),
    (
        "Back and forward step one place; the clock lists every place, newest first.",
        "后退与前进各走一步；时钟图标列出所有去过的地方，最新在前。",
    ),
    (
        "Anchors mark where sections start. The table marks the one in view and brings any to the top.",
        "锚点标记各章节的起点；列表标出当前可见的章节，点任意一条都能滚到顶部对齐。",
    ),
    (
        "Line, or line and column. The note says where you will land, or why you cannot.",
        "行号，或行列号；旁边的说明告诉你会跳到哪，或为什么去不了。",
    ),
    (
        "A line slides to the chosen tab. Arrows move it; disabled tabs are skipped.",
        "一条线滑到所选标签；方向键移动，被禁用的标签自动跳过。",
    ),
    (
        "Close, pin, drag to reorder. A dot marks unsaved work until you hover; italics mark a preview. The chevron lists every tab.",
        "关闭、置顶、拖动重排；未保存用圆点标记（悬停才显示），斜体表示预览；箭头列出全部标签。",
    ),
    // ---- 10 Menus: section notes ----
    (
        "The app's own menus, the ones cx.set_menus gave the system, drawn in the window. Picking a row dispatches its action.",
        "应用自己的菜单（cx.set_menus 交给系统的那套），直接画在窗口里；选中一行即派发对应动作。",
    ),
    (
        "A menu of links wide enough for a line of detail each.",
        "一列链接组成的菜单，宽到每条都能带一行说明。",
    ),
    (
        "A filter field heads the menu. Typing narrows the rows, best first, and marks the letters that matched.",
        "菜单顶部是过滤框；输入即收窄条目、最像的在前，并标出命中的字母。",
    ),
    (
        "A right click opens a ring at the pointer. The pointer's direction marks a slice and a click runs it; arrows turn the ring, the hub cancels.",
        "右键在指针处张开一个环形菜单；指针朝向选定扇区、单击执行，方向键转动圆环，中心按钮取消。",
    ),
    (
        "Rows with an icon and a key hint, split by rules and titled groups. The hint only shows a chord; the app binds it. A row with an arrow opens the next menu beside it; Right and Left walk in and out.",
        "带图标与键位提示的行，用分隔线与标题分组。提示只展示组合键，绑定在应用；带箭头的行会在旁边展开下一级菜单，Right 与 Left 进出层级。",
    ),
    (
        "A check flips on its own; a radio is one of its group. The owner holds both and redraws the menu.",
        "勾选项自己翻转；单选项在组内互斥。两者都由所有者持有状态并重绘菜单。",
    ),
    (
        "A button that opens a menu; a main action with related ones under its arrow; and More, for what did not fit.",
        "点开的菜单按钮；主操作加箭头下的相关操作；以及装不下的那些放进 More。",
    ),
    (
        "A right click opens the menu where it lands; it stays inside the window.",
        "右键在落点处打开菜单，且不会跑出窗口。",
    ),
    // ---- 11 Overlays: section notes ----
    (
        "A panel of any content under its button. A press outside or Escape closes it, and Tab stays inside. The Link variant sets the trigger inline.",
        "按钮之下弹出任意内容的面板；点外面或 Escape 关闭，Tab 困在面板内。Link 变体把触发器内联在文字里。",
    ),
    (
        "Rest the pointer on a name and a card opens. It stays while the pointer is on either, and never takes focus.",
        "指针停在一个名字上就展开卡片；指针在名字或卡片任一方都不消失，且从不抢焦点。",
    ),
    (
        "A card over a scrim; Tab stays inside and focus returns on close. An alert holds until acknowledged, a confirm asks first, a prompt checks its one line, and a fullscreen dialog fills the window.",
        "遮罩之上的卡片；Tab 困在里面、关闭时焦点归还。alert 要确认才散、confirm 先问再做、prompt 校验那一行输入、fullscreen 铺满窗口。",
    ),
    (
        "They live with the pieces they are made of.",
        "它们住在组成自己的那些零件里。",
    ),
    (
        "A picture over a dark layer, one at a time. Arrows step, Escape or a press on the dark closes it, and each picture fades in.",
        "暗层之上的图片，一次一张；方向键切换，Escape 或点暗层关闭，每张淡入。",
    ),
    (
        "The window dims around one control and a card explains it. A tour walks the stops; the lit box glides from one to the next.",
        "窗口围绕某个控件变暗，一张卡片解释它。导览会走完全部站点，高亮框在站点之间滑移。",
    ),
    (
        "Tools float over a selection while the field has focus, and leave focus where it is.",
        "字段聚焦时，工具浮在所选内容上方，且不打扰当前焦点。",
    ),
    (
        "A panel opens in the flow under its line, the way an editor peeks at a definition, and pushes what follows down.",
        "面板在该行的文档流中展开——就像编辑器偷看定义那样——并把后面的内容往下推。",
    ),
    // ---- 12 Feedback: section notes ----
    (
        "Short messages stack in the corner, the newest nearest it. Each rises in as its room opens and folds away under the rest. The pointer on the stack holds their time.",
        "短消息在角落里堆叠，最新的最靠角。每条随空位打开而浮现、被后来的压下去；指针停在堆上会延长它们的停留。",
    ),
    (
        "Messages under day headings. Unread ones carry a dot; Mark all read and Clear show only while they have work.",
        "按日期分组的消息；未读带点，“全部已读”与“清空”只在有活干时出现。",
    ),
    (
        "A message in the flow of the page, tinted by how much it matters.",
        "排在页面流里的消息，按重要程度着色。",
    ),
    (
        "A strip across the top of a window. Closing it folds it away.",
        "横贯窗口顶部的一条，关掉即收起。",
    ),
    (
        "A note set apart in reading: note, tip, warning and danger.",
        "阅读中单独隔出的提示：note、tip、warning 与 danger。",
    ),
    (
        "One line beside what it is about. A status line rises in each time it changes.",
        "贴着相关对象的一行字；状态行每次变化都会浮现。",
    ),
    (
        "What a view shows with nothing in it: no data yet, no results, or a first start.",
        "视图空空如也时给的东西：还没有数据、没有结果，或第一次启动。",
    ),
    (
        "A view that could not show what was asked says what happened and offers a way on.",
        "没能按请求展示的视图，会说明发生了什么并给出出路。",
    ),
    (
        "Shows its content, or the error it holds, logged once. It renders a Result; it never catches a panic.",
        "展示内容或其中的错误，只记一次日志。它渲染的是 Result，从不捕获 panic。",
    ),
    (
        "Where a task ended. The ring and its mark draw themselves once.",
        "任务落在了哪一步；圆环与记号只画一次。",
    ),
    (
        "Asks in place before an action runs, with what it will do.",
        "动作执行前就地询问，并说明它将要做什么。",
    ),
    (
        "Quiet lines for work in the background. Each rises in when its state changes; sync arrows turn while they work.",
        "后台任务用安静的行表示；状态一变就浮现，同步箭头在干活时转动。",
    ),
    (
        "Time left and time spent. Digits roll as they change.",
        "剩余时间与已花时间，数字随变化滚动。",
    ),
    // ---- 19 Git: section notes ----
    (
        "One file's changes, unified or side by side: each change in its context with old and new numbers, the words that changed washed, the stretches between folded until pressed. InlineDiff inside an editor is CodeEditor::set_diff.",
        "单个文件的改动，统一或并排视图：每处改动带上下文与新旧行号，变化的词淡化标注，中间未变的段落折叠到点击才展开。编辑器内的行内差异走 CodeEditor::set_diff。",
    ),
    (
        "Ours, the base and theirs side by side: clean changes merge themselves, each conflict waits for a side, and the result shows what is still open. In a file with markers, each conflict offers the current change, the incoming one, or both.",
        "我方、基准与对方三栏并排：无冲突的改动自动合并，每处冲突等你选边，结果里能看到还剩哪些没解决。带标记的文件里，每处冲突可选保留当前、 incoming 或两者都要。",
    ),
    (
        "Who last changed each run of lines, the bar fading with age, the current line annotated; a file's commits on a line through them with what each changed; sizes as counts and five dots; statuses as letters.",
        "每段行是谁最后改的：色条随时间变淡，当前行可展开批注；一条线串起文件的提交并注明各自改了什么；体量用数字与五个圆点表示，状态用字母表示。",
    ),
    (
        "A pull request at a glance: state, branches, checks, reviewers, comments and size. A review thread on a line: each comment, a reply, and resolve.",
        "一眼看完 PR：状态、分支、检查、评审人、评论与体量。行上的评审串：每条评论、回复，以及“已解决”。",
    ),
    (
        "The work tree's changes, the staged apart, each with its status and size; hover a file to open, discard, stage or unstage it. The subject counts toward fifty; Cmd-Enter commits.",
        "工作区的改动与已暂存的分开列出，各带状态与体量；悬停文件可打开、丢弃、暂存或取消暂存。标题算在五十字内；Cmd-Enter 提交。",
    ),
    (
        "Commits newest first beside the lanes they take: a branch forks off, merges join, each lane in its own color; refs sit on their commits.",
        "提交按最新在前排列，旁边是它们所在的泳道：分支分叉、合并汇入，每条泳道一种颜色；引用挂在各自的提交上。",
    ),
    (
        "Branches as a palette, local and remote apart, a new name offering a new branch; the lists show how far each branch is from its upstream, tags with their messages, and stashes to apply, pop or drop.",
        "分支做成面板，本地与远程分开，输入新名字即可建新分支；列表显示各分支与上游的差距、带消息的标签，以及可 apply、pop 或 drop 的 stash。",
    ),
    // ---- 17 Editor: section notes ----
    (
        "Rust, colored as it reads. The gutter holds breakpoints, problems, numbers, folds and git marks, and the blocks around the top line stay pinned as it scrolls. Cmd-D selects the next match, Cmd-Alt-Down adds a cursor, Alt-click adds one anywhere, Cmd-/ comments, Tab indents.",
        "Rust 按语义着色。行号槽放断点、问题、行号、折叠与 git 标记，滚过首行时上方的块保持钉住。Cmd-D 选下一个匹配，Cmd-Alt-Down 加光标，Alt 点按任意处加光标，Cmd-/ 注释，Tab 缩进。",
    ),
    (
        "Dots for spaces, and the cursor as a line, a block or an underline; it blinks, and holds still under reduced motion. Numbers count from the top or from the cursor.",
        "空格画成点，光标可选竖线、方块或下划线；会闪烁，开启减少动效后静止。行号从顶部计或从光标处计。",
    ),
    (
        "A changed line under the one it replaced; a suggestion in grey that Tab takes; a prompt between lines; a banner over code that cannot change here; and the mode of a modal editor.",
        "被替换行的下方显示旧行；灰色的建议按 Tab 采纳；行间插入的提示；不可编辑代码上方的横幅；以及模态编辑器的模式。",
    ),
    (
        "Completions as you type, the typed letters marked and the chosen one's documentation beside; the call's signature with the parameter at the cursor; and what a hover over a symbol reads.",
        "边打边补全，已输入部分标出、候选项的文档列在旁边；调用签名高亮光标所在参数；悬停符号即可读到说明。",
    ),
    (
        "Fixes and refactors behind a lightbulb; renaming in place; where a symbol is used; a file's symbols as a tree that follows the cursor, or as a palette to jump by name; and who calls a function.",
        "灯泡后面是修复与重构；就地重命名；符号的使用处；文件符号做成跟随光标的树，或按名字跳转的面板；以及谁在调用某函数。",
    ),
    (
        "Every problem by file, worst first, with toggles by severity; and a tool's log by channel, levels tinted, following the newest line.",
        "按文件列出所有问题、最严重的在前，可按严重级别筛选；工具日志按通道分组、级别着色，跟随最新一行。",
    ),
    (
        "An expression prompt beside a paused program; and extensions with their reach, rating, and the one action each needs next.",
        "暂停程序旁的表达式输入；以及扩展及其权限范围、评分，和每个扩展下一步需要的授权。",
    ),
    (
        "Find over code by case, whole word or pattern; the current match glows among the rest, and the arrows step between them. The replace row swaps one or every match.",
        "按大小写、全词或模式在代码中查找；当前命中在其余之中发光，箭头逐个跳转。替换行可换一处或全部。",
    ),
    (
        "Matches across files, each file folding away, each line trimmed near its match; globs choose which files count.",
        "跨文件的命中，按文件折叠，每行截取命中附近；glob 决定哪些文件参与。",
    ),
    (
        "Settings as controls by section, each change marked with a way back, or as the JSON a settings file holds; and every command's keys, clashes marked. Double-press a command to record new keys.",
        "设置既可按节显示为控件（每次修改都有标记与撤销），也可看设置文件的 JSON；以及每条命令的按键，冲突会标出。双击命令即可录制新键。",
    ),
    (
        "Where an editor starts: ways in, recent projects and a switcher; each theme painted in its own colors; and tasks with how their last run went, one open to its configuration.",
        "编辑器的起点：各种入口、最近项目与切换器；每个主题按自己的配色绘制；以及任务与上一次运行的结果，可点开看配置。",
    ),
    (
        "The quiet row under the code. Each item says one thing and opens the one control that changes it: press the language, the line ending or the indent to switch them, the server to step through its states, the bell to read.",
        "代码下面那条安静的行。每个元素只说一件事、点开正好是改它的控件：点语言、行尾或缩进可切换，点服务器看它的状态流转，点铃铛读通知。",
    ),
    // ---- 18 Terminal: section notes ----
    (
        "Each command a block: where it ran, how it ended and how long it took, its output folding away; copy it or run it again. Past commands open as a palette, found by any of their letters.",
        "每条命令一个块：在哪运行、如何结束、耗时多少，输出默认折叠；可复制或重跑。历史命令做成面板，输入任意字母即可找到。",
    ),
    (
        "Text colored by the escape codes in it, as a terminal prints it: the sixteen theme colors, bold, dim, italic, underline, inverse, the 256-color cube and true color. Other codes drop away.",
        "文本按其中的转义序列着色，与终端打印一致：主题十六色、粗体、暗淡、斜体、下划线、反显、256 色立方与真彩色；其余序列被丢弃。",
    ),
    (
        "A real shell on a pseudo-terminal, drawn cell by cell in the theme's sixteen colors. Tabs hold shells and output; split for another shell beside it. Find lights every match through the scrollback. Cmd-press a link or a path:line.",
        "跑在伪终端上的真实 shell，按主题十六色逐格绘制。标签页装 shell 与输出；可分屏再开一个。查找会点亮回滚区里的每一处命中。Cmd+点击链接或 path:line。",
    ),
    (
        "A log from a chosen level up, filtered as you type, following new lines until a scroll up lets go. Long logs draw only the lines in view.",
        "从所选级别起过滤日志，输入即筛，自动跟随新行——向上滚动即解除跟随。长日志只绘制可见行。",
    ),
    (
        "Running processes, busiest first: found by a query, sorted by any column, and the selected ones ended.",
        "运行中的进程，最忙在前：按查询过滤、按任意列排序，可结束选中的进程。",
    ),
    // ---- 20 Debug: section notes ----
    (
        "Memory as a hex dump, sixteen bytes a row in fours beside the same bytes as text; the byte under the pointer lights on both sides and the picked one reads below as numbers. Machine code with its source lines, registers and numbers colored, the current instruction marked; press the gutter to set a breakpoint.",
        "内存十六进制转储：每行十六字节、四个一组，旁边是同样字节的可读文本；指针所指字节两侧同时高亮，选中的字节在下方以数字解读。机器码带源码行，寄存器与数字着色，当前指令有标记；点行号槽设断点。",
    ),
    (
        "Where the time went: each function as wide as its samples with its callees under it; press a frame to zoom in and a caller to zoom back out. Work over time on tracks; the wheel zooms around the pointer and a sideways wheel pans.",
        "时间花在哪：每个函数的宽度就是它的采样量，被调函数列在其下；点帧放大、点调用方缩小。时间轴上铺开工作；滚轮以指针为中心缩放，横向滚轮平移。",
    ),
    (
        "Requests in the order they started, with status, kind, size and time, and a waterfall of when each ran; a filter keeps matching addresses. Press one to read its headers and body.",
        "请求按发起顺序排列，带状态、类型、大小与耗时，以及各自运行时刻的瀑布图；过滤器保留匹配的地址。点开一条可读请求头与响应体。",
    ),
    (
        "The floating row that drives a session: continue or pause, the three steps while paused, restart and stop. Each tooltip names the usual key; the app binds it.",
        "驱动调试会话的悬浮行：继续或暂停，暂停时的三种单步，以及重启与停止。每个提示都写着惯用按键，绑定在应用。",
    ),
    (
        "Every breakpoint with its condition and hits; the stack where the program waits, library frames folded until pressed; and the threads, one picked. The gutter's breakpoints are the editor's (CodeEditor).",
        "全部断点及其条件与命中次数；程序等待处的调用栈，库帧默认折叠、点击展开；线程列表里选中一个。行号槽上的断点与编辑器（CodeEditor）是同一份。",
    ),
    (
        "Variables by scope as a tree, colored by kind, with the value that changed at the last step washed; expressions watched at every step, errors in red. The REPL beside a paused program is the editor's DebugConsole.",
        "按作用域组织的变量树，按类型着色，上一步变化的值淡化标注；每步都会求值的监视表达式，错误显红。暂停程序旁的 REPL 是编辑器的 DebugConsole。",
    ),
    // ---- 21 Documents: section notes ----
    (
        "Markdown that reads as it is written: styles show as they are typed and their marks recede. The toolbar above and the one over a selection apply formats; ⌘K edits the link at the caret. @ suggests people, a colon and two letters suggest emoji. The foot counts words, characters and minutes.",
        "所见即所写的 Markdown：样式边打边生效、标记随之隐去。顶部工具栏与选区上的浮动栏负责套格式；⌘K 编辑光标处的链接；@ 联想人名，冒号加两个字母联想 emoji。底部统计字数、字符数与阅读时长。",
    ),
    (
        "One document three ways: visual, where it reads as it will and a pressed block opens as its source; source, styled as typed; and split, the source beside what it makes.",
        "一份文档三种视图：visual 按最终效果阅读、点某个块展开其源码；source 按输入样式显示源码；split 则源码与结果并排。",
    ),
    (
        "Blocks like a notebook's. Hover one for its handle: drag it to move the block, press it to turn, copy or delete it. / on an empty line lists every kind, and an empty line says so. Undo covers text and blocks alike.",
        "类似 notebook 的块。悬停出现把手：拖动移动块，点按可转换、复制或删除。空行输入 / 列出全部块类型，空行也会有提示。撤销同时覆盖文本与块。",
    ),
    (
        "Code colored as code, math set as it is typed, diagrams the app draws from their source, dividers, tables that grow and merge cells, pictures to resize and align, pages to open, columns, and synced copies that change together.",
        "代码按代码着色，公式边打边排版，图表由源码即时绘制；还有分隔线、可增行并合并单元格的表格、可缩放对齐的图片、可打开的页面、分栏，以及同步联动的副本。",
    ),
    (
        "To-dos under a title, with how many are done as a count and a bar; a done one goes quiet and struck through.",
        "标题下的一组待办，完成数以计数与进度条呈现；已完成的一项变淡并划线。",
    ),
    (
        "Pages kept close at hand above the tree of every page: favorites open with a press, drag to reorder and let go by their star. The tree is lists::Tree, a page's emoji leading its label.",
        "全部页面组成的树之上是触手可及的收藏：点按即打开，拖动排序，拖离星标即移除。树是 lists::Tree，页面 emoji 领在标签前。",
    ),
    (
        "A page's head: its cover, its icon set large over the cover's edge, its title, and its properties. Hover the cover to change or remove it, the icon to pick another. The properties are data_display::PropertyGrid, a field's editor in each row.",
        "页面头部：封面、压在封面边缘的大号图标、标题与属性。悬停封面可更换或移除，悬停图标可另选。属性区是 data_display::PropertyGrid，每行一个字段的编辑器。",
    ),
    (
        "The pages that link here, each with the words around its link; and every page as a node, its links as edges. GraphView is charts::NetworkGraph.",
        "链接到这里的页面，各带链接前后的文字；以及把每页当节点、链接当边的图。GraphView 是 charts::NetworkGraph。",
    ),
    (
        "Pages to start from: found by name, kept by category, the one in view shown as it will read; Use takes it.",
        "可从这些页面起步：按名字查找、按分类收藏，当前选中的按最终效果预览；点 Use 采用。",
    ),
    (
        "Deleted pages, newest first, with where each lived and when it went: restore one, delete one for good or empty it all, each final step asked twice.",
        "已删除的页面，最新在前，注明原位置与删除时间：可恢复单页、彻底删除单页或清空全部，每个不可逆动作都会二次确认。",
    ),
    (
        "A page's versions by time and author: read one, or see word by word what it changed from the one before, added words washed and removed ones struck; restore any but the current.",
        "页面按时间与作者排列的版本：可阅读某一版，或逐词查看它与上一版的差异——新增淡化、删除划线；除当前版外都可恢复。",
    ),
    (
        "Markdown as a document, its headings marked for a table of contents beside it: the section in view is lit as it scrolls, and a press brings a section to the top. The bar above fills as the text scrolls. DocumentOutline is the same TableOfContents.",
        "把 Markdown 当文档读，标题标记出来供旁边的目录使用：滚动时高亮当前章节，点任一条把该章节顶到最上。上方进度条随文字滚动填充。DocumentOutline 与 TableOfContents 是同一个组件。",
    ),
    (
        "Pages the app drew, on a desk: the page in view and its count, zoom by steps or to the width, the pages as thumbnails beside, a find bar whose hits light on the page, and notes pinned where a press puts them. The app reads its PDF; the viewer shows it.",
        "应用画好的页面摆在桌面上：当前页与总页数、按档或按宽缩放、旁边的缩略图、命中会在页面上点亮的查找条，以及点哪钉哪的便签。PDF 由应用自己解析，查看器只负责展示。",
    ),
    (
        "A slide deck, a sheet or a document the app turned into pages, shown by the same viewer.",
        "幻灯片、表格或被应用转成页面的文档，用的都是同一个查看器。",
    ),
    (
        "A book a chapter at a time, in a column sized for reading: how far through the chapter, the chapters listed, the type a step larger or smaller, and the next chapter a press away.",
        "一次一章的阅读视图，列宽适合阅读：章节内进度、章节列表、字号可大可小，点一下进入下一章。",
    ),
    (
        "A note's number in running text, the note a rest of the pointer away; a term underlined in dashes that explains itself; and every term in order under its letter.",
        "正文中的注释编号，悬停片刻即可看到注释内容；虚线下划线的术语会自我解释；全部术语按字母顺序列在下面。",
    ),
    (
        "Zen mode folds the tools away and sets the text in a quiet column; Escape brings them back.",
        "禅模式收起所有工具，把正文放进安静的一列；Escape 恢复。",
    ),
    (
        "The pages the app laid out for the settings, small on a desk, with paper, turn, margins and copies beside; each change asks the app for new pages.",
        "应用为设置项排出的页面，缩在桌面上，旁边是纸张、单双面、页边距与份数；每次改动都会向应用要新页面。",
    ),
    // ---- 27 Files: section notes ----
    (
        "What a right press on files offers, as menus::ContextMenu draws it: open, rename, duplicate, and the trash apart.",
        "在文件上右键给出的东西，由 menus::ContextMenu 绘制：打开、重命名、复制一份，回收站单独隔开。",
    ),
    (
        "A name edited in place, as forms::InlineEdit does it: a press or Enter edits, Enter keeps and Escape drops; the icon follows the new name.",
        "就地编辑名字，做法同 forms::InlineEdit：点按或 Enter 进入编辑，Enter 保留、Escape 放弃；图标会跟着新名字变。",
    ),
    (
        "Files dropped in, as forms::DropZone takes them; each lands in the list below.",
        "拖入的文件由 forms::DropZone 接住，逐个落到下面的列表里。",
    ),
    (
        "A folder three ways: a list that sorts by column, tiles, or columns that walk the path. Its name, its count and a switch sit above the path, which climbs; a folder opens in place.",
        "文件夹的三种看法：按列排序的列表、平铺块、逐级下钻的多列。路径上方是名字、计数与切换开关，路径可逐级返回；文件夹就地展开。",
    ),
    (
        "The same folders as lists::FileTree draws them: folders first, a filter above, Enter or a double press opens a file.",
        "与 lists::FileTree 相同的文件夹视图：文件夹在前，上方是过滤器，Enter 或双击打开文件。",
    ),
    (
        "A file shown large: its picture in its shape, its text's first lines, or its kind. Space on a picked tile shows it over the page, as a quick look does; Escape puts it away.",
        "放大展示的文件：按原比例显示图片、文本的前几行或类型。选中块上按 Space 像快速预览那样盖在页面上，Escape 收走。",
    ),
    (
        "Files opened lately, newest first, under a header for each day. Enter or a double press opens one.",
        "最近打开的文件，最新在前，按日期分头；Enter 或双击打开。",
    ),
    (
        "Places pinned for quick reach, as documents::Favorites keeps them: a press opens one, a drag reorders, the star lets one go.",
        "为快速到达而固定的位置，维护方式同 documents::Favorites：点按打开，拖动排序，拖离星标即解除固定。",
    ),
    (
        "A search over files: the host searches as the name is typed, and each file found shows its folder. Enter or a double press opens one.",
        "文件搜索：边输入边由宿主查找，每个结果都标出所在文件夹。Enter 或双击打开。",
    ),
    (
        "Files found more than once, each copy with a mark; the last copy a group keeps stays. What the marked copies free sits beside Remove.",
        "重复找到的文件，每份副本带标记；组内保留最后一份。移除按钮旁标出可释放的空间。",
    ),
    (
        "What an archive holds, as a tree of its folders: each file with its size and how much packing saved.",
        "压缩包里有什么：按文件夹组成的树，每个文件标出大小与压缩省下的空间。",
    ),
    (
        "A file's first bytes, as debug::HexViewer shows them: offsets, hex and text side by side.",
        "文件的头几个字节，展示方式同 debug::HexViewer：偏移、十六进制与文本并排。",
    ),
    (
        "A file's icon by its name's extension: code, text, pictures, sound, video, archives, tables, fonts and scripts; a folder shut or open.",
        "按扩展名给出的文件图标：代码、文本、图片、音频、视频、压缩包、表格、字体与脚本；文件夹分开合两态。",
    ),
    (
        "A file or folder as a row: its icon and name over its size and when it changed.",
        "把文件或文件夹画成一行：图标与名字在上，大小与修改时间在下。",
    ),
    (
        "A folder's entries in columns, as lists::DirectoryListing draws them: sort by a column, climb the path, open with Enter or a double press.",
        "多列呈现的文件夹条目，与 lists::DirectoryListing 相同：按列排序、沿路径返回，Enter 或双击打开。",
    ),
    (
        "Files and folders as tiles. A press picks one and a double press opens it; with focus the arrows move by tile and by row, and Enter opens.",
        "平铺的文件与文件夹。点选、双击打开；聚焦后方向键按块与按行移动，Enter 打开。",
    ),
    (
        "A copy or a move under way: what it does and where to, a bar, the bytes done with the time left, and the file it takes now. Pause, resume and cancel while it runs.",
        "进行中的复制或移动：做什么、去哪里、进度条、已完成字节与剩余时间，以及正在处理的文件。运行中可暂停、继续或取消。",
    ),
    (
        "Uploads and downloads in one queue: which way each goes, its bar and status, and pause, resume, retry or cancel as its state allows.",
        "上传与下载共用一个队列：各自的方向、进度条与状态，并按状态提供暂停、继续、重试或取消。",
    ),
    (
        "Downloads as a browser keeps them: those on their way above, the finished below under Clear. A finished one opens with a press and shows its folder.",
        "浏览器式的下载列表：进行中的在上，已完成的在 Clear 之下；点已完成的即打开，并可显示所在文件夹。",
    ),
    (
        "Where a disk's room goes, as data_display::UsageBar shows it: a part a kind, and what is left free.",
        "磁盘空间的去向，画法同 data_display::UsageBar：一类一段，剩余空间单独标出。",
    ),
    // ---- 33 DB & Dev Tools: section notes ----
    (
        "Saved requests in folders; a drag moves one, and Enter opens it in the builder with its method and address, its query's pairs, headers, body and sign-in, and the address that goes out with the environment's variables filled in. Send brings back an answer: its status, time and size, then its body or its headers.",
        "保存的请求分文件夹存放，拖动可移动；Enter 在构建器里打开它——方法、地址、查询对、请求头、请求体与鉴权，以及用环境变量填好的最终地址。Send 发回响应：状态、耗时与大小，以及响应体或响应头。",
    ),
    (
        "A socket's conversation: where it stands, each message marked sent or received with its time, JSON indented, and a field that sends on Enter.",
        "一段 socket 会话：当前状态，逐条消息标注发送或接收及时间，JSON 缩进展示，还有回车即发送的输入框。",
    ),
    (
        "A schema's types and fields beside a query and its variables; Run brings back the answer as a tree.",
        "模式里的类型与字段列在查询与变量旁；Run 以树的形式返回结果。",
    ),
    (
        "A query in the code editor over its result in the data grid; the queries that ran, found by their text, each with how it ended, and one run again on Enter; and the plan, each step with its share of the cost.",
        "代码编辑器里的查询对着数据网格里的结果；历史查询可按文本查找、各带结束方式，Enter 重跑一条；还有执行计划，每一步标出成本占比。",
    ),
    (
        "Keys found by a glob, each with its type and the time it has left; the key picked shows what it holds, laid out for its type.",
        "按 glob 找到的键，各带类型与剩余时间；选中某个键即按其类型展示内容。",
    ),
    (
        "Saved connections with their engine and address, or why one failed. Enter or a double press connects; the pencil opens one in the form, where Test tries it and Save keeps it.",
        "保存的连接及其引擎与地址，失败的会说明原因。Enter 或双击连接；铅笔在表单里打开，Test 试连，Save 保存。",
    ),
    (
        "A database's schemas, tables and views, each field with its type and keys marked. The diagram draws each table as a box of fields and a line from every foreign key to the field it points at; a table's title drags it.",
        "数据库的模式、表与视图，每个字段标出类型与键。关系图把每张表画成一个字段框，外键到目标字段之间连线；拖表标题即可移动。",
    ),
    (
        "A table's fields edited in place: a name or a default rewritten on a press, a type, whether it may be empty, and the key. Its indexes list their fields and method, and a new one is drawn up from the table's fields.",
        "表的字段就地编辑：点一下即可改名字或默认值，还有类型、是否可空与键。索引列出各自的字段与方法，新索引可直接从表的字段拼出来。",
    ),
    (
        "A document as a tree: each key with its value, lists and maps with their counts; a search keeps what holds it and opens the way, and Enter on a value copies its path. The editor writes JSON with Format and Minify and says whether it reads. YAML reads as the same tree; XML as elements with their attributes and text.",
        "把文档画成树：每个键配值，列表与映射带数量；搜索会保留持有它的路径并展开，在值上按 Enter 复制其路径。编辑器带 Format 与 Minify，并判断 JSON 是否合法。YAML 读成同一棵树；XML 则是带属性与文本的元素。",
    ),
    (
        "A pattern tried against a sample with each match washed and its groups listed; text encoded or decoded as Base64, for a URL or as hex; and text on a background with the contrast between them and the WCAG levels it meets.",
        "用样本试正则，每处命中淡化显示并列出分组；文本按 Base64、URL 或十六进制编解码；以及文字与背景的对比度及其达到的 WCAG 等级。",
    ),
    (
        "Containers with where each stands and its load, started, stopped or restarted from its row; pods in a table narrowed by namespace; and a machine's readings, each with its latest value, its recent past and how full it is.",
        "容器及其状态与负载，可在行上启动、停止或重启；pod 表格按命名空间收窄；以及机器的读数——最新值、近期走势与用量占比。",
    ),
    // ---- 43 Library Tooling: section notes ----
    (
        "The theme's shared colors and scales in the mode shown, each named as code reads it: every palette, syntax, chart and terminal color with its hex, and the type, radius, control, icon, avatar and container sizes, the elevations and the spacing steps.",
        "当前模式下主题的共享颜色与刻度，命名与代码一致：全部调色板、语法、图表与终端色都带 hex，以及字号、圆角、控件、图标、头像与容器尺寸、阴影层级和间距档位。",
    ),
    (
        "The library's icons, searchable: this is forms::IconPicker, its home. Pick one to see the name code uses.",
        "库的图标可搜索：它就是 forms::IconPicker 的老家。点一个即可看到代码里用的名字。",
    ),
    (
        "The theme's contrast in the mode shown: each text color over the surfaces it sits on and each code color over the editor's at 4.5:1, the focus ring and the chart hues beside the page at 3:1, failures first. Ely's palettes pass in both modes, with high contrast and without.",
        "当前模式下主题的对比度：每种文字色在其底面上的对比、每种代码色对编辑器底色达到 4.5:1，焦点环与图表色对页面达到 3:1，不达标排前。Ely 的调色板在明暗两种模式、含与不含高对比度下都达标。",
    ),
    (
        "The gallery photographs itself: --capture writes every page top to bottom, light and dark, then each page's scripted states, with input posted to its own event queue. Each pass starts its page fresh, and reduced motion holds the shots still.",
        "画廊给自己拍照：--capture 从上到下写出每页的明暗两版，再拍每页的脚本化状态，输入走各自的事件队列。每一遍都让页面从头开始，减少动效会让画面保持静止。",
    ),
    (
        "gpui's inspector, which debug builds have, drawn in Ely's look. Inspect opens it at the window's right: hovering picks a box and shows its margin, border, padding and size over the page, the wheel steps out to the boxes around it, and a press holds it.",
        "gpui 的检查器（debug 构建自带），按 Ely 的观感绘制。Inspect 在窗口右侧打开它：悬停选中一个盒子并在页面上标出其 margin、border、padding 与尺寸，滚轮逐层向外选包裹它的盒子，点按锁定。",
    ),
    (
        "Frames per second over the last second, the worst frame, and a bar per frame against the 60 Hz budget: green on time, amber a frame late, red later. It keeps the window drawing while shown, so Measure shows it only while you look.",
        "最近一秒的帧率、最慢的一帧，以及每帧对 60 Hz 预算的柱条：准时绿色、晚一帧黄色、更晚红色。显示期间会持续驱动窗口绘制，所以 Measure 只在你盯着看时才显示它。",
    ),
    (
        "How often the view that draws it renders. The page's count climbs with every frame the window draws; the view beside it is drawn cached, so it renders only when it redraws itself or the window refreshes.",
        "绘制它的那个视图渲染了多少次。页面的计数随窗口每帧递增；旁边的视图是缓存绘制，只有自重绘或窗口刷新才渲染。",
    ),
    (
        "The input events that reach what it holds, newest first: presses, releases, moves run together while the pointer travels, wheels, keys and modifiers, each timed from the first.",
        "到达其内容的事件，最新在前：按下、松开、指针移动（移动会合并连发）、滚轮、按键与修饰键，时间都从第一条起算。",
    ),
    (
        "This app is the gallery: a page per chapter of the library, light and dark, each with scripted states. The --page flag opens one page, and --capture photographs every page and state.",
        "本应用就是画廊：库的每章一页，明暗两版，各带脚本化状态。--page 打开某一页，--capture 给每一页与每个状态拍照。",
    ),
    (
        "A component beside the knobs that shape it: a switch, a choice or a number edits one property, and the preview redraws.",
        "组件与塑造它的旋钮并排：一个开关、一个选项或一个数字改一个属性，预览随即重绘。",
    ),
    // ---- 44 Rendering: section notes ----
    (
        "The host draws every pixel of each frame at the box's size in device pixels, here a field of three sources in contour bands. The slider moves a source; the frame draws again only for a new size or revision. Shaders, cameras, video and other GPU work run on the host's side and hand their frames in: gpui shares no device, so zero-copy textures and Metal, OpenGL, Vulkan or Direct3D surfaces have no way in.",
        "宿主按盒子尺寸以设备像素绘制每一帧——这里是三个源在等值带中的场。滑块移动源；只有尺寸或版本变化才会重绘。着色器、相机、视频等 GPU 工作都跑在宿主侧、再把帧交进来：gpui 不共享设备，所以零拷贝纹理与 Metal/OpenGL/Vulkan/Direct3D 表面都进不来。",
    ),
    // ---- 39 Theme: section notes ----
    (
        "The theme laid open: every change here applies to the whole gallery at once, the colors as the shown mode's own. The theme switch above is settings::ThemeSelector.",
        "主题摊开给你看：这里的每一处修改都立即作用于整个画廊，颜色即当前模式的颜色。上方的主题切换器是 settings::ThemeSelector。",
    ),
    (
        "A VS Code color theme dropped here reads into Ely's palette, the colors it leaves out kept from Ely's; Apply sets it on the whole gallery. Code palettes choose the colors of code for the mode shown.",
        "把 VS Code 的配色主题丢进来，会读进 Ely 的调色板，它没覆盖的颜色保留 Ely 的；Apply 作用于整个画廊。代码配色决定当前模式下代码的颜色。",
    ),
    (
        "Warm neutrals. Depth comes from tone, not shadow.",
        "暖色中性底。层次来自色调，不靠阴影。",
    ),
    (
        "Four steps of text. Blue marks focus and links.",
        "文字四档；蓝色标出焦点与链接。",
    ),
    (
        "Status colors, each with a quiet fill.",
        "状态色，各配一个安静的底。",
    ),
    (
        "Eight chart colors, even in weight.",
        "八种图表色，分量均匀。",
    ),
    (
        "Code tokens sit below text in weight.",
        "代码 token 的分量比正文低。",
    ),
    (
        "Inter for words. JetBrains Mono for code.",
        "正文 Inter，代码 JetBrains Mono。",
    ),
    ("Four radii. Three elevations.", "四档圆角，三档阴影。"),
    (
        "Ease out to arrive. Springs only where a hand leads. Slowed to one second here.",
        "用 ease-out 到位；只有手拖的地方才用弹簧。这里放慢到一秒。",
    ),
    (
        "The theme's platform decides window buttons and key caps, and starts as the system's. An icon theme lays an owner's rules over Ely's file icons. A frosted pane lets a blurred window's backdrop through; gpui blurs as vibrancy on macOS and acrylic on Windows, and sets Mica on Windows 11.",
        "主题的平台决定窗口按钮与键帽，初始取系统值。图标主题把所有者的规则叠在 Ely 的文件图标之上。毛玻璃面板让模糊的窗口背景透进来；gpui 在 macOS 上是 vibrancy、Windows 上是 acrylic，Windows 11 用 Mica。",
    ),
    // ---- 40 i18n & a11y: section notes ----
    (
        "A link that shows only while focused and moves focus past the nav to the main part: Tab into the card, then Enter. Arrows among a group's items live in interaction::RovingFocus.",
        "只在聚焦时出现的链接，把焦点从导航直接送到正文：先 Tab 进卡片，再按 Enter。组内条目的方向键导航在 interaction::RovingFocus。",
    ),
    (
        "Ely's chart hues share one lightness, so protanopia, deuteranopia and tritanopia run some together. The safe hues differ in lightness too: every pair stays apart for each, and each is 3:1 on its page.",
        "Ely 的图表色共享同一明度，红/绿/蓝三型色弱都会把它们混在一起。安全色在明度上也拉开了差距：对每种色弱任意两两分开，且对页面都达到 3:1。",
    ),
    (
        "One global locale, one catalogue per language, read through the same rust_i18n the host uses. A message with an unfilled argument fails; an unknown key falls back to English.",
        "一个全局语言、每种语言一份目录，走的与宿主相同的 rust_i18n。参数没填全的消息会报错；未知键回退英文。",
    ),
    (
        "Each locale's separators and its short numeric date, from CLDR.",
        "各语言的分隔符与短日期格式，取自 CLDR。",
    ),
    (
        "A right-to-left locale starts its stacks and sets its text at the right; pick עברית above. Ely's other components stay left to right.",
        "从右到左的语言会把堆叠镜像、文字靠右——在上面选 עברית 试试。其余组件仍保持从左到右。",
    ),
    // ---- 38 Interaction: section notes ----
    (
        "gpui's own bindings: actions! names them, bind_keys ties keys to them under a key context, and a binding of two strokes waits for the second while the first shows. Focus the box and try them. Ely adds nothing.",
        "gpui 自带的绑定：actions! 给动作命名，bind_keys 在某个 key context 下把键绑上去；两段式绑定会先显示第一段、等第二段。聚焦盒子试试。Ely 没有加东西。",
    ),
    (
        "forms::History keeps whole snapshots, the one every editor here undoes with; a burst of typing makes one step.",
        "forms::History 保存完整快照，本页所有编辑器都用它撤销；连续输入只算一步。",
    ),
    (
        "gpui's uniform_list draws only the rows in view, ten thousand here. Two panes scroll as one: a wheel on either takes the other along. An endless list is lists::InfiniteList.",
        "gpui 的 uniform_list 只绘制可见行——这里有一万行。两个窗格同步滚动：动一个另一个跟上。无限列表见 lists::InfiniteList。",
    ),
    (
        "A field takes focus as it appears through gpui's window.focus. A toolbar keeps one Tab stop and the arrows walk it. Typing a row's first letters moves a list to it. The focus ring is primitives::FocusRing.",
        "字段出现即通过 gpui 的 window.focus 拿到焦点。工具栏只有一个 Tab 停留点，方向键在其中走。输入行的首字母即可把列表定位到它。焦点环是 primitives::FocusRing。",
    ),
    (
        "gpui's own drag and drop: on_drag carries a value and builds the view that follows the pointer, drag_over lights a lane, can_drop refuses the lane a chip left, and on_drop takes it. Ely adds nothing.",
        "gpui 自带的拖放：on_drag 携带值并构建跟随指针的视图，drag_over 点亮一条泳道，can_drop 拒绝小片离开的泳道，on_drop 接住它。Ely 没有加东西。",
    ),
    (
        "A box resized from its edges or its grip; a picture turned by its knob, as gpui turns svgs alone; tiles taken by a band drawn on empty space; rows sorted by drag through motion::Reorder.",
        "盒子可从边缘或把手缩放；图片靠旋钮旋转（gpui 只有 svg 能转）；空白处画框即框选块；行排序走 motion::Reorder 的拖拽。",
    ),
    (
        "A long press is primitives::Pressable; a double press is gpui's click_count. The wheel pans and, with Command, zooms layout::Viewport; two fingers swipe lists::SwipeableListItem.",
        "长按是 primitives::Pressable；双击走 gpui 的 click_count。滚轮平移，配合 Command 在 layout::Viewport 缩放；双指滑动见 lists::SwipeableListItem。",
    ),
    // ---- 16 Finance: section notes ----
    (
        "What the portfolio is worth and how it moved, split across holdings; the same split as a donut; its growth against the market with each fall from a high below; and the market's mood on a dial.",
        "投资组合的价值与走势，按持仓拆分；同样的拆分画成环形图；相对市场的增长与每次回落；以及表盘上的市场情绪。",
    ),
    (
        "Dividends by date with their yield, and a statement by quarter whose lines open into their parts.",
        "股息按日期列出并标收益率，季度对账单每行可展开明细。",
    ),
    (
        "An amount in another currency, at an illustrative rate; a wallet with its balance and an address to scan; transfers in and out with whether each settled; and the time left in the current candle.",
        "换算成另一种货币的金额（示例汇率）；钱包余额与可扫的地址；转入转出及是否到账；还有当前 K 线的剩余时间。",
    ),
    (
        "Quotes running past in a band, and symbols at a glance: the price flashes green or red as it moves, and the badge reads the move since the last close.",
        "滚动的行情带与一眼可读的标的：价格跳动时闪绿或闪红，角标显示相对昨收的涨跌。",
    ),
    (
        "Orders waiting on each side, best in the middle, each level shaded by its running total; the spread and the balance between sides; and each venue's quotes side by side.",
        "买卖两侧的挂单、最优价在中间，每档按累计量着色；点差与两侧的平衡；以及各交易市场的报价并排对比。",
    ),
    (
        "Trades as they print, newest on top and large prints bold; and the book as a ladder around the last price. Press a size to trade at its price.",
        "逐笔成交，最新在上、大单加粗；以及围绕最新价的订单簿阶梯。点某个量即可按该价下单。",
    ),
    (
        "Prices as candles, bars, smoothed candles, a line or an area, with volume along the bottom and averages over the top. Drag or scroll sideways to move through time, hold Cmd and scroll to zoom, double-press to return to the newest.",
        "蜡烛、美国线、平滑蜡烛、折线或面积图，底部成交量、上方均线。拖动或横向滚动移动时间，按 Cmd 滚动缩放，双击回到最新。",
    ),
    (
        "Bollinger bands over the prices, MACD and RSI in panes below, and a second symbol drawn from the same start so their moves compare. Choose indicators from the menu.",
        "价格上的布林带、下方窗格的 MACD 与 RSI，以及从同一起点绘制的第二个标的，便于对比走势。指标从菜单选。",
    ),
    (
        "Pick a tool and drag over the prices: a trend line, a level, a box, or retracement levels between a swing's two ends. The pointer moves through time again.",
        "选个工具在价格上拖画：趋势线、水平位、方框，或一段摆动的两个端点之间的回撤位。指针再次随时间移动。",
    ),
    (
        "Every bid and ask as a running total out from the spread. Hover to read the total to any price.",
        "从点差向外的每个买卖价的累计量；悬停可读任意价格处的累计。",
    ),
    (
        "How much traded at each price in view, as bars from the right edge; the busiest price is the darkest.",
        "可见范围内每个价格的成交量，从右缘伸出的柱条；最活跃的价格颜色最深。",
    ),
    (
        "Time drops out: a brick for each whole move of two dollars, and columns of Xs and Os that turn after three boxes the other way.",
        "时间被抽掉：每整两美元一格砖，X 与 O 列在反向走满三格后换列。",
    ),
    (
        "Four symbols moving together: pan, zoom or point at one and the others follow. Switch between one chart, two across, two down or four.",
        "四个标的联动：平移、缩放或指向其中一个，其余跟随。可在单图、横向两图、纵向两图与四图之间切换。",
    ),
    (
        "Symbols to follow, each with its day as a line, its price and its move; a symbol as a tile of its letters; and a search by letters or name.",
        "自选标的：各带当日折线、价格与涨跌；字母方块形式的标的；以及按代码或名字搜索。",
    ),
    (
        "Each symbol a tile sized by its market value and tinted by its move: deeper green as it rises, deeper red as it falls. Hover a tile to read it.",
        "每个标的一方块按市值定大小、按涨跌着色：涨得越深绿、跌得越深红。悬停可读详情。",
    ),
    (
        "Securities narrowed by rules: build the rules above, and the table keeps only what passes them.",
        "按规则筛选证券：在上方拼规则，表格只保留符合条件的。",
    ),
    (
        "A market at a glance: its session, its indexes as cards, and the day's movers. Below, each market's hours across one day in UTC, those open now lit.",
        "一眼看市场：交易时段、卡片形式的指数、今日活跃股。下方是各市场在 UTC 一天之内的营业时间，正在开市的点亮。",
    ),
    (
        "The day's releases with how much each moves markets and its figure beside the forecast; reports due by date; and stories newest first, each dot leaning green or red. Press a story to open it.",
        "当日发布项及其对市场的影响、实际值与预测值对照；按日期排列的待发报告；最新在前的新闻，每个圆点偏绿或偏红。点开阅读。",
    ),
    (
        "An order filled in field by field, with what it comes to; its button asks before the order goes. Quick buttons trade at the bid or the ask in one press.",
        "逐字段填出的订单及合计；下单前按钮会再问一次。快捷按钮一按即按买价或卖价成交。",
    ),
    (
        "What is held and what it has made, orders still waiting with how much has filled, and each fill with its fee. Results read green for a gain and red for a loss.",
        "持仓及其盈亏、仍在等待的挂单及已成交量、每笔成交及其费用。盈利读绿、亏损读红。",
    ),
    (
        "Leverage marked at the common steps; the margin it takes against the account, and the risk it carries. Each turns amber, then red, as it climbs. Drag the slider.",
        "杠杆按常用档位标记；占用保证金与随之而来的风险。攀升时依次变黄、变红。拖动滑杆。",
    ),
    (
        "Calls to the left and puts to the right of each strike, the side in the money shaded and a line at the price; a position's greeks; and what a call spread pays at expiry. Hover the diagram to read any price.",
        "每个行权价左侧看涨、右侧看跌，价内一侧着色并有一条现价线；持仓的希腊字母；以及看涨价差在到期日的收益。悬停图上任意位置读数。",
    ),
    // ---- 26 Media: section notes ----
    (
        "A song the host plays: its cover, the time played and left along a scrubber, the transport and the volume.",
        "宿主在放的一首歌：封面、拖动条上的已播与剩余时间、播放控制与音量。",
    ),
    (
        "A sound's peaks, dark as far as it has played. A press or a drag seeks, the time under the pointer shows above it, and with focus the arrows step five seconds.",
        "声音的波形峰，已播放部分更深。点按或拖动定位，指针上方显示时间，聚焦后方向键一次五秒。",
    ),
    (
        "Sound by pitch, low to high. The host sends new levels as it plays; they hold while it pauses.",
        "按音高从低到高的声音。播放时宿主持续送来新电平，暂停时保持。",
    ),
    (
        "A speaker that mutes, drawn by the level, beside a slider. Muted, the slider rests at zero; moving it unmutes.",
        "一个可静音的喇叭按电平绘制，旁边是滑杆。静音时滑杆停在零，推动滑杆即取消静音。",
    ),
    (
        "How fast it plays, from half to double, chosen from a menu.",
        "播放速度从半速到双倍，从菜单里选。",
    ),
    (
        "Shuffle, back, play, forward and repeat. Shuffle and repeat stay lit while on; repeat steps from all to one to off.",
        "随机、上一首、播放、下一首与循环。随机与循环开着时保持点亮；循环在全部、单曲与关闭之间切换。",
    ),
    (
        "Songs in order. The one under way is lit, its bars moving while it plays; Enter or a double press plays another.",
        "按顺序排列的歌曲；正在播放的高亮，播放时 bars 滚动。Enter 或双击播放另一首。",
    ),
    (
        "Record, then pause, resume or stop. While it runs, a live dot and the time it has run; the mic and camera toggles choose what it takes.",
        "录制，然后暂停、继续或停止。运行中有红点与已录时长；麦克风与摄像头开关决定录什么。",
    ),
    (
        "What a camera sees, in its own shape, from frames the host hands in. With no frame, the camera is off.",
        "摄像头看到的画面，保持原始比例，帧由宿主送入。没有帧时表示摄像头未开。",
    ),
    (
        "A microphone's level as it comes in: amber near the top, red at the top.",
        "麦克风进来的电平：接近顶部变黄，到顶变红。",
    ),
    (
        "A microphone, a camera or a speaker to use, each with its kind's icon. With none attached, it says so and stays shut.",
        "要使用的麦克风、摄像头或扬声器，各带类型的图标；一个都没接时会说明并保持关闭。",
    ),
    (
        "A model the host renders, a frame for each view. A drag or the arrows turn around it; Command-scroll, plus and minus bring it nearer; 0 or the reset button returns.",
        "宿主渲染的模型，每个视图一帧。拖动或方向键绕转；Cmd 滚动、加号减号拉近；0 或复位按钮还原。",
    ),
    (
        "A box to crop a picture by. Drag it to move it and its handles to size it; with a shape, only its corners size it. The arrows move it, Shift further.",
        "裁剪图片的框。拖动移动、拖把手改大小；带形状时只有四角能改大小。方向键移动，加 Shift 更快。",
    ),
    (
        "A picture to send: drop one or browse for it, then crop it. Replace picks another; Remove empties it.",
        "要发送的图片：拖入或浏览选择，然后裁剪。Replace 另选一张，Remove 清空。",
    ),
    (
        "Marks on a picture: boxes, arrows, lines drawn freehand, numbered pins. Pick a tool and a color above; with Select, a press chooses a mark and Delete removes it. Cmd-Z undoes.",
        "图上的标注：方框、箭头、手绘线与编号图钉。上方选工具与颜色；用 Select 时点按选中标注、Delete 删除，Cmd-Z 撤销。",
    ),
    (
        "A picture to look at closely. It fits at first; the buttons, plus and minus, or Cmd-scroll zoom it, and 0 and 1 go back to fit and actual size, as a double press does. Drag it or use the arrows once it overhangs; R turns it.",
        "细看一张图。初始适配显示；按钮、加减号或 Cmd 滚轮缩放，0 与 1 回到适配与实际大小，双击亦可。超出后拖动或方向键移动，R 旋转。",
    ),
    (
        "Pictures small, each in its own shape, with a label when one helps. The one last opened keeps a ring; a press or Enter opens the lightbox, which steps through them all.",
        "小尺寸的图片，各自保持比例，需要时带标签。最近打开的一张带描边；点按或 Enter 打开灯箱，可在其中逐张看。",
    ),
    (
        "A video the host plays, a frame at a time. Its controls fade once the pointer rests while it plays. Space plays, J and L jump ten seconds, C turns captions, F fills the screen.",
        "宿主逐帧播放的视频。播放中指针停下时控件淡出。Space 播放，J 与 L 跳十秒，C 切字幕，F 全屏。",
    ),
    (
        "Where a video stands: what has loaded, how far it has played, its chapters, and the time under the pointer. A press or a drag seeks; with focus the arrows step five seconds.",
        "视频的进度：已加载多少、播到哪、章节划分，以及指针处的时间。点按或拖动定位；聚焦后方向键一次五秒。",
    ),
    (
        "Frames along the video under its playhead, and the part kept between two handles. A press seeks; a handle trims; I and O set the ends at the playhead.",
        "播放头下沿视频排开的帧，以及两个把手之间保留的片段。点按定位；把手裁剪；I 与 O 在播放头处设首尾。",
    ),
    (
        "Captions to edit in place, their times as mm:ss.mmm over their words. The one under the playhead is lit; Add puts a new cue there.",
        "就地编辑的字幕，文字上方是 mm:ss.mmm 时间。播放头处的字幕高亮；Add 在那里加一条新字幕。",
    ),
    // ---- 28 Messaging: section notes ----
    (
        "A huddle's mark rings while it is live; a press joins. In the call, a bar tells where it is, how long it has run and who is in it, and the controls turn the microphone, camera and a shared screen on and off, or leave.",
        "语音房进行中时记号会晃动，点按加入。通话中，一条栏告诉你位置、时长与成员；控件可开关麦克风、摄像头与共享屏幕，或离开。",
    ),
    (
        "Everyone in the call as tiles in even rows, a short last row centered: a camera's frame or the person's initials, their name with a mark when muted, and a ring while they speak. Your tile follows the controls above.",
        "通话中的每个人排成均匀的方块，最后不满一行居中：摄像头画面或姓名首字母，名字旁的标记表示静音，说话时有描边。你自己的方块跟随上方控件。",
    ),
    (
        "A shared screen in its shape under a bar that names who presents, the call's tiles beside it. Share your screen from the controls above, and the bar offers Stop presenting.",
        "共享屏幕按原比例展示，上方一条栏写着谁在共享，通话方块列在旁边。在上方控件里共享你的屏幕，栏上会出现停止共享。",
    ),
    (
        "A call coming in: the caller ringing, their name and the kind of call, with Decline and Accept. Escape declines.",
        "来电：呼叫中、对方名字与通话类型，带拒绝与接听。Escape 拒绝。",
    ),
    (
        "Messages under their day: one author's messages within five minutes form a run, the head only on the first and the time in the gutter on hover. A rule marks where unread ones start. Reactions and a thread sit under a message, and a line says who is typing.",
        "消息按日期归组：同一作者五分钟内的消息连成一串，头像只在首条出现，时间悬停才显示。未读起点有一条分隔线。回应与话题在消息下方，还有一行写着谁在输入。",
    ),
    (
        "A thread's mark under its message: who replied, how many, and when the last came; a press opens the thread. The panel holds the first message, a rule counting the replies, the replies in their runs, and a composer at the bottom.",
        "消息下方的话题标记：谁回了、回了几条、最后一条何时；点按打开话题。面板里有首条消息、计数的分隔线、按串排列的回复，底部是输入框。",
    ),
    (
        "Where a sent message stands: a clock while it goes, one tick once sent, two once delivered, two in the info tone once read. Read in a group, it shows who saw it.",
        "已发消息的状态：发送中是时钟，发出一枚勾、送达两枚勾、已读两枚变信息色。群聊已读还会显示谁看了。",
    ),
    (
        "Channels and people in sections, each a list: a press, an arrow or Enter opens a chat and clears what waited. Unread names stand out; a muted channel keeps quiet.",
        "频道与人按节分组、各成列表：点按、方向键或 Enter 打开会话并清掉等待。未读的名字更醒目；静音的频道保持安静。",
    ),
    (
        "A channel by its mark, a hash or a lock, with its unread count, quiet when muted, pinned with a pin; a person with their presence on their initials.",
        "频道带自己的记号——井号或锁——以及未读数；静音时安静，可置顶；人名首字母上带在线状态。",
    ),
    (
        "A channel's name and topic, and beside them its members, its pins and a call; the actions drop below on a narrow window.",
        "频道名与主题，旁边是成员、置顶与通话；窗口窄时这些动作落到下面。",
    ),
    (
        "Messages pinned in a chat, newest first: a press, an arrow or Enter opens one where it stands; Unpin lets one go.",
        "会话里的置顶消息，最新在前：点按、方向键或 Enter 打开原位；Unpin 解除置顶。",
    ),
    (
        "Members online and offline by name, each group under its count; offline members go quiet. A press or an arrow shows a member's card: their standing, title and status, the time where they are, and ways to reach them.",
        "在线与离线成员按名字分组、各归各组之下；离线的变淡。点按或方向键展开成员卡：状态、头衔与签名、所在时区，以及联系方式。",
    ),
    (
        "Where someone stands, as a dot in its color and its word.",
        "某人的状态：一个有颜色、有文字的圆点。",
    ),
    (
        "An emoji, a few words, and when they clear. A suggestion fills all three; Save sets the status, and Clear status removes it.",
        "一个 emoji、几个字，以及何时清掉。建议会填齐三项；Save 设状态，Clear status 清除。",
    ),
    (
        "Stickers a pack at a time, chosen above; a search spans every pack and keeps the stickers whose names hold it. Arrows move, and Enter or a press sends one.",
        "按包切换的贴纸；搜索覆盖所有包，保留名字命中的。方向键移动，Enter 或点按发送。",
    ),
    (
        "GIFs from the host in two columns packed tight, each moving. The host answers the search as it changes; while it is empty, categories fill it. Each GIF is a Tab stop that a press, Enter or Space sends.",
        "宿主给的 GIF 紧排两列、各自动图。搜索变化时宿主实时应答；为空时以分类填充。每个 GIF 是一个 Tab 停留点，点按、Enter 或 Space 发送。",
    ),
    // ---- 29 Mail: section notes ----
    (
        "Mailboxes, folders and labels in sections: a press, an arrow or Enter opens one, and the one open is lit. A box with mail unread stands out with its count; a label wears its hue.",
        "邮箱、文件夹与标签按节分组：点按、方向键或 Enter 打开，打开的高亮。有未读的邮箱带计数突出；标签带自己的颜色。",
    ),
    (
        "Messages with the sender strong while unread, the subject and first words under it, when each came, a paperclip for files, and a star to press. A press or an arrow selects; Command or Shift takes several.",
        "未读时发件人加粗，下面是主题与开头几句、收到时间，附件显示回形针，还有可点的星标。点按或方向键选择；Command 或 Shift 多选。",
    ),
    (
        "An inbox with nothing left in it says so, calm and brief.",
        "空空的收件箱会平静而简短地说明这一点。",
    ),
    (
        "A message open for reading: the subject, who wrote to whom and when, replies, the words and the files. Words quoted from before fold behind a small mark until pressed.",
        "打开阅读的邮件：主题、谁写给谁与何时、回复、正文与附件。引用的旧文收在小小的标记后，点开才见。",
    ),
    (
        "A conversation, oldest first: the newest stands open, and the others fold to a line of who, their first words and when, which a press, Enter or Space opens.",
        "会话从旧到新：最新的一封展开，其余折成一行——谁、开头几个字、何时——点按、Enter 或 Space 展开。",
    ),
    (
        "The labels a message wears: the field finds them by name, arrows move, and a press or Enter turns one on or off. A name no label has can be made into one, on at once.",
        "邮件上的标签：输入框按名字查找，方向键移动，点按或 Enter 开关；没见过的名字可以新建，创建即开启。",
    ),
    (
        "Put a message off until later today, tomorrow, this weekend or next week, each with its day and hour, or until a time of your own.",
        "把邮件延后到今天稍后、明天、本周末或下周，各带日期与时刻，也可以自定时间。",
    ),
    (
        "To, and Cc and Bcc once asked for, suggest people as you type and mark what is not an address. Then the subject, the words, the files and the signature. Send wakes once there is someone to send to, now or at a time; Discard lets the draft go.",
        "To，以及展开后的 Cc 与 Bcc，边输入边联想联系人并标出不像地址的内容；然后是主题、正文、附件与签名。只要有收件人（现在发或定时发）Send 就亮起；Discard 丢弃草稿。",
    ),
    (
        "A signature's lines in a field, and under it how they will sit below a message.",
        "签名内容在输入框里，下方预览它在邮件底部的样子。",
    ),
    (
        "Send now, or open the arrow for tomorrow morning, tomorrow afternoon, Monday morning, or a day and hour of your own.",
        "立即发送，或点箭头选明天早上、明天下午、周一早上，或自定日期与时刻。",
    ),
    // ---- 30 Calendar: section notes ----
    (
        "An event's parts in a form: its title, whole days or hours, where, its color, how it repeats with the next days it falls on, its reminders and notes. Save waits for a title and an end after the start; saved, every view on the page shows the change.",
        "表单里的活动要素：标题、全天或按小时、地点、颜色、重复规则及其后落上的几天、提醒与备注。Save 等待标题与晚于开始的结束；保存后本页所有视图同步更新。",
    ),
    (
        "Times to meet over the working days ahead: each day with its count of open slots, then the slots of the one chosen. What the page's events take is closed.",
        "未来工作日的开会时间：每天带空闲时段数，选中某天再列出它的时段。页面里已有活动占用的时段不可选。",
    ),
    (
        "A month in week rows. What spans days lies across them, the rest sits by the hour, and a crowded day keeps the rest behind N more. Arrows move a day, Page keys turn the month, and Enter or a double press opens the day.",
        "按周成行的月视图。跨天的横贯显示，其余按小时排，太挤的一天把多余的收进“还有 N 项”。方向键移动日期，Page 翻月，Enter 或双击打开。",
    ),
    (
        "A month at a glance is forms::Calendar. Here a day picked in it turns the month view above to its month.",
        "月视图概览就是 forms::Calendar。这里在它里面选一天，上方的月视图就跳到那个月。",
    ),
    (
        "Twelve small months. A day with something on it wears a dot in its first event's hue; arrows move a day, Page keys a month, and Enter or a double press opens the day.",
        "十二个小月。有安排的日子按第一个活动的颜色带点；方向键移动日期，Page 翻月，Enter 或双击打开。",
    ),
    (
        "An event in a line, by its hour or across whole days, and as a block with its hours and place, the way a time grid holds it.",
        "行形式的活动：按小时或跨全天；以及时间网格里那种带时刻与地点的块。",
    ),
    (
        "The days ahead with their events under each day's name, whole days first, then by the hour; days with nothing are left out. A press or an arrow selects, Enter or a double press opens.",
        "未来几天、各自的安排排在日期名下：全天在前、其余按小时；没安排的日子略去。点按或方向键选择，Enter 或双击打开。",
    ),
    (
        "An event's chip opens its details: when and where, who is asked and what each said, and what can be done with it. The list of guests stands on its own too.",
        "活动小片点开是详情：时间地点、受邀者与各自答复，以及可做的操作。受邀者列表也能单独展示。",
    ),
    (
        "A week under its days' names: whole days lie above, the rest by their hours, and events that overlap share the width. A drag on empty time makes an event of a quarter hour or more, and a drag on an event's lower edge moves its end. Today carries a line at the minute now.",
        "一周的视图按日期名分列：全天活动在上，其余按小时；重叠的活动分享宽度。在空白处拖动创建不少于一刻钟的活动，拖活动下缘改结束时间。今天带一条到分钟的线。",
    ),
    (
        "One day, with another zone's hours beside its own, so a call across zones reads in both.",
        "一天，旁边并排另一个时区的时刻，跨时区的通话两边都能对上。",
    ),
    // ---- 31 Project: section notes ----
    (
        "Columns of issue cards for a sprint. A card drags to a place in any column while the others glide aside, Option with an arrow moves the focused one, and a press or Enter opens it in the panel below. Sprint Board is this board.",
        "冲刺的工单卡片分列摆放。卡片可拖到任意列的任意位置，其余卡片滑开让位；Option 加方向键移动聚焦的卡片，点按或 Enter 在下方面板打开。Sprint Board 就是这块看板。",
    ),
    (
        "A task in full beside the board: its title edits in place, its properties change where they stand, and its notes and steps follow. Each edit shows on the board and in the list.",
        "看板旁完整显示一个任务：标题就地编辑，属性就地修改，备注与步骤跟随其后。每一处编辑都反映到看板与列表。",
    ),
    (
        "A team's labels: a press on a hue picks another, a name edits in place, and the field at the end makes a new one. Names stay unique whatever their case, and each change shows on the cards.",
        "团队的标签：点色块换颜色，名字就地编辑，末尾输入框新建。名字忽略大小写保持唯一，每处改动都反映到卡片上。",
    ),
    (
        "A roadmap lays initiatives across months in lanes, each bar filled as far as it has come, under a line at today; hover a bar for its dates. GanttChart and TimelineView are the charts GanttChart, for tasks by the day.",
        "路线图把举措按月铺进泳道，每条条形按进度填充，上方一条今天的线；悬停条形看日期。GanttChart 与 TimelineView 就是 GanttChart 图表——按天排任务。",
    ),
    (
        "Work left over a sprint: the ideal line from the scope down to nothing, what was left at each day's end so far, and a line at today. Work added mid-sprint lifts the line. Hover a day to read it. It lives with the charts as BurndownChart.",
        "冲刺剩余工作量：从总量降到零的理想线、至今每天结束时的剩余，以及今天的线。冲刺中途加需求会抬高线。悬停某天可读数。它就是图表里的 BurndownChart。",
    ),
    (
        "A milestone's issues done as a bar and a count, when it is due, and whether it keeps pace: on track, at risk when the work trails the time gone, late past its day, or done.",
        "里程碑的工单完成度用条形与计数表示，还有截止日与是否跟得上进度：正常；工作量落后于时间流逝为有风险，过了截止日为延期，完成即 done。",
    ),
    (
        "Who carries how much this sprint: each person's points against what they can take, the bar turning red past it.",
        "冲刺中每人扛了多少：各人的点数对比其容量，超出后条形变红。",
    ),
    (
        "How urgent a task is in three bars, urgent in red, its words in a tip or beside it. An issue goes by its team's key and its number.",
        "任务紧迫度用三根条表示，紧急为红，说明写在提示里或旁边。工单按团队 key 加编号标识。",
    ),
    (
        "A task's properties: where it stands in its workflow, who has it, and when it is due; DueDatePicker is the forms DatePicker. Each change shows in the list below.",
        "任务属性：工作流中的位置、负责人与截止日；DueDatePicker 就是 forms 的 DatePicker。每处修改都显示在下面的列表。",
    ),
    (
        "Tasks in rows: a box ticks one done, then its priority, its issue and its title; the due day and who has it sit at the end, and a day past due reads in red. Arrows move between rows; Enter or a double press opens one.",
        "任务逐行排开：勾选框打勾即完成，然后是优先级、工单号与标题；截止日与负责人在末尾，逾期用红色读出。方向键在行间移动，Enter 或双击打开。",
    ),
    (
        "A task's steps with how many are done, as a count and a bar. It is the documents Checklist, whose field adds a step on Enter.",
        "任务的步骤与完成数，以计数与进度条呈现。它就是 documents 的 Checklist，输入框按 Enter 加一步。",
    ),
    (
        "Time against a task: name it, Start or Enter starts the clock, and Stop turns it into an entry. Today's entries sit under it with their total.",
        "给任务计时：起个名字，按 Start 或 Enter 开始，Stop 生成一条记录。今天的记录列在下面并合计。",
    ),
    (
        "Focus for twenty-five minutes, rest for five, and after the fourth round for fifteen. The ring counts the phase down around the time left; a phase that ends waits for Start.",
        "专注二十五分钟，休息五分钟，第四轮之后休息十五分钟。圆环围绕剩余时间倒计当前阶段；阶段结束会等 Start。",
    ),
    (
        "The studio's tasks five ways: a table, a board by status, a gallery of cards, a month calendar by due day, and a timeline from start to due. A card moved on the board takes its column's status, and a record opens in the panel above.",
        "工作室任务的五种视图：表格、按状态的看板、卡片画廊、按截止日的月历，以及从开始到截止的时间线。看板上拖动的卡片采用所在列的状态，记录在上方面板打开。",
    ),
    // ---- 22 Collaboration: section notes ----
    (
        "People in the same document: who is here and where, comments and reactions, suggested changes, and sharing.",
        "同一份文档里的人：谁在、在哪，评论与回应、建议修改，以及分享。",
    ),
    (
        "What the host sees, drawn in a frame, with the rest of the call beside; who owns the cursor's tint.",
        "宿主看到的画面画在框里，旁边是通话其他人；以及光标颜色归谁。",
    ),
    (
        "People in the same call, each a tile; your tile follows the controls.",
        "同一通电话里的人，每人一块；你自己的块跟随控件。",
    ),
    (
        "Presence marks who is where; avatars carry the tint.",
        "在线状态标注谁在哪；头像带颜色。",
    ),
    (
        "A shared cursor in another person's tint, a name beside it.",
        "别人的共享光标带其颜色，旁边是名字。",
    ),
    (
        "Suggestions shown as marks over the text; a press opens the change.",
        "建议以标注形式盖在文字上，点按展开修改。",
    ),
    (
        "A change made together: one side edits, the other side reads.",
        "一起做的修改：一边编辑，一边阅读。",
    ),
    (
        "A share sheet: who may reach it and with what rights.",
        "分享面板：谁能访问、以什么权限。",
    ),
    // ---- 32 Canvas & Design: section notes ----
    (
        "Steps joined by links that follow them: ends as ellipses, work as boxes, choices as diamonds, and words on a link. With Select a step moves, resizes, and takes new words on a double press; the connector draws a link from one step to another.",
        "步骤由连线串联：端点画成椭圆、工作画成方框、判断画成菱形，连线上写着文字。用 Select 时步骤可移动、缩放，双击可改文字；连接器负责在两个步骤之间画线。",
    ),
    (
        "Topics branching from a root, half to each side. A press selects; arrows walk, Tab adds a child and Enter a sibling, Delete removes one, and F2 or a double press rewrites it. The owner keeps the map.",
        "主题从根向两侧分叉。点按选中；方向键走位，Tab 加子项、Enter 加兄弟，Delete 删除，F2 或双击改名。思维导图由所有者持有。",
    ),
    (
        "Nodes joined by wires, as in a pipeline editor. A title drags its node; a drag out of an output draws a wire that lands on an input of the same kind, each kind in its own hue. An input takes one wire, so a new one replaces the old; a press on a wired input lifts the wire to move it or let it go.",
        "节点由连线相连，就像流水线编辑器。标题拖动节点；从输出口拖出连线，落在同类的输入口上，每种类型一种颜色。一个输入口只接一根线，新线会替换旧线；点按已接线的输入口可把线提起，挪动或丢弃。",
    ),
    (
        "A board to think on, with its tools over its corner and its zoom at its foot. The note tool drops a sticky note, a double press writes on it, the connector links two notes, and Delete removes the selection. It keeps its own tool and view and hands the owner the whole board after each change.",
        "一块用来思考的白板，角落上是工具，底部是缩放。便签工具放下便利贴，双击书写，连接器连起两张便签，Delete 删除选中。它自己记住工具与视图，每次修改后把整块板交给所有者。",
    ),
    (
        "What a shape looks like. The color panel sets its fill, over the document's colors; the gradient panel is forms::GradientEditor; shadows stack; a border's corners join or split; the type panel sets a style over a line of it. The card takes each edit.",
        "形状长什么样。颜色面板在其文档色之上设置填充；渐变面板是 forms::GradientEditor；阴影可叠加；边框的圆角可合并或分离；文字面板为一行字设定样式。卡片接受每一处编辑。",
    ),
    (
        "How a shape keeps to its frame and lays out its children, and what it exports to. The first box shows where a child pinned so lands once its frame grows from the dashed size; the second lays three children by the controls; export names each file once.",
        "形状如何约束在框内、如何排布子元素，以及可以导出成什么。第一个盒子展示虚线尺寸放大后，钉住方式的子元素落在哪里；第二个按控件排三个子元素；导出为每份文件命名。",
    ),
    (
        "Panels on the drawing above. The layers list it topmost first: a box shows or hides a layer, F2 renames it, a drag restacks it. The inspector edits the one selected; the toolbar lines up or spreads several; an asset pressed lands on the canvas. Each change is a step in the history.",
        "上面画布的面板。图层列表把最上层的排前：勾选框显示或隐藏图层，F2 重命名，拖动重排。检查器编辑选中的一个；工具栏对齐或分散多个；点资产即可落到画布。每处修改都是历史里的一步。",
    ),
    (
        "An endless plane: a dot grid that thins as it zooms out, rulers in canvas units, guides, and artboards with their names. Drag to pan, scroll to move, and hold Command to zoom about the pointer. The zoom steps in fixed stops, and the minimap moves the view to a press.",
        "无尽画布：缩小时变稀的点阵、以画布为单位的标尺、参考线，以及带名字的画板。拖动平移，滚动移动，按住 Command 以指针为中心缩放。缩放按固定档位走，点小地图即可移动视图。",
    ),
    (
        "Tools at work on the plane. Select presses a shape, Shift adds, a drag moves it and snaps its edges to the others, a handle resizes it, and a drag on empty space draws a marquee. The shape tools draw by a drag, Shift keeping them square; the pen adds a point per press and a double press ends the path; text drops a line; the brush strokes. Each change is a step in the history, which a double press or Enter takes back to.",
        "在画布上工作的工具。Select 点选形状，Shift 加选，拖动移动并把边对齐到其他形状，把手缩放，空白处拖动出框选。形状工具拖画，Shift 保持正方；钢笔每按一点、双击结束路径；文字放下一行；笔刷涂抹。每处修改都是历史里的一步，双击或 Enter 可回退。",
    ),
    // ---- 34 Dashboard & Monitoring: section notes ----
    (
        "Widgets on a grid of twelve columns: a tile's top strip drags it and its corner resizes it, a cell at a time, while the others make room and rise to fill gaps; a focused tile steps with the arrows. Over it, the window of time and the filters, and how often it refreshes.",
        "十二列网格上的小组件：块顶部拖动移动、角落拖动缩放，一次一格，其余让位并补上空隙；聚焦的块可用方向键走。上方是时间范围、过滤器与刷新频率。",
    ),
    (
        "Sixty days of a service, a bar a day in the tone of its uptime; services under a line for the whole system; and checks with what each found, when it ran and how long it took.",
        "服务六十天，一天一根柱、按在线率着色；一条线之下是各服务；以及各项检查——查到了什么、何时跑的、跑了多久。",
    ),
    (
        "Alerts with their severity, what raised them and since when; a firing one is acknowledged from its row. An incident with its severity and phase, what it touches, and its updates newest first.",
        "告警及其级别、触发者与起始时间；正在触发的可在行上确认。事件带级别与阶段、影响范围，更新按最新在前。",
    ),
    (
        "Events as they come, newest first, kept by kind, with Pause to hold the list while new ones count up. Logs, traces, a machine's readings, its processes and live throughput have their homes in the terminal, agent, dev tools and charts chapters; the maps live in the maps chapter.",
        "事件按到达顺序、最新在前，按类型归档；Pause 可暂停列表、新事件累计计数。日志、追踪、机器读数、进程与实时吞吐各有归属——在终端、智能体、开发工具与图表章节；地图在地图章节。",
    ),
    // ---- 36 Account: section notes ----
    (
        "The account and the workspace in use, each a row that opens a menu of the others; the avatar opens who is signed in, the owner's rows, and Sign out.",
        "当前账户与工作区，各成一行、可打开其余项的菜单；头像打开登录者、所有者的行，以及退出登录。",
    ),
    (
        "Someone at a glance, and their profile to edit: Save waits for a change, a name and an email that reads; Discard puts the saved profile back.",
        "一眼看到某人，及其可编辑的资料：Save 等待有改动、名字合法且邮箱可读；Discard 恢复已保存的资料。",
    ),
    (
        "Where the account is signed in, this device first, each other one signing out on its own or all at once after a second press. API keys show their ends only; a new key's secret shows once.",
        "账户登录的位置，本机在前；其他设备可单独登出，或按两次一起登出。API 密钥只显示结尾；新密钥的机密只显示一次。",
    ),
    (
        "The plan with where it stands and what comes next; Cancel asks twice and Keep plan takes it back. Each limit's bar turns amber near it and red at it. A card saves once its number checks out, its month is ahead and its code is whole.",
        "当前套餐与后续安排；Cancel 二次确认，Keep plan 撤销。每项限额的条形接近时变黄、到达时变红。卡号合法、月份未过、码位完整时卡片才会保存。",
    ),
    (
        "Past bills with what each was for, its amount, date and state, and a download. A prompt past a limit stands as one line, or as a paywall with what the next plan brings.",
        "过去的账单及用途、金额、日期与状态，可下载。超限的提示或是一行字，或是付费墙并列下一计划的内容。",
    ),
    (
        "Members with a role to change, each role saying what it may do, and a Remove that asks twice; the owner stays. Invitations out resend or revoke, and an address with a role invites someone new.",
        "成员及其可改的角色，每个角色说明能做什么；Remove 二次确认，所有者不可移除。已发出的邀请可重发或撤销，填入地址并选角色即可邀请新人。",
    ),
    (
        "Sign in with a provider or with an email and a password. Sign in rests until the address reads and a password is typed; Enter in a field signs in, and a failed try says why above the button.",
        "用第三方或邮箱加密码登录。地址合法且密码已填时 Sign in 才可用；在字段里按 Enter 即登录，失败时按钮上方说明原因。",
    ),
    (
        "A name, an email and a new password, whose rules tick as they hold. Create account waits for every rule and the terms.",
        "名字、邮箱与新密码，规则满足一条勾一条。Create account 等全部规则与条款勾选。",
    ),
    (
        "A link by email: one that signs in, one that sets a new password. Once sent, each says where it went and offers it again.",
        "邮件链接：一个用于登录，一个用于设置新密码。发送后各自显示发往哪里，并可重发。",
    ),
    // ---- 37 Onboarding & Help: section notes ----
    (
        "A first run in a few steps: where it stands over a bar in as many parts, each step's title and line, and Next once its fields are ready. The welcome tour that follows is overlays::Tour.",
        "几步走完的首次运行：进度条分成几段显示位置，每步有标题与一行说明，字段就绪后 Next 可用。随后的欢迎导览是 overlays::Tour。",
    ),
    (
        "A hotspot's rings call out something new, and a press or Enter opens its tip. A highlight names a feature with a New badge over it; the badge alone marks a new row. Coachmarks are overlays::Spotlight.",
        "热点的圆环标出新东西，点按或 Enter 打开提示。高亮用一个 New 徽标点出功能；仅凭徽标即可标记新行。引导遮罩是 overlays::Spotlight。",
    ),
    (
        "First things to do, counted in words and a bar. Start opens each; here it marks it done, and the check eases in. Once all are done, Hide puts it away.",
        "该先做的事，用文字与进度条计数。Start 打开各项；完成后在这里标记，对勾淡入。全部完成时 Hide 收走。",
    ),
    (
        "A help mark beside a name shows its tip on hover; FormLabel's help is one. A line of help sits where it is needed, and Learn more opens the article in the panel below. An empty state with an action is feedback::EmptyState.",
        "名字旁的帮助标记悬停显示提示；FormLabel 的 help 是其中之一。帮助文字就放在需要的地方，Learn more 在下方面板打开文章。带动作的空态是 feedback::EmptyState。",
    ),
    (
        "Help beside the work: a search over the articles, one article with the way back, and links at its foot. A link to help:<key> opens another article.",
        "工作旁的帮助：搜索文章、阅读一篇并可返回，底部还有链接。help:<key> 形式的链接会打开另一篇文章。",
    ),
    (
        "Every shortcut in one dialog, its search taking focus as it opens; and what changed, newest first.",
        "全部快捷键集中在一个对话框，打开即聚焦搜索；以及更新内容，最新在前。",
    ),
    (
        "Four faces and a note, sent with one press; and a message to people, sent once it has a topic and words.",
        "四张表情加一句留言，一次点按发出；以及发给人们的消息，有主题与正文即可发送。",
    ),
    // ---- 25 Generative: section notes ----
    (
        "A picture to repaint in part: paint a mask where it should change, erase where it went too far; the mask washes over the picture. On the canvas, [ and ] size the brush and X swaps painting for erasing. Undo takes the last stroke, Clear takes them all.",
        "重绘图片的局部：在要改的地方涂蒙版，涂多了就擦掉；蒙版覆盖在图上。画布上 [ 与 ] 调笔刷大小，X 切换画与擦。Undo 撤最后一笔，Clear 全清。",
    ),
    (
        "Samples of a dataset: a prompt, its answer, its split and its length. Sort by any column, page through, and open a sample to read it whole.",
        "数据集的样本：提示、回答、所属划分与长度。按任意列排序、翻页，点开样本可读全文。",
    ),
    (
        "Notes placed by what they mean, colored by topic, flat or turned in three dimensions. Drag the turned one, or use the arrow keys. The pointer names a note; the legend hides a topic.",
        "按语义摆放的便签，按主题着色，平面或三维翻转。拖动翻转的那张，或用方向键。指针自报便签，图例可隐藏某主题。",
    ),
    (
        "A prompt to try. Each {name} in the template gets a field, and the prompt below marks what is still blank. Run waits for every value, then shows the answer with its tokens and time.",
        "试运行一段提示。模板里每个 {name} 都有输入位，下方的提示词标出还没填的。Run 等全部填好，然后给出回答及其 token 数与耗时。",
    ),
    (
        "A prompt's versions, newest first, and what the chosen one changed: words taken out struck through, words put in washed green. Restore brings an older one back as the newest.",
        "提示词的版本，最新在前，以及所选版本改了什么：删掉的划线、新增的淡绿。Restore 把旧版取回作为最新。",
    ),
    (
        "A model to choose: its maker, what it does, facts such as its size and license, and what it can do. The one in use says so; the others offer Use.",
        "可选择的模型：制作者、用途、参数量与许可等事实，以及它能做什么。在用的会说明，其余提供 Use。",
    ),
    (
        "Models on their way to this machine: how much of how much, how fast and how long is left, with Pause and Resume, Cancel, Retry, and Delete once a model is here.",
        "正在下载到本机的模型：进度、速度与剩余时间，带 Pause 与 Resume、Cancel、Retry，模型到位后是 Delete。",
    ),
    (
        "Models on this machine, loaded, loading or not, with the GPU memory each holds; and the machine at work, each part's meter turning amber then red as it fills, beside its recent history.",
        "本机的模型：已加载、加载中或未加载，各占多少显存；以及机器的工作状态——每项仪表随占用变黄再变红，旁边是近期历史。",
    ),
    (
        "A fine-tuning job: the base model and data it learns from, where it stands, how far it has come by epoch, and its loss on training and held-out data.",
        "微调任务：基础模型与训练数据、所处状态、按 epoch 的进度，以及训练集与验证集上的损失。",
    ),
    (
        "A picture's settings in one form: the prompt, which Enhance offers to fill out, the words it adds washed; what to leave out; a style; a shape; and a seed, empty for a new one each run. Cmd-Enter generates.",
        "一张图的设置集中在表单里：提示词（Enhance 可以代填，补的词淡化标出）、排除项、风格、形状，以及种子——留空则每次都新的。Cmd-Enter 生成。",
    ),
    (
        "Generations in line: each shows its picture or where it stands, its prompt, and how far it has come. Cancel while it waits or runs, Retry once it fails, Remove once it ends; a retried job folds back in at the end.",
        "排成一列的生成任务：各自显示图片或进度、提示词与进行到哪。等待或运行时可 Cancel，失败后 Retry，结束后 Remove；重试的任务排回末尾。",
    ),
    (
        "A run's results in its shape: a breathing placeholder with its percent while one comes, the picture once done, or why it failed with Retry. Beside it, an upscale against its source; drag the divider or use Left and Right.",
        "按形状排开的运行结果：未出图时是带百分比的呼吸占位，完成后是图片，失败则说明原因并带 Retry。旁边是与原图对照的放大版；拖分隔线或用 Left 与 Right。",
    ),
    (
        "Variations of one picture, the chosen one large; vary it a little or a lot, or upscale it. Then two models' answers side by side, blind until you pick the better or call a tie.",
        "同一张图的多个变体，选中的一大；可轻微或大幅变化，或放大。然后是两个模型的回答并排，选出更好的或判平局之前不显示是谁。",
    ),
    (
        "A generated sound: its prompt, model and length, a bar while it is made, then a player whose waveform lights as it plays. A press on the waveform seeks, and so do Left and Right once it has focus.",
        "生成的声音：提示词、模型与时长，生成中的进度条，然后是播放时波形点亮的播放器。点波形定位，聚焦后 Left 与 Right 亦可。",
    ),
    (
        "Voices to speak with: play a sample, then choose one by its row. The chosen voice takes the active wash and a check; tags drop below the name when narrow.",
        "可用的声音：先播样例，再按行选择。选中的声音带高亮与对勾；空间不足时标签落到名字下方。",
    ),
    (
        "A video made shot by shot along its time: each shot as wide as it runs, its poster once made, a placeholder while it comes. Choose a shot to read it; Extend asks for the next.",
        "沿时间线逐镜头生成的视频：每个镜头占其时长那么宽，生成完显示海报，未完成时是占位。选中镜头可读它；Extend 请求下一段。",
    ),
    (
        "Who else is here, ringed in their colors, their carets and selections live in the text. Rest on someone to see where they are; press to follow their view, and Escape to stop.",
        "还有谁在场，以各自的颜色圈出，光标与选区实时显示在文字里。悬停某人看他在哪；点按跟随他的视图，Escape 停止。",
    ),
    (
        "Sharing in one place: add people by email, each chip marked until it is an address; choose what they may do; change or take away anyone's access; and choose who the link lets in.",
        "分享集中一处：按邮箱加人，每个小片在成为合法地址前都有标记；选择其权限；可改可收回任何人的访问；并选择链接放谁进来。",
    ),
    (
        "What people did in the document, newest first.",
        "人们在这份文档里做了什么，最新在前。",
    ),
    (
        "A conversation on the words it quotes: each comment with its author, time and reactions; a reply below, Enter to send; resolve above. Press a reaction to add yours or take it back, or pick another.",
        "针对所选文字的讨论：每条评论带作者、时间与回应；下方回复，Enter 发送；上方是 resolve。点回应即添加或收回自己的，也可另选一个。",
    ),
    (
        "A mark beside the words a thread is about, with its count; a press opens the thread in a bubble over the page.",
        "话题所针对的文字旁有标记与计数；点按在页面上方的气泡里打开话题。",
    ),
    (
        "Every conversation down a column, open or resolved; a press picks one, and the one picked takes replies.",
        "全部讨论列成一列，未解决或已解决；点按选中，被选中的那条约新回复。",
    ),
    (
        "Stretches people marked, washed in their colors; the ones with a note are underlined, and resting on one shows it. A press picks one. A plain mark with no author is typography::Highlight.",
        "人们标记的文字段，染上各自的颜色；带批注的加下划线，悬停显示内容。点按选中。没有作者 Pure 标记是 typography::Highlight。",
    ),
    (
        "Editing changes the words; suggesting keeps them and proposes the change, removed words struck and added ones underlined in the author's color, each to accept or reject; viewing only reads. Type in the field while suggesting.",
        "编辑模式直接改字；建议模式保留原文、提出修改——删除划线、新增按作者颜色加下划线，逐条接受或拒绝；查看模式只读。建议模式下也可在输入框打字。",
    ),
    (
        "What an agent would change, file by file: how much moves, the diff a press away, and a verdict each or for all; in a side panel the counts and verdicts wrap below. A command it ran is terminal::CommandBlock.",
        "智能体打算改什么，逐文件列出：改动量、一键查看 diff，逐个或全部裁决；侧栏里计数与裁决折行排在下面。它运行过的命令是 terminal::CommandBlock。",
    ),
    (
        "A way to stop an agent mid-task and pick it up again; and a question it waits on, with answers to pick or a line to write one. Once answered, the card keeps the answer.",
        "中途叫停智能体并随时接续的方式；以及它等待的问题——给出选项或写一行回答。回答后卡片会保留答案。",
    ),
    (
        "Tokens each turn spent, split into what the model read, what it wrote and what came from the cache; and what the session cost, part by part.",
        "每轮花费的 token：模型读了多少、写了多少、多少来自缓存；以及会话成本，逐项列出。",
    ),
    (
        "A run as spans on a track for each depth: the session, its turns, and the calls in each; the wheel zooms and a press picks a span. Then how an agent did on a set of cases, sortable by any column.",
        "一次运行按深度铺成轨道上的区段：会话、其中的轮次、轮次里的调用；滚轮缩放，点按选中区段。之后是智能体在一组用例上的表现，可按任意列排序。",
    ),
    (
        "Where an agent's sandbox stands: a dot and a word, what it uses of what it has, and how long it has run. A dot rings while it starts or works.",
        "智能体沙箱的状态：一个点加一个词，已用多少配额、跑了多久。启动或工作中圆点会晃动。",
    ),
    (
        "Servers and connectors the agent can reach, each with where it stands and a switch; and every tool they bring, found by name, each allowed, asked for first, or denied.",
        "智能体可达的服务器与连接器，各带状态与开关；以及它们带来的全部工具，按名字查找，每个都是允许、先询问或拒绝。",
    ),
    (
        "What an agent keeps between conversations, a line to add to it and a way to forget; and the points it can go back to, newest first, the one it stands on marked.",
        "智能体在会话之间记住了什么：可加一条，也可忘记；以及可以回退的检查点，最新在前，当前所站在的那个有标记。",
    ),
    (
        "What an agent sees as it works, in frames from the host: a browser with its address and the frames so far, a press on one shows it; and a screen with where the agent points, a ring for each new point, and what it does there.",
        "智能体工作时看到的画面，来自宿主的帧：浏览器及其地址、到目前为止的帧，点一帧即可查看；还有屏幕画面与智能体指向的位置——每个新位置一个圆环，以及它在做什么。",
    ),
    (
        "What an agent made, in a panel of its own: the version shown among all, copy and download, the artifact as preview or source; the version switcher is chat::BranchNavigator. A live preview holds the host's page with its address and reload.",
        "智能体做出的东西，有专门的面板：在全部版本中显示当前版本、复制与下载、产物的预览或源码；版本切换器是 chat::BranchNavigator。实时预览承载宿主页面，带地址与刷新。",
    ),
    (
        "An agent's work as it goes: its steps down a rail, its plan ticking off as each item finishes, where it stands in a word, and how far it has come. The button finishes the next item.",
        "智能体的进行时：步骤沿一条轨排开，计划随各项完成逐条打勾，一个词说明现状，以及完成度。按钮会完成下一项。",
    ),
    (
        "Agents an agent sent out, and theirs in turn, each with where it stands: lists::Tree, a node per agent with its state as its note, a failed one tinted.",
        "智能体派出的子智能体及其子子智能体，各带状态：lists::Tree，每个智能体一个节点、状态写在节点上，失败的有颜色。",
    ),
    (
        "Each call an assistant makes: a mark for where it stands, the tool, what it touched and how long it took; a press opens its arguments and what came back. Several calls fold under one line.",
        "助手的每一次调用：状态标记、工具名、碰了什么、耗时多久；点按展开参数与返回。多个调用折在一行之下。",
    ),
    (
        "Asking first: a dialog before one call runs, with its arguments, where Escape denies; and a prompt in place that allows once, always, or not at all.",
        "先询问：调用运行前的对话框，带参数，Escape 即拒绝；以及就地弹出的提示——允许一次、始终允许或拒绝。",
    ),
    (
        "Code in a message: its language, copy, and the host's run, apply and download. Long code folds to its first lines until opened.",
        "消息里的代码：语言、复制，以及宿主的运行、应用与下载。长代码默认折到前几行，点开才展开。",
    ),
    (
        "Math is typography::Latex and tables are the markdown renderer's, in a message as in a document. A diagram arrives as the picture the host drew from its source.",
        "数学是 typography::Latex，表格来自 markdown 渲染器——消息里与文档里一致。图表以宿主从源码绘制的图片形式到达。",
    ),
    (
        "What a message can carry: a picture in its own shape, pictures as a grid, a file, a voice note to play and scrub, a video over the host's frames, and a link unfolded.",
        "消息能带什么：按原比例的图片、成网格的多图、文件、可播放可拖动的语音、叠在宿主帧上的视频，以及展开的链接。",
    ),
    (
        "Where an answer's words came from: a numbered pill after the claim, its source a rest of the pointer away; the sources folded under the answer, each a card.",
        "回答的话出自哪里：论断后面的编号小片，悬停片刻即见来源；来源折叠在回答下方，每条一张卡片。",
    ),
    (
        "A search as it runs, step by step; a result as the web gives it; and a passage found in a document, the words that matched washed.",
        "搜索的逐步过程；网页形式的结果；以及在文档里找到的段落，命中词淡化标注。",
    ),
    (
        "Where a message is written: attachments and context above the field, tools below, send at the end; Enter sends and Shift-Enter breaks the line. A slash at the start offers commands, an at sign offers context, and files dropped on it attach.",
        "写消息的地方：字段上方是附件与上下文，下方是工具，末尾发送；Enter 发送、Shift-Enter 换行。开头打斜杠出命令，打 at 出上下文，拖文件进来即附加。",
    ),
    (
        "The model is a forms::Select with a note on each; the mode a buttons::SegmentedControl; tools and saved prompts are menus::DropdownMenu, one with checks and one that inserts a prompt.",
        "模型是 forms::Select，每个选项带说明；模式是 buttons::SegmentedControl；工具与保存的提示是 menus::DropdownMenu——一个带勾选，一个插入提示词。",
    ),
    (
        "How answers are made: each knob a slider with its value; the instructions every answer follows, counted against their limit; how full the context is; and what a message may cost. The context meter is data_display::UsageBar.",
        "回答如何产生：每个旋钮是一个带数值的滑杆；每次回答遵循的指令（计入上限）；上下文有多满；以及一条消息可能的花费。上下文表是 data_display::UsageBar。",
    ),
    (
        "A quote to answer, the actions on a message, a sent message opened to change (the submit key saves, Escape cancels), the versions an answer has, and the buttons that regenerate, stop and continue.",
        "引用作答、消息上的操作、点开已发消息再修改（提交键保存、Escape 取消）、回答的多个版本，以及重新生成、停止与继续的按钮。",
    ),
    (
        "An answer that failed and a way to try again; a limit reached, counting down to when it lifts; and after a bad mark, what went wrong.",
        "失败的回答与重试入口；到达限额，倒计到解除；以及打了差评后，哪里出了问题。",
    ),
    (
        "The reasoning behind an answer, folded under how long it took; open, its steps read in a quiet column. While the assistant works, a word says what it is doing.",
        "回答背后的推理，按耗时折叠；展开后步骤在安静的一列里呈现。助手工作时，一个词说明它在做什么。",
    ),
    (
        "Conversations down a sidebar, pinned first, then by day; the one open marked with its menu, the others show theirs on hover or focus. The search is a forms::SearchInput; the list keeps titles that hold every word.",
        "侧栏里的会话，置顶在前、再按日期；打开的那个带菜单，其余悬停或聚焦时显示。搜索是 forms::SearchInput；列表保留包含全部关键词的标题。",
    ),
    (
        "Projects that hold conversations, each with how many; and what a project knows beyond the conversation, how full that knowledge is, with a way to add more.",
        "装着会话的项目，各带会话数；以及项目在会话之外知道的东西、这些知识的用量，还有添加的入口。",
    ),
    (
        "A conversation someone shared, to read and carry on; and saving one to a file, in a format, with or without its thinking.",
        "别人分享的会话，可以阅读并继续；以及把会话存成文件——选格式、带或不带推理过程。",
    ),
    (
        "A conversation held at its newest message and drawn only near the view. Yours sit on the right in a quiet bubble; the assistant's read as prose beside its mark. Scroll up and it stays put, with a way back down that counts what arrived.",
        "会话锚定在最新消息、只绘制视口附近。你自己的消息在右侧安静的气泡里；助手的以正文形式排在标记旁。向上滚时它保持不动，并有一个按钮计数新到的内容、带你回去。",
    ),
    (
        "Text as it arrives: what came in shows at a steady pace and catches up when far behind, a caret where more will come; markdown takes shape as its lines land. The cursor alone waits for the first words.",
        "流式到达的文字：已到的以稳定速度显示、落后太多会追赶，还将出现的位置有光标；markdown 随行落地逐渐成形。只有光标在等第一句话。",
    ),
    (
        "The first thing a new conversation shows: a greeting for the time of day, the composer, prompts to start from and what the assistant can do; a press fills the composer.",
        "新会话的第一屏：按时间问候、输入区、可起步的提示与助手能做什么；点一下即填入输入区。",
    ),
    (
        "Questions to ask next, under an answer; a press asks one.",
        "回答下方是接下来可以问的问题；点一下即发出。",
    ),
    (
        "Saved prompts, found by name and kept by category, the one in view shown whole: documents::TemplatePicker over prompts.",
        "保存的提示词，按名字查找、按分类归档，当前选中的完整展示：prompts 之上的 documents::TemplatePicker。",
    ),
    (
        "A poll turns to results once the owner holds the vote, with Change vote on the same stop. A survey asks on one page and sends when every question has its answer.",
        "发起者收票后投票即变为结果，同一位置出现 Change vote。问卷在一页里提问，全部作答后发送。",
    ),
    (
        "A quiz checks each answer and shows the right one, then the score. Flashcards turn on a press and go round the deck.",
        "测验逐题判定并显示正确答案，最后给分。卡片点按翻面，可循环整副。",
    ),
    (
        "A camera's view with a square to aim at, reading QR codes off the main thread from frames the host hands in. gpui has no camera, so the gallery hands in one frame of its own.",
        "摄像头画面配一个瞄准框，从宿主送入的帧里在主线程之外读二维码。gpui 没有摄像头，所以画廊送入了自己的一帧。",
    ),
    (
        "A picture of characters to copy, from the owner, who checks the answer and says how it stood; the refresh asks for another picture.",
        "要复制字符的图片，来自宿主；宿主校验回答并给出对错，刷新会要一张新图。",
    ),
    (
        "Each has one home, shown on its own chapter's page: BarcodeDisplay is data_display::Barcode, Signature is forms::SignaturePad, and Rating Widget is forms::Rating.",
        "每个组件都住在自己章节的页面里：BarcodeDisplay 是 data_display::Barcode，Signature 是 forms::SignaturePad，Rating Widget 是 forms::Rating。",
    ),
    (
        "The time in a zone, a face over its digits, ticking each second. A world clock lays places in rows: how each one's day and hour stand against home.",
        "某时区的时间，数字上盖一个表盘，每秒走动。世界时钟把城市排成行：各自的日期与时刻相对本地如何。",
    ),
    (
        "Time that runs while started and holds while paused, in tenths. Lap marks the time so far; once paused, it reads Reset and clears it.",
        "启动即走、暂停即停，精确到十分之一秒。Lap 记下目前的时间；暂停后按钮变成 Reset 并清零。",
    ),
    (
        "Digits, the four operations, sign, delete and clear, worked in precedence; keys type too, and Enter gives the result.",
        "数字、四则运算、正负号、删除与清空，按优先级计算；键盘也能输入，Enter 出结果。",
    ),
    (
        "An amount read in another unit: length, area, volume, mass, temperature, speed and data, with a swap.",
        "一个数量换算成别的单位：长度、面积、体积、质量、温度、速度与数据量，可互换。",
    ),
    (
        "ChangeLog Viewer is data_display::Changelog, shown on the Data Display page.",
        "ChangeLog Viewer 是 data_display::Changelog，显示在 Data Display 页。",
    ),
    (
        "The packages a product ships, each with its license, beside the chosen one's text. These are Ely's own.",
        "产品附带的所有包及其许可，旁边是所选许可的原文。这些是 Ely 自己的。",
    ),
    (
        "Terms to agree to before going on, and cookies to choose when the app shows web content. Each hands its answer to the owner.",
        "继续之前需要同意的条款，以及应用展示网页内容时的 cookie 选择。两者都把结果交给所有者。",
    ),
    (
        "Data out in a format the owner writes, and data in: columns mapped to the app's fields, matched by name to start, and a CSV file read into records through the same mapping.",
        "按所有者写的格式导出数据，以及导入：列映射到应用的字段——初始按名字对上——同一映射再把 CSV 文件读成记录。",
    ),
    (
        "A native web view laid over its box: an address, or HTML the owner hands in, which makes it an HTML preview. It draws above everything gpui paints and follows the box each frame.",
        "叠在盒子上的原生网页视图：给地址，或由所有者传入 HTML——那就成了 HTML 预览。它画在 gpui 所有内容之上，并逐帧跟随盒子。",
    ),
    (
        "A page from another address in a frame of its own, with Open in browser. EmbedCard is chat::LinkPreviewCard, shown on the AI Chat page.",
        "来自其他地址的页面装在自己的框里，带 Open in browser。EmbedCard 是 chat::LinkPreviewCard，显示在 AI Chat 页。",
    ),
    (
        "ShareSheet is buttons::ShareButton, shown on the Buttons & Actions page.",
        "ShareSheet 是 buttons::ShareButton，显示在 Buttons & Actions 页。",
    ),
    ("A box to pick", "一个可供选择的框"),
    (
        "A component library for gpui, in light and dark.",
        "一个 gpui 组件库，明暗两版。",
    ),
    (
        "A lift blends a color toward white, in gamma space.",
        "在 gamma 空间把颜色向白色提亮。",
    ),
    ("A line about you", "一行关于你的介绍"),
    ("A message to send", "要发送的消息"),
    ("A word, then Enter", "输入一个词，然后回车"),
    ("Aa 12", "Aa 12"),
    ("Ada Lovelace", "Ada Lovelace"),
    ("Add a tag", "添加标签"),
    ("Add an expression", "添加表达式"),
    ("Add one", "添加一个"),
    ("Add payment method", "添加支付方式"),
    (
        "Agreed. I'll move them on the plan.",
        "同意，我会把它们排进计划。",
    ),
    ("All notifications", "全部通知"),
    ("Any element fits here.", "任何元素都能放这里。"),
    ("Any label", "任意标签"),
    (
        "Anyone with the link can view it.",
        "任何有链接的人都可查看。",
    ),
    ("Archive", "归档"),
    (
        "Ask anything, / for commands, @ for context",
        "问任何问题，/ 命令，@ 上下文",
    ),
    ("Assigned to", "负责人"),
    ("Back to Ely's theme", "回到 Ely 主题"),
    ("Backdrop", "遮罩"),
    ("Badge 3", "徽标 3"),
    ("Blend a color toward white.", "把颜色向白色提亮。"),
    ("Bold", "粗体"),
    (
        "By the south windows, where the stair takes its light.",
        "在南窗边，楼梯把光带进来的地方。",
    ),
    ("Cancel", "取消"),
    ("Celebrate", "庆祝"),
    ("Center", "居中"),
    ("Clear", "清空"),
    ("Clear search", "清除搜索"),
    ("Clearable", "可清空"),
    ("Click anywhere else.", "点击其他任意位置。"),
    ("Click outside to close.", "点击外部关闭。"),
    ("Close", "关闭"),
    ("Close anyway", "仍然关闭"),
    ("Color-blind safe charts", "色盲安全图表"),
    ("Command palette", "命令面板"),
    ("Compose", "撰写"),
    ("Copy", "复制"),
    ("Copy CSV", "复制 CSV"),
    ("Copy SVG", "复制 SVG"),
    ("Copy link", "复制链接"),
    ("Copy “Ely”", "复制“Ely”"),
    ("Country", "国家"),
    ("Currency", "货币"),
    ("Dark themes lift further.", "深色主题下提亮更多。"),
    ("Day", "日"),
    ("Delete", "删除"),
    (
        "Deleting a workspace removes its files for everyone.",
        "删除工作区会对所有人移除其文件。",
    ),
    ("Describe a picture", "描述一张图片"),
    ("Design · London", "设计 · 伦敦"),
    ("Discard", "丢弃"),
    ("Done", "完成"),
    ("Double-click to zoom this window", "双击缩放此窗口"),
    ("Edit", "编辑"),
    (
        "Ely's trigger waits 400ms by default; gpui's waits 500ms.",
        "Ely 的触发器默认等 400ms；gpui 的是 500ms。",
    ),
    (
        "Every color, size and radius comes from one token set.",
        "每种颜色、尺寸与圆角都来自同一套令牌。",
    ),
    ("Export", "导出"),
    ("Export…", "导出…"),
    ("Filter addresses", "筛选地址"),
    ("Find", "查找"),
    ("Find a key or a value", "查找键或值"),
    ("Find a place", "查找地点"),
    ("Find a query", "查找查询"),
    ("Find a tool", "查找工具"),
    ("Find in the answer", "在回答中查找"),
    ("Finish the next item", "完成下一项"),
    ("Focus mode", "专注模式"),
    ("Focus on the text", "聚焦到文字"),
    ("Focused as it appears", "出现即聚焦"),
    ("Follow", "跟随"),
    ("Font", "字体"),
    ("Forward", "转发"),
    ("Full name", "全名"),
    ("Generate", "生成"),
    ("Go home", "回到首页"),
    ("Go to file", "转到文件"),
    ("Go to symbol…", "转到符号…"),
    ("Handbook / Color / Lift", "手册 / 颜色 / 提亮"),
    ("Here", "在这里"),
    ("Here while true.", "为真时在这里。"),
    ("Host", "宿主"),
    ("How can I help?", "需要我做什么？"),
    (
        "I'll look before the review. Is the tadelakt sealed yet?",
        "评审前我会去看。泰德拉克涂料封好了吗？",
    ),
    (
        "In dark, accents lift further so they read with the same weight.",
        "深色下强调色进一步提亮，视觉分量保持一致。",
    ),
    ("Inspect", "检查"),
    ("Invite", "邀请"),
    ("Italic", "斜体"),
    ("Keyboard shortcuts", "键盘快捷键"),
    ("Language", "语言"),
    ("Large", "大"),
    ("Later", "稍后"),
    (
        "Layouts saved by a newer version are refused, not guessed.",
        "更新版本保存的布局会被拒绝而不是猜测。",
    ),
    ("Left", "左"),
    ("Lift", "提亮"),
    ("Line or line:column", "行 或 行:列"),
    ("Live", "直播中"),
    ("Ln 12, Col 4", "第 12 行，第 4 列"),
    ("Manage", "管理"),
    ("Managed by your admin", "由管理员管理"),
    ("Mark as read", "标为已读"),
    ("Marker", "标记"),
    ("Measure", "测量"),
    ("Message", "消息"),
    ("Month", "月"),
    (
        "Morning. The plaster samples came in overnight.",
        "早。石膏样本昨晚到了。",
    ),
    ("Move", "移动"),
    ("Move across me", "从我上面划过"),
    (
        "Move, press or scroll here, or type in the field.",
        "在这里移动、点按或滚动，也可以在输入框打字。",
    ),
    ("Mute", "静音"),
    ("New data", "新数据"),
    ("New file", "新建文件"),
    ("New item", "新条目"),
    ("New name", "新名字"),
    ("New project", "新项目"),
    ("Next", "下一步"),
    ("Next action", "下一个动作"),
    ("Next line", "下一行"),
    ("Next reading", "下一次阅读"),
    ("Next status", "下一个状态"),
    ("Next week", "下周"),
    ("No notifications", "没有通知"),
    ("Not yet. It needs another day.", "还不行，需要再放一天。"),
    ("Note", "备注"),
    (
        "Notes grow from three lines to six, then scroll.",
        "备注从三行长到六行，之后滚动。",
    ),
    ("One more", "再来一个"),
    ("Open a frosted window", "打开毛玻璃窗口"),
    ("Open crash window", "打开崩溃窗口"),
    ("Open drawer", "打开抽屉"),
    ("Open panel", "打开面板"),
    ("Open sheet", "打开侧板"),
    ("Open the archive", "打开归档"),
    ("Open the launcher", "打开启动器"),
    ("Open the thread", "打开话题"),
    ("Open update window", "打开更新窗口"),
    ("Or write an answer", "或自己写回答"),
    (
        "Painted last, above the clipped box.",
        "最后绘制，盖在裁剪框之上。",
    ),
    ("Password", "密码"),
    ("Paste", "粘贴"),
    ("Pattern, such as user:*", "模式，如 user:*"),
    ("Photo options", "照片选项"),
    ("Pick cities", "选择城市"),
    ("Pick “Open gallery”", "选择“打开画廊”"),
    ("Pinned", "已置顶"),
    (
        "Pinned the brief: light from above, one stair, olive trees by the windows. Thoughts on where the trees go?",
        "需求已定：顶部采光、一段楼梯、窗边的橄榄树。树放哪里好？",
    ),
    ("Plain text, as a child.", "作为子元素的纯文本。"),
    (
        "Plain words: you own your work, and we keep it private.",
        "简单说：作品归你，我们为你保密。",
    ),
    ("Planned", "已排期"),
    ("Press anywhere", "点按任意处"),
    ("Previous", "上一个"),
    ("Public link", "公开链接"),
    ("Quarterly review", "季度评审"),
    ("Quiet interfaces age well.", "安静的界面经得起用。"),
    ("Read the", "阅读"),
    ("Read the guide", "阅读指南"),
    ("Redo", "重做"),
    ("Redraw this view", "重绘此视图"),
    ("Refresh", "刷新"),
    ("Refreshing prices…", "正在刷新报价…"),
    (
        "Release builds leave gpui's inspector out.",
        "release 构建不含 gpui 的检查器。",
    ),
    ("Reload", "重新加载"),
    ("Reload and fail", "重载并失败"),
    ("Rename", "重命名"),
    ("Rename lookup", "重命名查找"),
    ("Replay", "回放"),
    ("Replay the stream", "回放流"),
    ("Reply", "回复"),
    ("Reply…", "回复…"),
    ("Request access", "申请访问"),
    ("Reset", "复位"),
    ("Restart", "重启"),
    ("Restore", "还原"),
    ("Retention", "留存"),
    (
        "Revenue rose 12% on the quarter. Churn held at 2.1%; three new markets opened.",
        "季度营收增长 12%，流失率稳定在 2.1%，新开了三个市场。",
    ),
    ("Revenue, third quarter: 1.2 M", "第三季度营收：120 万"),
    ("Review", "评审"),
    ("Review the call…", "回顾这通电话…"),
    ("Review the terms", "查看条款"),
    ("Rich content", "富内容"),
    ("Right", "右"),
    ("Right-click anywhere here", "在此处任意右键"),
    ("Right-click for the ring", "右键打开圆环"),
    ("Ring", "圆环"),
    ("Run", "运行"),
    ("Save", "保存"),
    ("Save changes", "保存修改"),
    ("Save layout", "保存布局"),
    ("Search cities", "搜索城市"),
    ("Search conversations", "搜索会话"),
    ("Search docs", "搜索文档"),
    ("Search everything", "搜索全部"),
    ("Search fonts", "搜索字体"),
    ("Search history…", "搜索历史…"),
    ("Search symbols", "搜索符号"),
    ("Search zones", "搜索时区"),
    ("Searching the web", "正在搜索网页"),
    ("Select one more", "再选一个"),
    ("Send more", "多发几条"),
    ("Sent. We answer within a day.", "已发送，我们一天内回复。"),
    ("Share", "分享"),
    ("Share this page", "分享此页"),
    ("Show a coachmark", "显示引导遮罩"),
    ("Show again", "再次显示"),
    ("Show backdrop", "显示遮罩"),
    ("Show help", "显示帮助"),
    ("Show pages", "显示页面"),
    ("Show the banner", "显示横幅"),
    ("Show the checklist", "显示待办清单"),
    ("Show the closed alert", "显示已关提示"),
    ("Shuffle", "随机"),
    ("Sign in", "登录"),
    ("Small", "小"),
    ("Something happens", "有事情发生"),
    ("Something to keep", "值得保留的东西"),
    ("Sort", "排序"),
    ("Start over", "重新开始"),
    ("Step", "单步"),
    ("Subscribe", "订阅"),
    ("Switch", "切换"),
    ("Switch branch", "切换分支"),
    ("Switch branch…", "切换分支…"),
    ("Switch project…", "切换项目…"),
    ("Switch windows", "切换窗口"),
    ("Sync over cellular", "用移动网络同步"),
    ("Synced", "已同步"),
    (
        "Tab walks every control on a page; Shift-Tab walks back.",
        "Tab 走遍页面上的控件，Shift-Tab 反向。",
    ),
    ("Take one", "拿一个"),
    ("Take the tour", "看看导览"),
    (
        "Text wraps to the width it gets. The box reports each new size once.",
        "文字按得到的宽度换行，盒子每次新尺寸只上报一次。",
    ),
    ("The main part starts here.", "主内容从这里开始。"),
    ("The owner's icons", "所有者的图标"),
    (
        "The pick reaches the owner as it moves; the log shows it.",
        "选择随移动实时到达所有者，日志会展示。",
    ),
    (
        "The sidebar lets the desktop through; this pane stays solid.",
        "侧栏透出桌面，这一块保持实底。",
    ),
    (
        "Themes cross-fade in 280 ms. The theme's reduced motion settles entrances, folds and fades at once; an undo line still counts real time.",
        "主题在 280ms 内交叉淡入。主题的减少动效会同时定住入场、折叠与淡出；撤销一行仍计真实时间。",
    ),
    (
        "They're in the stair hall, along the left wall.",
        "它们在楼梯厅，沿左墙排着。",
    ),
    (
        "Three finishes: lime, tadelakt and a sanded gypsum.",
        "三种饰面：石灰、泰德拉克与磨砂石膏。",
    ),
    ("Tick", "刻度"),
    ("Title", "标题"),
    ("Try again", "重试"),
    ("Try it", "试一试"),
    (
        "Two there, and one on the landing.",
        "那里有两个，平台上还有一个。",
    ),
    ("Type here", "在这里输入"),
    (
        "Type scale settled; spacing loosened by one step.",
        "字号定了，间距放宽一档。",
    ),
    ("UTF-8", "UTF-8"),
    ("Underline", "下划线"),
    ("Undo", "撤销"),
    ("View profile", "查看资料"),
    ("View receipt", "查看收据"),
    ("Waiting for approval", "等待批准"),
    ("Week", "周"),
    ("What's new", "新功能"),
    (
        "Where motion explains a change, and nowhere else.",
        "只在动效能说明变化的地方用，别处不用。",
    ),
    ("Wi-Fi", "Wi-Fi"),
    ("Write a message", "写一条消息"),
    (
        "Write a note. @ for people, # for tags.",
        "写条备注。@ 人名，# 标签。",
    ),
    (
        "Writes the first programs for machines not yet built.",
        "为尚未造出的机器写下最初的程序。",
    ),
    ("Yes", "是"),
    (
        "You left the huddle. Join it again from its mark.",
        "你已离开语音房，可从它的记号再次加入。",
    ),
    (
        "DockLook sets a floor; a panel's own look wins per field.",
        "DockLook 定的是底线，面板自己的外观逐字段覆盖它。",
    ),
    (
        "Border thickness, corner radius and background, per panel or for the whole dock; the divider's hit area and line are the dock's.",
        "每块面板可各自设置边框粗细、圆角与背景，也可整块 Dock 统一设置；分隔条的命中区与线宽属于 Dock。",
    ),
    ("1 px, radius 6", "1 px，圆角 6"),
    ("2 px, tinted", "2 px，带底色"),
    ("Gap and divider", "间隙与分隔条"),
];
