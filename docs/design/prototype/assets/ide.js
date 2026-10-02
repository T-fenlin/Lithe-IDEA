/* =============================================================================
   IDE 窗口 / 对话框 / 弹窗 / 欢迎屏 的渲染
   只负责拼 HTML，不带状态。主题与缩放由 shell.js 处理。
   ========================================================================== */
(function () {
  const svg = window.svg;
  const ICONS = window.ICONS;

  /* ---------- 主工具栏 ---------- */
  function toolbar() {
    return '<div class="toolbar">' +
      '<div class="gradient"></div>' +
      '<div class="traffic"><i></i><i></i><i></i></div>' +
      '<div class="brand"><span class="logo">IJ</span><span class="txt">IntelliJ</span>' +
        svg('dropdown') + '</div>' +
      '<div class="branch">' + svg('branch') + '<span>main</span>' + svg('dropdown') + '</div>' +
      '<div class="spacer"></div>' +
      '<div class="right">' +
        '<div class="item">' + svg('project') + '<span>IDEA Community</span>' + svg('dropdown') + '</div>' +
        '<div class="item" style="color:var(--success)">' + svg('play') + '</div>' +
        '<div class="item" style="color:var(--success)">' + svg('bug') + '</div>' +
        '<div class="item">' + svg('more') + '</div>' +
        '<div class="item">' + svg('user') + '</div>' +
        '<div class="item">' + svg('search') + '</div>' +
        '<div class="item">' + svg('settings') + '</div>' +
        '<div class="item">' + svg('hamburger') + '</div>' +
      '</div>' +
    '</div>';
  }

  /* ---------- 左右竖条 ---------- */
  function stripe(side) {
    if (side === 'right') {
      return '<div class="stripe right">' +
        '<div class="act on">' + svg('info') + '</div>' +
        '<div class="act">' + svg('services') + '</div>' +
        '<div class="act" style="color:var(--orange)">' + svg('markdown') + '</div>' +
      '</div>';
    }
    return '<div class="stripe">' +
      '<div class="act on">' + svg('project') + '</div>' +
      '<div class="act">' + svg('structure') + '</div>' +
      '<div class="act">' + svg('commit') + '</div>' +
      '<div class="act">' + svg('bookmark') + '</div>' +
      '<div class="act">' + svg('more') + '</div>' +
      '<div class="grow"></div>' +
      '<div class="act">' + svg('services') + '</div>' +
      '<div class="act">' + svg('play') + '</div>' +
      '<div class="act">' + svg('problems') + '</div>' +
    '</div>';
  }

  /* ---------- 项目工具窗 ---------- */
  function project() {
    const rows = window.TREE.map(function (n) {
      const isChev = n.ic.indexOf('chev') === 0;
      return '<div class="row lvl' + n.lvl + (n.sel ? ' on' : '') + '">' +
        '<span class="sel"></span>' +
        '<span class="chev">' + (isChev ? svg(n.ic) : '') + '</span>' +
        '<span class="ic">' + (isChev ? '' : svg(window.TREE_ICON[n.type] || 'folder')) + '</span>' +
        '<span class="lbl">' + n.label + '</span>' +
        (n.dim ? '<span class="dim">' + n.dim + '</span>' : '') +
      '</div>';
    }).join('');
    return '<div class="panel project">' +
      '<div class="panel-head">' +
        '<span class="title">Project</span><span class="spacer"></span>' +
        '<span class="act">' + svg('collapse') + '</span>' +
        '<span class="act">' + svg('chevD') + '</span>' +
        '<span class="act">' + svg('more') + '</span>' +
        '<span class="act">' + svg('minus') + '</span>' +
        '<span class="act">' + svg('close') + '</span>' +
      '</div>' +
      '<div class="panel-body"><div class="tree">' + rows + '</div></div>' +
    '</div>';
  }

  /* ---------- 编辑器 ---------- */
  function editor() {
    const FILE_ICON = {java:'fileJava', svg:'fileSvg'};
    const tabs = window.TABS.map(function (t) {
      const ic = t.icon ? '<span class="ticon">' + svg(FILE_ICON[t.icon] || 'class') + '</span>' : '';
      const style = t.w ? ' style="width:' + t.w + 'px"' : '';
      return '<div class="tab ' + (t.on ? 'on' : 'idle') + '"' + style + '>' + ic +
        '<span class="tname">' + t.name + '</span>' +
        (t.close ? '<span class="x">×</span>' : '') + '</div>';
    }).join('');
    const start = window.CODE_START, hl = window.CODE_HL;
    const gut = window.CODE.map(function (_, i) {
      const n = start + i;
      return '<div class="ln' + (n === hl ? ' hl' : '') + '">' + n + '</div>';
    }).join('');
    const src = window.CODE.map(function (row, i) {
      const n = start + i;
      return '<div class="ln' + (n === hl ? ' hl' : '') + '">' + (row[1] || '&nbsp;') + '</div>';
    }).join('');
    const marks = window.SCROLL_MARKS.map(function (m) {
      return '<i style="top:' + m.top + ';height:' + m.height + ';background:' + m.color + '"></i>';
    }).join('');
    /* 检查部件位于代码区右上角，不在标签行里 —— 这一点是从渲染图核对出来的 */
    const inspect = '<div class="inspect">' +
        '<span class="iw" style="color:var(--warn)">' + svg('warning') + ' 3</span>' +
        '<span class="iw" style="color:var(--success)">' + svg('check') + ' 10</span>' +
        '<span class="iw">' + svg('chevD') + '</span>' +
        '<span class="iw">' + svg('chevD') + '</span>' +
      '</div>';
    return '<div class="editor">' +
      '<div class="tabs">' + tabs + '<span class="spacer"></span>' +
        '<div class="acts">' + svg('chevD') + svg('more') + '</div>' +
      '</div>' +
      '<div class="code">' +
        inspect +
        '<div class="gut">' + gut + '</div>' +
        '<div class="src">' + src + '</div>' +
        '<div class="scrollbar">' + marks + '</div>' +
      '</div>' +
    '</div>';
  }

  /* ---------- 终端 ---------- */
  function terminal() {
    function col(items) {
      return '<div>' + items.map(function (n) { return '<div>' + n + '</div>'; }).join('') + '</div>';
    }
    const split = window.TERM_SPLIT;
    const cols = col(window.TERM_LS.slice(0, split)) + col(window.TERM_LS.slice(split));
    return '<div class="panel terminal">' +
      '<div class="panel-head">' +
        '<span class="tab">Terminal</span>' +
        '<span class="tab on">Local <span class="x">×</span></span>' +
        '<span class="tab">Local (1) <span class="x">×</span></span>' +
        '<span class="tab" style="color:var(--icon)">+</span>' +
        '<span class="spacer"></span>' +
        '<span class="act">' + svg('more') + '</span>' +
        '<span class="act">' + svg('minus') + '</span>' +
      '</div>' +
      '<div class="panel-body"><div class="term">alina.mishina@laptop-AlinaM intellij % ls\n' +
        '<div class="cols">' + cols + '</div></div></div>' +
    '</div>';
  }

  /* ---------- 状态栏 ---------- */
  function status() {
    const c = window.CRUMBS;
    const html = c.map(function (name, i) {
      const last = i === c.length - 1;
      const icon = i === 0 ? svg('project') : (last ? svg('class') : svg('folder'));
      return (i ? '<span class="sep">›</span>' : '') +
        '<span class="cr">' + icon + '<span>' + name + '</span></span>';
    }).join('');
    return '<div class="status">' +
      '<div class="crumbs">' + html + '</div>' +
      '<span class="spacer"></span>' +
      '<span class="w">39:34</span><span class="w">LF</span>' +
      '<span class="w">UTF-8</span><span class="w">' + svg('lock') + '</span>' +
    '</div>';
  }

  /* ---------- 完整窗口 + 可选覆盖层 ---------- */
  function mainWindow(overlay) {
    const ide = '<div class="ide">' + toolbar() +
      '<div style="height:1px;background:var(--border);flex:0 0 auto"></div>' +
      '<div class="body">' + stripe('left') +
        '<div class="main"><div class="top">' + project() + editor() + '</div>' + terminal() + '</div>' +
        stripe('right') +
      '</div>' + status() +
    '</div>';

    const layers = [];
    if (overlay === 'settings') {
      layers.push('<div class="overlay show"><div class="scrim"></div>' + settings() + '</div>');
    }
    if (overlay === 'search') {
      layers.push('<div class="overlay show pin-top"><div class="scrim"></div>' + searchEverywhere() + '</div>');
    }
    if (overlay === 'find') {
      layers.push('<div class="overlay show pin-tl">' + findInFiles() + '</div>');
    }
    return '<div class="canvas">' + ide + layers.join('') + '</div>';
  }

  /* ---------- 设置对话框 ---------- */
  function settings() {
    const nav = ['Appearance & Behavior','Keymap','Editor','Plugins','Version Control',
                 'Build, Execution, Deployment','Languages & Frameworks','Tools'];
    const navHTML = nav.map(function (n, i) {
      return '<div class="it' + (i === 0 ? ' on' : '') + '">' + n + '</div>';
    }).join('');
    function chk(label, on) {
      return '<div class="chk"><span class="box' + (on ? ' on' : '') + '">' +
        (on ? svg('check') : '') + '</span><span>' + label + '</span></div>';
    }
    return '<div class="dlg settings">' +
      '<div class="dlg-head">Settings</div>' +
      '<div class="settings-body">' +
        '<div class="settings-nav">' +
          '<div class="search"><input class="input" style="width:100%" value="Editor Tabs" readonly></div>' +
          '<div class="items">' + navHTML + '</div>' +
        '</div>' +
        '<div class="settings-main">' +
          '<div class="crumbs2"><span>Appearance and Behavior</span><span>›</span>' +
            '<span>Appearance</span><span>›</span><span class="cur">Editor Tabs</span></div>' +
          '<div class="field-row"><label style="width:80px">Placement</label>' +
            '<select class="input"><option>Top</option></select>' +
            chk('Show pinned tabs in a separate row', true) + '</div>' +
          '<div class="field-row"><label style="width:80px">Tab limits</label>' +
            '<input class="input" style="width:64px" value="10" readonly>' +
            '<span style="font-size:13px">Tab limit</span></div>' +
          '<div class="field-row" style="gap:14px">' +
            chk('Show directory for non-unique file names', true) +
            '<span class="link">Configure file colors</span></div>' +
          '<div class="group-title">Opening and closing</div>' +
          chk('Open declaration source in the same tab', false) +
          chk('Open files in the same tab unless they are pinned', false) +
          chk('Open new tabs at the end', true) +
          '<div class="group-title">Tab appearance</div>' +
          '<div class="field-row">' + chk('Show close button', true) + chk('Show file icon', true) + '</div>' +
          '<div class="field-row"><label style="min-width:120px">Active tab highlight</label>' +
            '<select class="input" style="width:150px"><option>Underline</option></select></div>' +
        '</div>' +
      '</div>' +
      '<div class="dlg-foot">' +
        '<button class="btn secondary">Cancel</button>' +
        '<button class="btn primary">Apply</button>' +
        '<button class="btn primary">OK</button>' +
      '</div>' +
    '</div>';
  }

  /* ---------- Search Everywhere ---------- */
  function searchEverywhere() {
    const rows = [
      ['analysis','Classes','analysis.CorrelatedRandomVectorGenerator'],
      ['BivariateFunction','Interfaces','org.apache.commons.math3.analysis.BivariateFunction'],
      ['FunctionUtils','Classes','org.apache.commons.math3.analysis.FunctionUtils'],
      ['MultivariateFunction','Interfaces','org.apache.commons.math3.analysis.MultivariateFunction'],
      ['TrivariateFunction','Interfaces','org.apache.commons.math3.analysis.TrivariateFunction']
    ].map(function (r, i) {
      return '<div class="prow' + (i === 0 ? ' on' : '') + '">' + svg('class') +
        '<span>' + r[0] + '</span><span class="badge">' + r[1] + '</span>' +
        '<span class="path">' + r[2] + '</span></div>';
    }).join('');
    return '<div class="popup search">' +
      '<div class="pinput">' + svg('search') + '<span class="fake">fun</span><span class="caret"></span></div>' +
      '<div class="ptabs"><span class="on">All</span><span>Classes</span><span>Files</span>' +
        '<span>Symbols</span><span>Actions</span></div>' +
      '<div class="psec">Classes and Interfaces</div>' +
      '<div class="plist">' + rows + '</div>' +
    '</div>';
  }

  /* ---------- Find in Files ---------- */
  function findInFiles() {
    const files = [
      ['FunctionUtils.java','8'],['FunctionUtilsTest.java','5'],
      ['MultivariateFunction.java','3'],['BivariateFunction.java','2']
    ].map(function (f, i) {
      return '<div class="prow' + (i === 0 ? ' on' : '') + '">' + svg('class') +
        '<span>' + f[0] + '</span><span class="path">' + f[1] + '</span></div>';
    }).join('');
    const res = [
      ['28','public static MultivariateDifferentiableFunction toDifferentiable(final MultivariateFunction f,'],
      ['29','                                                       final MultivariateVectorFunction gradient) {'],
      ['42','        final double[] dPoint = new double[point.length];'],
      ['51','        final double  v = f.value(dPoint);']
    ].map(function (r) {
      const marked = r[1].replace(/Multivariate|double|final/g, function (m) {
        return '<span class="hit">' + m + '</span>';
      });
      return '<div><span style="color:var(--gutter);display:inline-block;width:34px;text-align:right;margin-right:12px">' +
        r[0] + '</span>' + marked + '</div>';
    }).join('');
    return '<div class="popup find">' +
      '<div class="pinput">' + svg('search') + '<span class="fake">Multivariate</span>' +
        '<span class="caret"></span><span class="badge">Match case</span>' +
        '<span class="badge">Words</span><span class="badge">Regex</span></div>' +
      '<div class="find-split">' +
        '<div class="find-list">' + files + '</div>' +
        '<div class="find-res"><div style="color:var(--fg-dim);margin-bottom:8px">4 files · 18 matches</div>' +
          res + '</div>' +
      '</div>' +
    '</div>';
  }

  /* ---------- 欢迎屏 ---------- */
  function welcome() {
    const actions = [
      ['project','New Project'],['folderOpen','Open'],['commit','Get from VCS'],['library','Remote Development']
    ].map(function (a) {
      return '<div class="wact"><span class="ic">' + svg(a[0]) + '</span><span>' + a[1] + '</span></div>';
    }).join('');
    function card(title, items) {
      return '<div class="card"><h4>' + title + '</h4>' + items.map(function (it) {
        return '<div class="ln"><span class="ic">' + svg(it[0]) + '</span><span>' + it[1] + '</span></div>';
      }).join('') + '</div>';
    }
    const recent = [
      ['intellij','~/intellij','2 minutes ago'],
      ['accurate-math-core','~/projects/accurate-math-core','yesterday']
    ].map(function (r) {
      return '<div class="ritem">' + svg('project') + '<span>' + r[0] + '</span>' +
        '<span class="path">' + r[1] + '</span>' +
        '<span class="path" style="margin-left:auto">' + r[2] + '</span></div>';
    }).join('');
    return '<div class="canvas"><div class="welcome">' +
      '<div class="wbar"><div class="traffic"><i></i><i></i><i></i></div>' +
        '<span class="spacer"></span>' +
        '<div class="right">' + svg('search') + svg('settings') + '</div></div>' +
      '<div class="wbody">' +
        '<div class="wleft"><h3>Welcome to IntelliJ IDEA</h3>' + actions + '</div>' +
        '<div class="wmain">' +
          '<h2>IntelliJ IDEA Community</h2>' +
          '<div class="wsub">Version 2026.1 · accurate-math-core</div>' +
          '<div class="cards">' +
            card('Learn IDE', [['play','Take a Tour'],['info','Getting Started']]) +
            card('Recent', [['commit','Git log'],['problems','Inspect code']]) +
            card('Plugins', [['library','Browse Marketplace'],['settings','Manage plugins']]) +
          '</div>' +
          '<div class="recent"><h4>Recent projects</h4>' + recent + '</div>' +
        '</div>' +
      '</div>' +
    '</div></div>';
  }

  window.IDE = {
    mainWindow: mainWindow,
    welcome: welcome,
    settings: settings,
    searchEverywhere: searchEverywhere,
    findInFiles: findInFiles
  };
})();
