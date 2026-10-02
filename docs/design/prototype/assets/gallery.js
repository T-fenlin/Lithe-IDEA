/* =============================================================================
   组件画廊
   每个分组的尺寸与色值都取自 Figma 文档树 Components 页的实测值，注释写在
   分组标题下方。组件主题跟随 :root[data-theme]。
   ========================================================================== */
(function () {
  const svg = window.svg;

  function row(label, content) {
    return '<div class="grow"><span class="glabel">' + label + '</span>' + content + '</div>';
  }

  function buttonsCard() {
    return '<div class="gcard">' +
      '<h5>Button</h5>' +
      '<p class="gsub">高 28px · 圆角 6px · Primary 填充 #3574F0 · Slim 高 24px</p>' +
      row('Primary',
        '<span class="btn primary">Button</span>' +
        '<span class="btn primary hover">Hovered</span>' +
        '<span class="btn primary pressed">Pressed</span>' +
        '<span class="btn primary focus">Focused</span>' +
        '<span class="btn disabled">Disabled</span>') +
      row('Secondary',
        '<span class="btn secondary">Button</span>' +
        '<span class="btn secondary hover">Hovered</span>' +
        '<span class="btn secondary pressed">Pressed</span>' +
        '<span class="btn secondary focus">Focused</span>' +
        '<span class="btn disabled">Disabled</span>') +
      row('Slim 24px',
        '<span class="btn secondary slim">Button</span>' +
        '<span class="btn secondary slim hover">Hovered</span>' +
        '<span class="btn primary slim">Primary</span>') +
    '</div>';
  }

  function checkCard() {
    const chk = svg('check'), dash = svg('dash');
    function box(cls) {
      const mark = cls.indexOf('ind') >= 0 ? dash : (cls.indexOf('on') >= 0 ? chk : '');
      return '<span class="chkbox ' + cls + '">' + mark + '</span>';
    }
    return '<div class="gcard">' +
      '<h5>Checkbox / Radio / Toggle</h5>' +
      '<p class="gsub">控件 16×16 · 树行 24px · Toggle 28×16 · 选中 #3574F0 · 无效 #DB3B4B</p>' +
      row('Checkbox',
        box('') + '<span style="font-size:13px">Unchecked</span>' +
        box('hover') + '<span style="font-size:13px">Hovered</span>' +
        box('on') + '<span style="font-size:13px">Checked</span>' +
        box('ind') + '<span style="font-size:13px">Indeterminate</span>' +
        box('focus') + '<span style="font-size:13px">Focused</span>' +
        box('invalid') + '<span style="font-size:13px">Invalid</span>' +
        box('on disabled') + '<span style="font-size:13px">Disabled</span>') +
      row('Radio',
        '<span class="radio"></span><span style="font-size:13px">Off</span>' +
        '<span class="radio on"></span><span style="font-size:13px">On</span>' +
        '<span class="radio invalid"></span><span style="font-size:13px">Invalid</span>' +
        '<span class="radio on" style="opacity:.45"></span><span style="font-size:13px">Disabled</span>') +
      row('Toggle',
        '<span class="toggle on"></span><span style="font-size:13px">On</span>' +
        '<span class="toggle"></span><span style="font-size:13px">Off</span>') +
    '</div>';
  }

  function inputCard() {
    return '<div class="gcard">' +
      '<h5>Input / Search / Dropdown / Combobox</h5>' +
      '<p class="gsub">高 28px · 圆角 6px · 边框 1px · 聚焦描边 #3574F0</p>' +
      row('Input',
        '<input class="input" style="width:200px" value="commons-math3" readonly>' +
        '<input class="input" style="width:120px;opacity:.5" value="Disabled" readonly>') +
      row('Search',
        '<span style="display:inline-flex;align-items:center;gap:6px;width:220px;height:28px;' +
        'border:1px solid var(--accent);border-radius:6px;padding:0 8px">' + svg('search') +
        '<span style="font-size:13px">Analysis</span>' +
        '<span style="width:1px;height:14px;background:var(--accent)"></span></span>') +
      row('Dropdown',
        '<span class="btn secondary" style="min-width:140px;justify-content:space-between">' +
        'Maven ' + svg('dropdown') + '</span>') +
      row('Combobox',
        '<span class="btn secondary" style="min-width:140px;justify-content:space-between">' +
        'Gradle <span style="opacity:.6">▾</span></span>') +
    '</div>';
  }

  function tabsCard() {
    return '<div class="gcard">' +
      '<h5>Tabs / Segmented control / Link</h5>' +
      '<p class="gsub">Tab 高 41px · 激活态底部 3px #3574F0 · 非激活 opacity .67</p>' +
      '<div style="border-bottom:1px solid var(--border);margin-bottom:14px">' +
        '<div style="display:flex;align-items:stretch">' +
          '<div class="tab">Editor / Tab</div>' +
          '<div class="tab on">Active</div>' +
          '<div class="tab idle">Inactive</div>' +
        '</div>' +
      '</div>' +
      row('Segmented',
        '<span class="seg"><div class="on">Java</div><div>Kotlin</div><div>Groovy</div>' +
        '<div>JavaScript</div><div>Python</div></span>') +
      row('Link',
        '<span class="link">Default</span><span class="link">Dropdown ▾</span>' +
        '<span class="link">External ↗</span>' +
        '<span style="font-size:13px;color:var(--fg-dim)">Disabled</span>') +
    '</div>';
  }

  function barCard() {
    return '<div class="gcard">' +
      '<h5>Progress / Scrollbar / Loader / Shortcut</h5>' +
      '<p class="gsub">进度条高 4px · 圆角 2px · 滚动条滑块宽 6px</p>' +
      row('Determinate',
        '<span class="progress"><i></i></span>' +
        '<span class="progress" style="width:120px"><i style="width:28%"></i></span>') +
      row('Indeterminate', '<span class="progress"><i style="width:34%"></i></span>') +
      row('Scrollbar',
        '<span class="scrollbox">content<div class="sb"><i></i></div></span>') +
      row('Loader',
        '<span class="spinner"></span><span style="font-size:13px;color:var(--fg-dim)">Loader…</span>') +
      row('Shortcut',
        '<span class="kbd">⇧</span><span class="kbd">⌘</span><span class="kbd">A</span>' +
        '<span class="codechip">psvm</span><span class="codechip">sout</span>') +
    '</div>';
  }

  function msgCard() {
    return '<div class="gcard">' +
      '<h5>Notification / Alert / Banner / Tooltip</h5>' +
      '<p class="gsub">Notification 圆角 8px · Banner 高 40px · Tooltip 底色 #2B2D30（两种主题下都是）</p>' +
      '<div class="gcol">' +
        row('Notification',
          '<span class="notif"><span class="nt">Build completed</span>' +
          '<span class="nb">Project built in 3 s 214 ms</span></span>') +
        row('Alert',
          '<span class="alert" style="width:360px"><span class="ah">Build failed</span>' +
          '<span class="ab">Cannot resolve symbol \'MultivariateDifferentiableFunction\'</span></span>') +
        row('Banner',
          '<span class="banner" style="width:420px">' + svg('info') +
          ' New version 2026.2 is available <span style="margin-left:auto">Update</span></span>') +
        row('Tooltip', '<span class="tooltip">Shows the tree indent guides</span>') +
      '</div>' +
    '</div>';
  }

  function dataCard() {
    return '<div class="gcard">' +
      '<h5>Tree / Table / Popup cell</h5>' +
      '<p class="gsub">树行 24px · 缩进每级 18px · 选中 #D4E2FF 圆角 4px</p>' +
      '<div class="grow" style="align-items:flex-start">' +
        '<div><div class="glabel" style="margin-bottom:6px">Tree</div>' +
          '<div class="treebox">' +
            '<div class="row" style="padding-left:8px"><span class="sel"></span>' +
              '<span class="chev">' + svg('chevD') + '</span>' +
              '<span class="ic">' + svg('folder') + '</span><span class="lbl">src</span></div>' +
            '<div class="row on" style="padding-left:26px"><span class="sel"></span>' +
              '<span class="chev"></span><span class="ic">' + svg('class') + '</span>' +
              '<span class="lbl">FunctionUtils</span></div>' +
            '<div class="row" style="padding-left:26px"><span class="sel"></span>' +
              '<span class="chev"></span><span class="ic">' + svg('class') + '</span>' +
              '<span class="lbl">MultivariateFunction</span></div>' +
            '<div class="row" style="padding-left:26px"><span class="sel"></span>' +
              '<span class="chev"></span><span class="ic">' + svg('class') + '</span>' +
              '<span class="lbl">TrivariateFunction</span></div>' +
          '</div>' +
        '</div>' +
        '<div><div class="glabel" style="margin-bottom:6px">Table</div>' +
          '<div class="tablebox">' +
            '<div class="tr th"><div>Name</div><div>Type</div><div>Size</div></div>' +
            '<div class="tr"><div>FunctionUtils.java</div><div>Java</div><div>12 KB</div></div>' +
            '<div class="tr" style="background:var(--sel)"><div>analysis</div><div>Class</div><div>4 KB</div></div>' +
            '<div class="tr"><div>add-hover.svg</div><div>SVG</div><div>2 KB</div></div>' +
          '</div>' +
        '</div>' +
        '<div><div class="glabel" style="margin-bottom:6px">Popup cell</div>' +
          '<div class="treebox" style="width:260px">' +
            '<div class="row" style="padding-left:10px"><span class="sel"></span>' +
              '<span class="ic">' + svg('class') + '</span>' +
              '<span class="lbl">Go to Declaration</span><span class="dim">⌘B</span></div>' +
            '<div class="row" style="padding-left:10px"><span class="sel"></span>' +
              '<span class="ic">' + svg('search') + '</span>' +
              '<span class="lbl">Find Usages</span><span class="dim">⌥F7</span></div>' +
            '<div class="row" style="padding-left:10px"><span class="sel"></span>' +
              '<span class="ic">' + svg('commit') + '</span>' +
              '<span class="lbl">Show Git Log</span></div>' +
            '<div class="row" style="padding-left:10px"><span class="sel"></span>' +
              '<span class="ic">' + svg('problems') + '</span>' +
              '<span class="lbl">Inspect Code</span></div>' +
          '</div>' +
        '</div>' +
      '</div>' +
    '</div>';
  }

  window.GALLERY = {
    render: function () {
      return '<div class="gallery"><div class="gallery-wrap">' +
        '<h2>通用组件</h2>' +
        '<p class="note">尺寸与色值取自 Figma 文档树（fileKey FEAVhuAJ1Pa7R2HfsllRHY）。' +
        '主题跟随底部切换条或 URL 的 theme 参数。</p>' +
        buttonsCard() + checkCard() + inputCard() + tabsCard() +
        barCard() + msgCard() + dataCard() +
        '<h2>说明</h2>' +
        '<p class="note">这是按 Int UI Kit 还原的一次性原型，用来对齐视觉与交互密度，不是产品代码。' +
        '浅色主题关键字色 #0033B3 与暗色主题 #D69A6B 是渲染图像素采样值，不是推测值。</p>' +
      '</div></div>';
    }
  };
})();
