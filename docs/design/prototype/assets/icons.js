/* =============================================================================
   图标集
   形态参考 Int UI Icons（fileKey TbXho57a8mA80b5vE0oTxk）：16x16 画布、
   1px 描边、圆角 2px 的描边风格，与 Int UI Icons Guidelines 的规则一致。
   只保留原型用得到的部分，不是完整图标库。
   ========================================================================== */
window.ICONS = {
  folder:'<path d="M1.5 3.5h4l1.2 1.4h7.8v7.6H1.5z"/>',
  folderOpen:'<path d="M1.5 3.5h4l1.2 1.4h7.8v2H3.6L1.5 12.5z"/><path d="M1.5 5.9V12.5h12L15.4 7H3.6z"/>',
  class:'<circle cx="8" cy="8" r="5.2"/><path d="M9.6 6.2A1.9 1.9 0 0 0 6.4 7.4v1.2a1.9 1.9 0 0 0 3.2 1.2"/>',
  markdown:'<rect x="2" y="4" width="12" height="8" rx="1.2"/><path d="M4.2 10V6l2 2.4L8.2 6v4M10.4 6v4M9.4 9l1 1 1-1"/>',
  gitignore:'<circle cx="8" cy="8" r="5.2"/><path d="M8 5.4v5.2"/>',
  library:'<rect x="2.5" y="2.5" width="11" height="11" rx="1.2"/><path d="M5.5 5.5h5M5.5 8h5M5.5 10.5h3"/>',
  chevR:'<path d="M6 4l4 4-4 4"/>',
  chevD:'<path d="M4 6l4 4 4-4"/>',
  project:'<path d="M1.5 3.5h4l1.2 1.4h7.8v7.6H1.5z"/>',
  structure:'<path d="M2 4h12M2 8h12M2 12h8"/>',
  commit:'<circle cx="5" cy="4.5" r="1.6"/><circle cx="5" cy="11.5" r="1.6"/><path d="M5 6.1v3.8M6.6 4.5H11a1.5 1.5 0 0 1 1.5 1.5v4"/>',
  bookmark:'<path d="M4 2.5h8v11l-4-2.6-4 2.6z"/>',
  more:'<circle cx="3.5" cy="8" r="1"/><circle cx="8" cy="8" r="1"/><circle cx="12.5" cy="8" r="1"/>',
  services:'<ellipse cx="8" cy="4" rx="5" ry="2"/><path d="M3 4v8c0 1.1 2.2 2 5 2s5-.9 5-2V4M3 8c0 1.1 2.2 2 5 2s5-.9 5-2"/>',
  run:'<path d="M4.5 3.2l8 4.8-8 4.8z" fill="currentColor" stroke="none"/>',
  debug:'<circle cx="8" cy="9" r="4"/><path d="M8 5V3M4 6.5L2.4 5M12 6.5L13.6 5M4.2 10H2.2M11.8 10h2M5.4 12.6L4 14M10.6 12.6L12 14"/>',
  problems:'<circle cx="8" cy="8" r="5.4"/><path d="M8 5.4v3.4M8 10.6v.6"/>',
  search:'<circle cx="7.2" cy="7.2" r="4.4"/><path d="M10.6 10.6L14 14"/>',
  settings:'<circle cx="8" cy="8" r="2.2"/><path d="M8 1.8v1.6M8 12.6v1.6M1.8 8h1.6M12.6 8h1.6M3.6 3.6l1.1 1.1M11.3 11.3l1.1 1.1M12.4 3.6l-1.1 1.1M4.7 11.3l-1.1 1.1"/>',
  user:'<circle cx="8" cy="5.6" r="2.6"/><path d="M3.4 13.4a4.8 4.8 0 0 1 9.2 0"/>',
  branch:'<circle cx="4.5" cy="4" r="1.6"/><circle cx="4.5" cy="12" r="1.6"/><circle cx="11.5" cy="7" r="1.6"/><path d="M4.5 5.6v4.8M6.1 4h3.8a1.6 1.6 0 0 1 1.6 1.6v1.8"/>',
  collapse:'<path d="M9 3v4h4M3 9h4v4"/>',
  minus:'<path d="M3.5 8h9"/>',
  close:'<path d="M4 4l8 8M12 4l-8 8"/>',
  hamburger:'<path d="M2.5 4.5h11M2.5 8h11M2.5 11.5h11"/>',
  check:'<path d="M3.5 8.5l3 3 6-6.5"/>',
  dash:'<path d="M4 8h8"/>',
  warning:'<path d="M8 2.6l5.8 10.4H2.2z"/><path d="M8 6.6v3M8 11.2v.4"/>',
  info:'<circle cx="8" cy="8" r="5.4"/><path d="M8 7.4v3.4M8 5.4v.4"/>',
  lock:'<rect x="3.5" y="7" width="9" height="6" rx="1.2"/><path d="M5.6 7V5.4a2.4 2.4 0 0 1 4.8 0V7"/>',
  play:'<path d="M5 3.6l7 4.4-7 4.4z" fill="currentColor" stroke="none"/>',
  bug:'<circle cx="8" cy="9" r="3.6"/><path d="M8 5.4V3.6"/>',
  dropdown:'<path d="M3 5l5 5 5-5"/>',
  /* 编辑器标签页的文件类型图标（原稿里 .java 是蓝色 C 圆圈，.svg 是蓝色图片方块） */
  fileJava:'<circle cx="8" cy="8" r="5.2"/><path d="M9.6 6.2A1.9 1.9 0 0 0 6.4 7.4v1.2a1.9 1.9 0 0 0 3.2 1.2"/>',
  fileSvg:'<rect x="2.5" y="3" width="11" height="10" rx="1.4"/><circle cx="6" cy="6.4" r="1"/><path d="M3 11l3-3 2.4 2.2L11 7.6l2.5 2.4"/>'
};

/** 生成一个内联 SVG 图标。 */
window.svg = function svg(name, cls) {
  const body = window.ICONS[name] || '';
  return '<svg class="' + (cls || '') + '" viewBox="0 0 16 16">' + body + '</svg>';
};
