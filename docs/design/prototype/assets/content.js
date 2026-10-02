/* =============================================================================
   内容数据
   项目树、编辑器标签页、源码、终端输出 —— 文案均取自 Components > Main Window
   的渲染图，用来还原真实的界面密度，不是占位符。
   ========================================================================== */

/** 项目树。lvl 对应缩进级别；ic 为 chevron 时不画文件图标。 */
window.TREE = [
  {lvl:1, ic:'chevD', open:true,  label:'intellij', dim:'~/intellij'},
  {lvl:2, ic:'chevR', label:'.idea', type:'folder'},
  {lvl:2, ic:'chevD', open:true,  label:'src', type:'folder'},
  {lvl:3, ic:'chevD', open:true,  label:'java', type:'folder'},
  {lvl:4, ic:'class', label:'analysis', type:'java', sel:true},
  {lvl:4, ic:'class', label:'BivariateFunction', type:'java'},
  {lvl:4, ic:'class', label:'FunctionUtils', type:'java'},
  {lvl:4, ic:'class', label:'MultivariateFunction', type:'java'},
  {lvl:4, ic:'class', label:'TrivariateFunction', type:'java'},
  {lvl:3, ic:'chevR', label:'polynomials', type:'folder'},
  {lvl:3, ic:'chevR', label:'solver', type:'folder'},
  {lvl:2, ic:'chevD', open:true,  label:'test', type:'folder'},
  {lvl:3, ic:'chevD', open:true,  label:'java', type:'folder'},
  {lvl:4, ic:'class', label:'FunctionUtilsTest', type:'java'},
  {lvl:4, ic:'class', label:'MonitoredFunction', type:'java'},
  {lvl:4, ic:'class', label:'SumSyncFunction', type:'java'},
  {lvl:2, ic:'chevD', open:true,  label:'target', type:'folder'},
  {lvl:3, ic:'chevR', label:'classes', type:'folder'},
  {lvl:3, ic:'chevR', label:'generated-sources', type:'folder'},
  {lvl:2, ic:'gitignore', label:'.gitignore', type:'file'},
  {lvl:2, ic:'chevR', label:'External Libraries', type:'library'}
];

/** 图标映射：树节点的 type -> 图标名 */
window.TREE_ICON = {folder:'folder', java:'class', file:'gitignore', library:'library'};

/** 编辑器标签页。原稿里激活的是 AdapterScript.java（下划线为灰 #A8ADBD，
    不是蓝色），前面几个带文件类型图标 —— 图标也是原稿里有的，不能省。
    w 是该标签页在文档树里的实测宽度，直接沿用：这样横向位置与原稿一致，
    不会因为字体回落而产生累计漂移。 */
window.TABS = [
  {name:'FunctionUtils.java', icon:'java', w:173},
  {name:'add-hover.svg', icon:'svg', w:152},
  {name:'add.svg', icon:'svg', w:112},
  {name:'AdapterScript.java', icon:'java', w:178, on:true, close:true},
  {name:'AdapterScriptInterface.java', icon:'java', w:234, close:true}
];

/* 源码。内容与语法着色分区取自原稿渲染图；关键字颜色是采样值：
   浅色 #0033B3（蓝），暗色 #D69A6B（橙）。 */
window.CODE = [
  ['c','/** Convert regular functions to {@link MultivariateDifferentiableFunction}. ... */'],
  ['p','<span class="k">public static</span> MultivariateDifferentiableFunction toDifferentiable(<span class="k">final</span> MultivariateFunction f,'],
  ['p','                                                             <span class="k">final</span> MultivariateVectorFunction gradient) {'],
  ['',''],
  ['p','    <span class="k">return new</span> MultivariateDifferentiableFunction() {'],
  ['',''],
  ['c','        /** {@inheritDoc} */'],
  ['a','        @Override'],
  ['p','        <span class="k">public double</span> value(<span class="k">final double</span>[] point) { <span class="k">return</span> f.value(point); }'],
  ['',''],
  ['c','        /** {@inheritDoc} */'],
  ['a','        @Override'],
  ['p','        <span class="k">public</span> DerivativeStructure value(<span class="k">final</span> DerivativeStructure[] point) {'],
  ['',''],
  ['c','            // set up the input parameters'],
  ['p','            <span class="k">final double</span>[] dPoint = <span class="k">new double</span>[point.length];'],
  ['p','            <span class="k">for</span> (<span class="k">int</span> i = 0; i &lt; point.length; ++i) {'],
  ['p','                dPoint[i] = point[i].getValue();'],
  ['p','                <span class="k">if</span> (point[i].getOrder() &gt; 1) {'],
  ['p','                    <span class="k">throw new</span> NumberIsTooLargeException(point[i].getOrder(), 1, <span class="k">true</span>);'],
  ['p','                }'],
  ['p','            }'],
  ['',''],
  ['c','            // evaluate regular functions'],
  ['p','            <span class="k">final double</span>  v = f.value(dPoint);'],
  ['p','            <span class="k">final double</span>[] dv = gradient.value(dPoint);']
];
window.CODE_START = 27;   // 第一行显示的行号
window.CODE_HL = 31;      // 高亮行

/** 终端 ls 输出，按列优先切成 8+9 两列，与原稿一致 */
window.TERM_LS = [
  'execution','fetch-tag.sh','git-hooks','gitBisect.py','goLand','help','idea-tests',
  'intellij.idea.ultimate.main.android.iml',
  'lightEdit','mobile-ide','native','out','phpstorm','platform','plugins','python','refactoring'
];
window.TERM_SPLIT = 8;

/** 状态栏面包屑 */
window.CRUMBS = ['intellij','accurate-math-core','src','main','java','org','math','core','AccurateMath'];

/** 编辑器右侧的标记条（错误 / 警告 / 可视区域） */
window.SCROLL_MARKS = [
  {top:'2%',  height:'5%', color:'var(--red)'},
  {top:'7%',  height:'3%', color:'var(--orange)'},
  {top:'30%', height:'34%', color:'var(--scroll)'}
];
