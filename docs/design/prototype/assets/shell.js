/* =============================================================================
   原型外壳控制：主题、画布缩放、底部切换条、bare 模式

   URL 参数
     ?theme=light|lighthdr|dark   主题
     ?bare=1                      隐藏原型自身的页眉与切换条，画布按 1:1 顶到左上角
                                  （用于导出干净的截图 / 与设计稿做像素对比）
   ========================================================================== */
(function () {
  const THEMES = ['light', 'lighthdr', 'dark'];
  const THEME_LABEL = {light:'Light', lighthdr:'Light + Light Header', dark:'Dark'};
  const STORE_KEY = 'intui-theme';
  const DESIGN = {w:1536, h:960};

  const SCREENS = [
    {id:'main',     file:'01-main-window.html',      label:'主窗口'},
    {id:'settings', file:'02-settings.html',         label:'设置'},
    {id:'welcome',  file:'03-welcome.html',          label:'欢迎屏'},
    {id:'search',   file:'04-search-everywhere.html',label:'Search Everywhere'},
    {id:'find',     file:'05-find-in-files.html',    label:'Find in Files'},
    {id:'gallery',  file:'06-components.html',       label:'组件画廊'}
  ];

  const params = new URLSearchParams(location.search);

  function readTheme() {
    const q = params.get('theme');
    if (q && THEMES.indexOf(q) >= 0) return q;
    try {
      const s = localStorage.getItem(STORE_KEY);
      if (s && THEMES.indexOf(s) >= 0) return s;
    } catch (e) { /* file:// 下 localStorage 可能被禁用，忽略 */ }
    return 'light';
  }

  const state = {
    screen: 'main',
    theme: readTheme(),
    bare: params.get('bare') === '1'
  };

  let fitCanvas = null;

  /* ---------- 缩放：保持 1536x960 的像素保真 ---------- */
  function installFit(host, canvas, stage) {
    fitCanvas = function () {
      if (state.bare) {
        canvas.style.transform = 'none';
        host.style.width = DESIGN.w + 'px';
        host.style.height = DESIGN.h + 'px';
        return;
      }
      const pad = 20;
      const s = Math.min(
        (stage.clientWidth - pad) / DESIGN.w,
        (stage.clientHeight - pad) / DESIGN.h,
        1
      );
      canvas.style.transform = 'scale(' + s + ')';
      host.style.width = (DESIGN.w * s) + 'px';
      host.style.height = (DESIGN.h * s) + 'px';
    };
    fitCanvas();
  }

  window.addEventListener('resize', function () { if (fitCanvas) fitCanvas(); });

  /* ---------- 底部切换条 ---------- */
  function buildDock(stateEl) {
    const dock = document.getElementById('dock');
    if (!dock) return;
    const screenLinks = SCREENS.map(function (s) {
      const on = s.id === state.screen ? ' on' : '';
      return '<a class="btnlink' + on + '" data-screen="' + s.id + '" href="' + s.file + '">' +
        s.label + '</a>';
    }).join('');
    const themeButtons = THEMES.map(function (t) {
      return '<button data-theme="' + t + '"' + (t === state.theme ? ' class="on"' : '') + '>' +
        THEME_LABEL[t] + '</button>';
    }).join('');
    dock.innerHTML =
      '<div class="grp"><label>界面</label>' +
        '<a class="btnlink" href="index.html">索引</a>' + screenLinks +
      '</div>' +
      '<div class="grp"><label>主题</label>' + themeButtons + '</div>' +
      '<div class="state" id="shell-state"></div>';

    dock.addEventListener('click', function (e) {
      const b = e.target.closest('button[data-theme]');
      if (!b) return;
      setTheme(b.dataset.theme);
    });
    syncLinks();
  }

  /** 让跳转到其它屏幕时保持当前主题 */
  function syncLinks() {
    document.querySelectorAll('#dock a[data-screen]').forEach(function (a) {
      const f = SCREENS.filter(function (s) { return s.id === a.dataset.screen; })[0];
      if (!f) return;
      a.setAttribute('href', f.file + '?theme=' + state.theme + (state.bare ? '&bare=1' : ''));
    });
  }

  function setTheme(t) {
    if (THEMES.indexOf(t) < 0) return;
    state.theme = t;
    document.documentElement.dataset.theme = t;
    try { localStorage.setItem(STORE_KEY, t); } catch (e) { /* 忽略 */ }
    const url = new URL(location.href);
    url.searchParams.set('theme', t);
    history.replaceState(null, '', url);
    document.querySelectorAll('#dock button[data-theme]').forEach(function (b) {
      b.classList.toggle('on', b.dataset.theme === t);
    });
    syncLinks();
    updateState();
  }

  function updateState() {
    const el = document.getElementById('shell-state');
    if (el) {
      el.textContent = 'screen=' + state.screen + '  theme=' + state.theme +
        (state.bare ? '  bare=1' : '');
    }
  }

  /* ---------- 启动 ---------- */
  window.Shell = {
    boot: function (opts) {
      state.screen = opts.screen;
      document.documentElement.dataset.theme = state.theme;
      if (state.bare) document.documentElement.dataset.bare = '1';

      const stage = document.getElementById('stage');
      stage.innerHTML = '<div class="screen">' + (state.bare
        ? opts.build()
        : '<div class="fit"><div class="canvas-host">' + opts.build() + '</div></div>') + '</div>';

      if (!state.bare) {
        const host = stage.querySelector('.canvas-host');
        const canvas = host.querySelector('.canvas');
        host.style.position = 'relative';
        canvas.style.position = 'absolute';
        canvas.style.top = '0';
        canvas.style.left = '0';
        canvas.style.transformOrigin = 'top left';
        installFit(host, canvas, stage);
      }

      buildDock();
      updateState();
      document.title = opts.title + ' · Int UI Kit 原型';
    },
    /* 画廊不需要缩放画布，单独一个入口 */
    bootGallery: function (opts) {
      state.screen = opts.screen;
      document.documentElement.dataset.theme = state.theme;
      if (state.bare) document.documentElement.dataset.bare = '1';
      document.getElementById('stage').innerHTML = '<div class="screen">' + opts.build() + '</div>';
      buildDock();
      updateState();
      document.title = opts.title + ' · Int UI Kit 原型';
    }
  };
})();
