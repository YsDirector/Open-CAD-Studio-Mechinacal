// spline_gui.html 运行时/行为冒烟（node 最小 DOM 垫片 + 最小 fetch 桩）。
//
// 目的：锁住「智能卡片」（OCSMCARD）本期卡类型「花键参数表」的交互契约
//（node --check / el("id") 静态扫描查不出）：
//   ① 表驱动：卡类型/体系/等级/配合/齿根/压力角清单全部由 /api/spline_options 下发，
//      页面里不写死（换体系/加卡片只加后端数据行）；
//   ② 齿形表达式：粘九字段表达式 → 后端反解 m/z/αD/x/Da/Df + 齿根（30° 平/圆可分辨）；
//      旧的「读选中/上一个块」路径**已删除**（不调用 /api/spline_meta、没有按钮）；
//   ③ 量棒：标准解 + 3 个工程备选都渲染成可点选 chip；手填也可；
//      ★ 选/填 Dp → 预览请求带 dp、Md 随之重算；
//   ④ 外花键：量棒面板可用（M_Re 备用，标注「公法线为主、跨棒距备用」），
//      主表项仍出公法线长度/跨测齿数（21 项卡面不加 M_Re 行）；
//   ⑤ 出表：无 at → 待放置件（请求不带 at）；有 at → 直插；成功后自动关窗；
//   ⑥ 旧插件 404 → 直白提示（照 hole_gui.html 的 readApi 判据）；
//   ⑦ 信息分层负断言：公式/口径/来源只在 title=，可见文本（剥标签）里没有。
//
// 用法：node spline_gui_smoke.mjs <spline_gui.html 路径>

import fs from 'node:fs';
import { renderZh } from './i18n_zh_fixture.mjs';

const htmlPath = process.argv[2];
if (!htmlPath) {
  console.error('usage: node spline_gui_smoke.mjs <spline_gui.html>');
  process.exit(2);
}
const html = renderZh(fs.readFileSync(htmlPath, 'utf8'));
const m = html.match(/<script>([\s\S]*?)<\/script>/);
if (!m) {
  console.error('no <script> found');
  process.exit(2);
}
const script = m[1];

const errors = [];
function check(cond, msg) {
  if (!cond) errors.push(msg);
}

// ── 选项表桩（形状与 /api/spline_options 一致；内容不参与生产断言，只锁交互）──
const COL_INT = [
  '齿形角', '齿数', '模数', '公差等级和配合类别', '大径 Dei', '渐开线终止圆直径最大值',
  '小径 Dii', '测量跨棒距 Md', '量棒直径 Dp', '作用齿槽宽最小值', '实际齿槽宽最大值',
  '齿根圆最小曲率半径 R_imin', '齿形公差 Ff', '齿距累计公差 Fp', '综合公差 λ',
  '小径下公差', '小径上公差', '测量跨棒距下公差', '测量跨棒距上公差', '大径下公差', '大径上公差',
];
const COL_EXT = COL_INT.map((s) =>
  s.replace('大径 Dei', '大径 Dee').replace('小径 Dii', '小径 Die')
   .replace('测量跨棒距 Md', '公法线长度 Wn').replace('量棒直径 Dp', '跨测齿数 Kn')
   .replace('作用齿槽宽最小值', '实际齿厚最小值').replace('实际齿槽宽最大值', '作用齿厚最大值')
   .replace('测量跨棒距下公差', '公法线长度下公差').replace('测量跨棒距上公差', '公法线长度上公差'));
function columns(prefix, labels) {
  return labels.map((label, i) => ({
    tag: `(${prefix})${label}`,
    label,
    unit: i === 0 ? '°' : (label.includes('公差') ? 'mm' : ''),
    formula: `stub 公式 ${i + 1}（不进可见文本）`,
    source: 'stub 来源',
  }));
}
const SYSTEMS = [{
  id: 'gb3478',
  aliases: ['gb', 'GB'],
  label: 'GB/T 3478（stub）',
  standard: 'GB/T 3478（stub）',
  note: 'stub 体系口径',
  x_note: 'stub x 口径',
  sides: [
    {
      id: 'int', label: '内花键', title: 'stub 内花键',
      grade_note: 'stub 等级口径', alpha_note: 'stub 压力角口径',
      grades: [4, 5, 6, 7],
      fits: [{ fit: 'H', code: 'H', label: 'H（内花键基孔制）', preferred_45: true, memo: 'stub 配合口径' }],
      roots: [
        { key: 'flat', label: '平齿根', alphas: [30, 37.5, 45], note: 'stub 平齿根口径' },
        { key: 'fillet', label: '圆齿根', alphas: [30, 37.5, 45], note: 'stub 圆齿根口径' },
      ],
      alphas: [30, 37.5, 45],
      pin: {
        applicable: true, label: '量棒直径 Dp 与测量跨棒距 Md',
        dp_label: '量棒直径 Dp', md_label: '测量跨棒距 Md',
        formula: "stub D'_Ri 公式", md_formula: 'stub M_Ri 公式', standard: 'stub R40 选棒规则',
      },
      columns: columns('内', COL_INT),
    },
    {
      id: 'ext', label: '外花键', title: 'stub 外花键',
      grade_note: 'stub 等级口径', alpha_note: 'stub 压力角口径',
      grades: [4, 5, 6, 7],
      fits: [
        { fit: 'H/k', code: 'k', label: 'H/k', preferred_45: true, memo: 'stub 过盈' },
        { fit: 'H/js', code: 'js', label: 'H/js', preferred_45: false, memo: 'stub 过渡' },
        { fit: 'H/h', code: 'h', label: 'H/h', preferred_45: true, memo: 'stub 间隙 0' },
        { fit: 'H/f', code: 'f', label: 'H/f', preferred_45: true, memo: 'stub 小间隙' },
        { fit: 'H/e', code: 'e', label: 'H/e', preferred_45: false, memo: 'stub 中间隙' },
        { fit: 'H/d', code: 'd', label: 'H/d', preferred_45: false, memo: 'stub 大间隙' },
      ],
      roots: [
        { key: 'flat', label: '平齿根', alphas: [30, 37.5, 45], note: 'stub 平齿根口径' },
        { key: 'fillet', label: '圆齿根', alphas: [30, 37.5, 45], note: 'stub 圆齿根口径' },
      ],
      alphas: [30, 37.5, 45],
      // 用户 2026-09-27：外花键主用 Wn/Kn，M_Re 面板也可用（公法线为主、跨棒距备用）。
      pin: {
        applicable: true,
        label: '量棒直径 D_Re 与跨棒距 M_Re（公法线为主、跨棒距备用）',
        dp_label: '量棒直径 D_Re', md_label: '跨棒距 M_Re',
        formula: "stub D'_Re 公式", md_formula: 'stub M_Re 公式', standard: 'stub R40 选棒规则',
      },
      columns: columns('外', COL_EXT),
    },
  ],
}];
const OPTIONS = {
  ok: true,
  card_types: [{
    id: '花键参数表',
    aliases: ['spline'],
    label: 'GB 花键参数表（内）',
    summary: 'stub 卡片说明',
    systems: [{ id: 'gb3478', label: 'GB/T 3478（stub）', standard: 'stub' }],
    direction: 'int',
    renderer: 'spline_table',
    form: null,
  }, {
    id: '花键参数表_外',
    aliases: ['splineext'],
    label: 'GB 花键参数表（外）',
    summary: 'stub 卡片说明（外）',
    systems: [{ id: 'gb3478', label: 'GB/T 3478（stub）', standard: 'stub' }],
    direction: 'ext',
    renderer: 'spline_table',
    form: null,
  }],
  systems: SYSTEMS,
  pin_series: [0.56, 0.60, 0.63, 0.67, 0.71, 0.75, 0.80, 0.85, 0.90, 0.95, 1.00, 1.06, 1.12, 1.18, 1.25],
  pin_series_note: 'stub 量棒系列',
};

const SERIES = OPTIONS.pin_series;
const DP_CALC = 0.93;
const pickStd = (x) => SERIES.find((v) => v >= x - 1e-9);
// 桩里的 Md 与 Dp 单调挂钩：选不同 Dp，Md 必然变——断言「选了 Dp 必须重算」用。
const mdOf = (dp) => Number((10 + 1.5 * (dp - 1.0)).toFixed(6));
const near = (a, b, eps = 1e-9) => Math.abs(Number(a) - Number(b)) < eps;

// 九字段表达式最小解析（与生产 `shaft::parse_program` 同字段口径，只取 stub 要用的）。
function parseExpr(expr) {
  const toks = String(expr || '').trim().split(/\s+/).filter(Boolean);
  const out = { mark: null, kind: null, m: null, z: null, alpha: null, x: 0, da: null, df: null };
  for (const t of toks) {
    const u = t.toUpperCase();
    if (u === 'GEAR' || u === 'SPLINE') out.mark = u;
    else if (u === 'IN' || u === 'EX') out.kind = u;
    else if (u.startsWith('ALPHA')) out.alpha = Number(u.slice(5));
    else if (u.startsWith('BETA')) { /* 本期只支持 0 */ }
    else if (u.startsWith('DA')) out.da = Number(u.slice(2));
    else if (u.startsWith('DF')) out.df = Number(u.slice(2));
    else if (u.startsWith('M')) out.m = Number(u.slice(1));
    else if (u.startsWith('Z')) out.z = Number(u.slice(1));
    else if (u.startsWith('X')) out.x = Number(u.slice(1));
    else if (u.startsWith('H')) { /* 轴段厚度：本卡不用 */ }
  }
  return out;
}

function inferRoot(side, g) {
  if (!(Math.abs(Number(g.alpha) - 30) < 1e-9)) return null;
  if (side === 'int' && g.da != null) {
    if (near(g.da, g.m * (g.z + 1.5))) return 'flat';
    if (near(g.da, g.m * (g.z + 1.8))) return 'fillet';
  }
  if (side === 'ext' && g.df != null) {
    if (near(g.df, g.m * (g.z - 1.5))) return 'flat';
    if (near(g.df, g.m * (g.z - 1.8))) return 'fillet';
  }
  return null;
}

function findCol(side, needle) {
  return (side.columns || []).find((c) => c.label.includes(needle));
}

function preview(m) {
  const sys = OPTIONS.systems.find((s) => s.id === (m.system || 'gb3478'));
  const side = sys && sys.sides.find((s) => s.id === m.side);
  if (!side) return { ok: false, error: `方向 ${m.side} 不存在` };
  if (!side.grades.includes(m.grade)) return { ok: false, error: `等级 ${m.grade} 不在可选项` };
  const g = parseExpr(m.expr);
  if (!g.mark || g.m == null || g.z == null || g.alpha == null) {
    return { ok: false, error: `花键参数表：齿形表达式无法解析（stub）：${m.expr}` };
  }
  if (g.kind && ((g.kind === 'IN') !== (m.side === 'int'))) {
    return { ok: false, error: '花键参数表：表达式 KIND 与卡片方向「不一致」（stub）' };
  }
  if (!side.alphas.includes(g.alpha)) return { ok: false, error: `压力角 ${g.alpha} 不在可选项` };
  if (!(g.m > 0) || !(g.z >= 6)) return { ok: false, error: '模数/齿数不合法' };
  const inferred = inferRoot(m.side, g);
  const explicit = m.root && m.root !== 'auto' ? m.root : null;
  const root = explicit || inferred || (Math.abs(g.alpha - 30) < 1e-9 ? 'flat' : 'fillet');
  const rootSource = explicit ? 'explicit' : inferred ? 'expr' : 'default';
  const items = side.columns.map((c) => {
    let value = `v-${c.label}`;
    if (c.label.includes('齿形角')) value = `${g.alpha}°`;
    if (c.label.includes('齿数')) value = String(g.z);
    if (c.label.includes('模数')) value = String(g.m);
    return { tag: c.tag, label: c.label, unit: c.unit, value, formula: c.formula, source: c.source };
  });
  let dp;
  if (side.pin && side.pin.applicable) {
    if (m.dp != null && !SERIES.some((v) => near(v, m.dp))) {
      return { ok: false, error: `量棒参数：Dp=${m.dp} 不在 GB/T 3478.9 表 1 量棒系列（stub）` };
    }
    const auto = pickStd(DP_CALC);
    const current = m.dp != null ? m.dp : auto;
    const md = mdOf(current);
    // 内花键：M_Ri 插回 21 项「测量跨棒距」；外花键：M_Re 只在面板（卡面 21 项不加行）。
    const col = findCol(side, '测量跨棒距');
    if (col) {
      const it = items.find((x) => x.tag === col.tag);
      if (it) it.value = md.toFixed(3);
    }
    // 与生产同口径：标准解 + 3 个**去重后**的最近系列值（共 4 个可点 chip）
    const choices = [{ value: auto, tag: '标准解', standard: true }];
    for (const v of SERIES.slice().sort((a, b) => Math.abs(a - DP_CALC) - Math.abs(b - DP_CALC))) {
      if (choices.length >= 4) break;
      if (choices.some((c) => near(c.value, v))) continue;
      choices.push({ value: v, tag: '备选', standard: false });
    }
    choices.sort((a, b) => a.value - b.value);
    dp = {
      applicable: true, label: side.pin.label,
      dp_label: side.pin.dp_label, md_label: side.pin.md_label,
      formula: side.pin.formula,
      md_formula: side.pin.md_formula, standard: side.pin.standard,
      current, auto, calc: DP_CALC, manual: m.dp != null,
      choices,
      md: { value: md, lower: md - 0.02, upper: md + 0.03 },
    };
  } else {
    dp = { applicable: false, reason: side.pin && side.pin.reason };
  }
  // 外花键主表项：公法线长度（与 M_Re 面板并存，不重复堆 M_Re）。
  if (side.id === 'ext') {
    const col = findCol(side, '公法线长度');
    if (col) {
      const it = items.find((x) => x.tag === col.tag);
      if (it) it.value = '35.500';
    }
  }
  return {
    ok: true, system: m.system || 'gb3478', side: side.id, side_label: side.label,
    grade_fit: `${m.grade}${m.fit}`, expr: m.expr,
    mark: g.mark, kind: m.side === 'int' ? 'IN' : 'EX',
    m: g.m, z: g.z, alpha: g.alpha, x: g.x || 0, da: g.da, df: g.df,
    root, root_source: rootSource, dp, items,
  };
}

// ── 最小 DOM 垫片 ────────────────────────────────────────────────
function mkEl(id) {
  const el = {
    id, tagName: 'DIV', type: '', dataset: {}, style: {}, value: '', checked: false,
    disabled: false, readOnly: false, textContent: '', children: [], options: [],
    _handlers: {}, _innerHTML: '',
    classList: {
      _on: new Set(),
      add(c) { this._on.add(c); },
      remove(c) { this._on.delete(c); },
      contains(c) { return this._on.has(c); },
      toggle(c, force) {
        const want = force === undefined ? !this._on.has(c) : !!force;
        if (want) this._on.add(c); else this._on.delete(c);
        return want;
      },
    },
    addEventListener(ev, fn) { (this._handlers[ev] ||= []).push(fn); },
    _fire(ev, target, extra) {
      const e = Object.assign({ target, type: ev }, extra || {});
      (this._handlers[ev] || []).forEach((f) => f(e));
    },
    appendChild(c) {
      if (this.tagName === 'SELECT' && c.tagName === 'OPTION') this.options.push(c);
      else this.children.push(c);
      return c;
    },
    removeChild(c) { this.children = this.children.filter((x) => x !== c); },
    querySelectorAll() { return []; },
    querySelector() { return null; },
    setSelectionRange() {},
    closest() { return null; },
    focus() {},
    remove() {},
    click() { this._fire('click', this); },
  };
  Object.defineProperty(el, 'innerHTML', {
    get() { return el._innerHTML; },
    set(v) { el._innerHTML = String(v); if (el._innerHTML === '') { el.children = []; el.options = []; } },
  });
  return el;
}

const els = new Map();
const SELECT_IDS = new Set(['cardType', 'grade', 'fit', 'alpha', 'root']);
global.document = {
  getElementById(id) {
    if (!els.has(id)) {
      const e = mkEl(id);
      if (SELECT_IDS.has(id)) e.tagName = 'SELECT';
      if (id === 'expr') e.tagName = 'TEXTAREA';
      els.set(id, e);
    }
    return els.get(id);
  },
  createElement(tag) {
    const e = mkEl('dyn');
    e.tagName = String(tag).toUpperCase();
    return e;
  },
  createTextNode(t) { const e = mkEl('text'); e.textContent = String(t); return e; },
  querySelectorAll() { return []; },
  querySelector() { return null; },
  addEventListener() {},
  activeElement: null,
};
// 出表成功后应像孔/轴/标准件那样自动关窗：window.close 置位 + setTimeout 立即执行
let closed = false;
global.window = { addEventListener() {}, close() { closed = true; } };
global.location = { search: '', href: 'http://127.0.0.1:9/spline' };
global.setTimeout = (fn) => { if (typeof fn === 'function') fn(); return 0; };
global.clearTimeout = () => {};
global.setInterval = () => 0;
global.URLSearchParams = class {
  constructor() {}
  get() { return null; }
};

// ── fetch 桩 ─────────────────────────────────────────────────────
let lastPreviewModel = null;
let lastExportModel = null;
let lastExportUrl = '';
let metaCalled = false;
let forceExport404 = false;
let forcePreview404 = false;
const consoleErrors = [];
const realError = console.error.bind(console); // report() 要用真 stderr（console.error 被冒烟接管了）
console.error = (...a) => { consoleErrors.push(a.join(' ')); };
function jsonResp(obj, status = 200) {
  return { ok: status < 400, status, json: async () => obj, text: async () => JSON.stringify(obj) };
}
function text404(path) {
  return {
    ok: false,
    status: 404,
    text: async () => 'not found',
    json: async () => { throw new SyntaxError(`Unexpected token 'o', "${path}" is not valid JSON`); },
  };
}
global.fetch = async (u, opts = {}) => {
  const url = String(u);
  if (url.startsWith('/api/spline_options')) return jsonResp(OPTIONS);
  if (url.startsWith('/api/spline_preview')) {
    if (forcePreview404) return text404('/api/spline_preview');
    const model = JSON.parse(opts.body || '{}');
    lastPreviewModel = model;
    const v = preview(model);
    if (v.ok === false) return jsonResp(v, 400);
    return jsonResp(v);
  }
  if (url.startsWith('/api/spline_meta')) {
    // 旧路径必须不再被调用（删路径要删干净）。
    metaCalled = true;
    return jsonResp({ ok: false, error: 'spline_meta 已移除' }, 404);
  }
  if (url.startsWith('/api/spline_export')) {
    lastExportUrl = url;
    if (forceExport404) return text404('/api/spline_export');
    lastExportModel = JSON.parse(opts.body || '{}');
    // 与生产后端同口径：导出前先校验（400 + {error} 原样透出）
    const v = preview(lastExportModel);
    if (v.ok === false) return jsonResp(v, 400);
    return jsonResp({ ok: true, message: 'stub 已生成', pending: lastExportModel.at == null });
  }
  return jsonResp({ ok: true });
};

// ── 跑 GUI 脚本（末尾探针暴露内部状态；不改生产代码）──────────────
const probed = script.replace(/\}\)\(\);\s*$/, `;globalThis.__card = {
  get opt() { return OPT; },
  get card() { return CARD; },
  get side() { return SIDE; },
  get dp() { return dpSel; },
  currentModel, refresh, syncUi, applyCardDirection,
};
})();`);
try {
  // 页面外链的共享助手（/ocsm_gui_common.js）：Node 里按同一目录文件先求值
  // （页面脚本用 fetchApi/ocsmStatus/ocsmSeq/ocsmClearStatus）。
  (0, eval)(renderZh(fs.readFileSync(htmlPath.replace(/[^/]+$/, 'ocsm_gui_common.js'), 'utf8')));
  (0, eval)(probed);
} catch (e) {
  errors.push('脚本求值异常: ' + (e && e.stack ? e.stack : e));
}
const tick = async (n = 4) => { for (let i = 0; i < n; i++) await new Promise((r) => setImmediate(r)); };
await tick();

const H = globalThis.__card;
const el = (id) => document.getElementById(id);
// 一卡一方向：切方向 = 切卡（面板不再有内外按钮）
function switchCard(id) {
  el('cardType').value = id;
  el('cardType')._fire('change', el('cardType'));
}
check(!!H, '探针 __card 未挂上（脚本初始化崩溃？）');
if (!H) report();

// ⓪ 信息分层（可见文本，剥标签 + 脚本/样式；title 属性随标签一起剥掉）：
//    公式/口径/来源不得常显；常显只放操作引导。
const visibleText = html
  .replace(/<script[\s\S]*?<\/script>/g, ' ')
  .replace(/<style[\s\S]*?<\/style>/g, ' ')
  .replace(/<[^>]+>/g, ' ');
for (const bad of [
  "D'_Ri", 'M_Ri', 'R40', '3478.9', '0.6√', 'invα', '表 23', '表 24', '表 25',
  '量棒系列', '公式', '来源：',
]) {
  check(!visibleText.includes(bad), `常显区不应含公式/口径/来源「${bad}」`);
}
check(visibleText.includes('点「计算书」') && visibleText.includes('出表'), `常显区应保留操作引导：${visibleText.slice(0, 200)}`);
check(html.includes('title="智能卡片类型（表驱动'), '卡类型口径应进原生 title');
check(el('pinLabel').title.includes("stub D'_Ri 公式"), `量棒公式应从选项表进 title：${el('pinLabel').title}`);
check(el('grade').title.includes('stub 等级口径'), `等级口径应进 title：${el('grade').title}`);

// ① 表驱动：卡类型/体系/清单全部来自 /api/spline_options
check(H.card && H.card.id === '花键参数表', `默认卡类型应为花键参数表，实为 ${H.card && H.card.id}`);
check(el('cardType').options.map((o) => o.value).join(',') === '花键参数表,花键参数表_外', '卡类型下拉来自 card_types');
check(el('cardType').disabled === false, '两个卡类型时不应置灰');
check(H.side && H.side.id === 'int', `默认方向应为内花键，实为 ${H.side && H.side.id}`);
check(!('system' in (lastPreviewModel || {})), '预览模型不应再带 system（体系字段已删）');
check(el('grade').options.map((o) => o.value).join(',') === '4,5,6,7', `等级清单：${el('grade').options.map((o) => o.value)}`);
check(el('grade').value === '6', `默认等级应为 6，实为 ${el('grade').value}`);
check(el('fit').options.map((o) => o.value).join(',') === 'H', `内花键配合应为 H，实为 ${el('fit').options.map((o) => o.value)}`);
check(el('alpha').value === '30', `默认压力角应为 30，实为 ${el('alpha').value}`);
check(el('root').options.map((o) => o.value).join(',') === 'auto,flat,fillet', `齿根形式：${el('root').options.map((o) => o.value)}`);
check(el('root').value === 'auto', `齿根默认应为自动反解，实为 ${el('root').value}`);

// ② 表达式反解：默认表达式 → m/z/αD/x/Da/Df + 齿根来源；旧路径不再调用
await H.refresh();
check(!!lastPreviewModel && lastPreviewModel.side === 'int', `预览模型：${JSON.stringify(lastPreviewModel)}`);
check(lastPreviewModel.dp === null, '默认 dp 应为 null（标准解自动）');
check(String(lastPreviewModel.expr).startsWith('SPLINE IN'), `预览请求应带表达式：${JSON.stringify(lastPreviewModel.expr)}`);
check(el('mOut').textContent === '3' && el('zOut').textContent === '20', `反解 m/z：${el('mOut').textContent}/${el('zOut').textContent}`);
check(el('alphaOut').textContent === '30', `反解 αD：${el('alphaOut').textContent}`);
check(el('daOut').textContent === '65.4' && el('dfOut').textContent === '57.3436', `反解 Da/Df：${el('daOut').textContent}/${el('dfOut').textContent}`);
check((el('rootOut').textContent || '').includes('圆齿根') && (el('rootOut').textContent || '').includes('反解'), `齿根反解读数：${el('rootOut').textContent}`);
check(metaCalled === false, '旧的 /api/spline_meta 路径不应再被调用');
check(el('items').innerHTML.split('class="row"').length - 1 === 21, `结果应渲染 21 项：${el('items').innerHTML.slice(0, 120)}`);
check(el('items').innerHTML.includes('测量跨棒距 Md'), '内花键结果应含测量跨棒距');
check(el('items').innerHTML.includes('title="stub 公式'), '公式应在每行 title 里');
// ① 单位不重复（通用规则）：值自带 `°` 时不得再追单位
check(el('items').innerHTML.includes('30°') && !el('items').innerHTML.includes('° °'),
  `值自带符号时不得再追加单位：${el('items').innerHTML.slice(0, 200)}`);
const mdAuto = el('mdOut').textContent;

// ②.5 换表达式（m2 z20 平齿根反解 + 显式 root 覆盖）→ 反解读数跟着变
el('expr').value = 'SPLINE IN M2 Z20 ALPHA30 X0 DA43.6 DF36.4 BETA0 H30';
el('expr')._fire('input', el('expr'));
await tick();
check(el('mOut').textContent === '2', `换表达式后 m 应 2：${el('mOut').textContent}`);
check((el('rootOut').textContent || '').includes('圆齿根'), `DA=43.6=m(z+1.8) 应反解圆齿根：${el('rootOut').textContent}`);
el('root').value = 'flat';
el('root')._fire('change', el('root'));
await tick();
check((el('rootOut').textContent || '').includes('平齿根') && (el('rootOut').textContent || '').includes('手选'), `显式 root 应显示手选：${el('rootOut').textContent}`);
el('root').value = 'auto';
el('root')._fire('change', el('root'));
await tick();
// 回默认表达式（后续 Dp 断言用它）
el('expr').value = 'SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30';
el('expr')._fire('input', el('expr'));
await tick();

// ③ 量棒：标准解 + 3 备选 chip，可点选；点选后 Md 重算
const chips = el('dpChoices').children;
check(chips.length === 4, `量棒应显示标准解 + 3 备选（4 个）：${chips.map((c) => c.textContent)}`);
check(chips.some((c) => c.textContent.startsWith('标准解')), `应有标准解 chip：${chips.map((c) => c.textContent)}`);
const autoVal = Number(el('dpOut').textContent);
const altChip = chips.find((c) => !near(Number(String(c.textContent).split(' ')[1]), autoVal));
check(!!altChip, '应有与标准解不同的备选 chip');
altChip.click();
await tick();
check(lastPreviewModel && near(Number(lastPreviewModel.dp), Number(String(altChip.textContent).split(' ')[1])),
  `点选备选后预览请求应带所选 Dp：${JSON.stringify(lastPreviewModel)}`);
const mdAlt = el('mdOut').textContent;
check(mdAlt !== mdAuto, `★ 选了 Dp，Md 必须重算：${mdAuto} → ${mdAlt}`);

// ③.5 手填：系列值 → 生效；非系列值 → 动态报错（后端拦）
el('dpManual').value = '1.00';
el('dpManual')._fire('input', el('dpManual'));
await tick();
check(lastPreviewModel && near(Number(lastPreviewModel.dp), 1.0), `手填 Dp 应进模型：${JSON.stringify(lastPreviewModel)}`);
check(el('dpManualOn').checked === true, '手填时应自动勾上「手填」');
check(el('mdOut').textContent !== mdAlt, `手填后 Md 应重算：${mdAlt} → ${el('mdOut').textContent}`);
el('dpManual').value = '1.03';
el('dpManual')._fire('input', el('dpManual'));
await tick();
check((el('status').textContent || '').includes('3478.9'), `非系列 Dp 应动态报错并指路：${el('status').textContent}`);
check(el('dpOut').textContent === '—', '报错时 Dp 读数应回 —');
el('dpManualOn').checked = false;
el('dpManualOn')._fire('change', el('dpManualOn'));
await tick();
check(lastPreviewModel.dp === null, '取消手填应回到标准解（dp=null）');

// ④ 表达式 KIND 与方向不一致 → 动态报错
el('expr').value = 'SPLINE EX M3 Z20 ALPHA30 X0 DA63 DF54.6 BETA0 H30';
el('expr')._fire('input', el('expr'));
await tick();
check((el('status').textContent || '').includes('KIND'), `方向不一致应动态报错：${el('status').textContent}`);

// ⑤ 切外花键：配合清单 6 项、45° 优先排前并标注；量棒面板可用（M_Re 备用）
switchCard('花键参数表_外');
el('expr').value = 'SPLINE EX M3 Z20 ALPHA30 X0 DA63 DF55.5 BETA0 H30';
el('expr')._fire('input', el('expr'));
await tick();
check(H.side.id === 'ext', '应切到外花键');
const fitCodes = el('fit').options.map((o) => o.value);
check(fitCodes.length === 6 && fitCodes.includes('js'), `外花键配合应 6 项：${fitCodes}`);
check(el('fit').value === 'h', `30/37.5° 默认外花键配合宜 h，实为 ${el('fit').value}`);
el('alpha').value = '45';
el('alpha')._fire('change', el('alpha'));
await tick();
check(el('fit').options[0].value === 'k', `45° 优先项应排前：${el('fit').options.map((o) => o.value)}`);
const labels45 = el('fit').options.map((o) => o.textContent).join('|');
check(labels45.includes('（45° 优先）'), `45° 优先项应有标注：${labels45}`);
// 用户 2026-09-27：外花键 M_Re 面板可用（能算、能显示；卡面不加行）。
check(el('dpManualOn').disabled === false && el('dpManual').disabled === false,
  '外花键量棒面板应可用（M_Re 备用）');
check((el('pinLabel').textContent || '').includes('公法线为主、跨棒距备用'),
  `外花键面板应标注主/备口径：${el('pinLabel').textContent}`);
check((el('dpLabel').textContent || '').includes('D_Re') && (el('mdLabel').textContent || '').includes('M_Re'),
  `外花键读数标签：${el('dpLabel').textContent}/${el('mdLabel').textContent}`);
check((el('pinLabel').title || '').includes("stub D'_Re 公式"),
  `外花键量棒公式应从选项表进 title：${el('pinLabel').title}`);
await H.refresh();
check(lastPreviewModel.side === 'ext' && lastPreviewModel.dp === null,
  `外花键预览不带 dp（标准解）：${JSON.stringify(lastPreviewModel)}`);
check(el('mdOut').textContent !== '—' && el('dpOut').textContent !== '—',
  `外花键应显示 M_Re 读数：D_Re=${el('dpOut').textContent} M_Re=${el('mdOut').textContent}`);
check((el('mdRange').textContent || '').includes('极限'),
  `M_Re 应显示极限：${el('mdRange').textContent}`);
const chipsExt = el('dpChoices').children;
check(chipsExt.length === 4, `外花键 M_Re 也应标准解 + 3 备选：${chipsExt.length}`);
const mdExtAuto = el('mdOut').textContent;
const altExt = chipsExt.find((c) => !near(Number(String(c.textContent).split(' ')[1]), Number(el('dpOut').textContent)));
check(!!altExt, `外花键应有备选 chip：${chipsExt.map((c) => c.textContent)}`);
if (altExt) {
  altExt.click();
  await tick();
  check(el('mdOut').textContent !== mdExtAuto,
    `★ 外花键选了 Dp，M_Re 必须重算：${mdExtAuto} → ${el('mdOut').textContent}`);
}
check(!el('items').innerHTML.includes('跨棒距'), '外花键 21 项卡面不应加 M_Re 行');
check((el('rootOut').textContent || '').includes('平齿根'), `外花键 DF=m(z−1.5) 应反解平齿根：${el('rootOut').textContent}`);
check(el('items').innerHTML.includes('公法线长度'), '外花键结果应含公法线长度');
check(el('items').innerHTML.includes('跨测齿数'), '外花键结果应含跨测齿数');

// ⑥ 出表：无 at → 待放置件；成功后自动关窗
el('ok').click();
await tick(6);
check(lastExportUrl.startsWith('/api/spline_export'), `导出 URL：${lastExportUrl}`);
check(lastExportModel && lastExportModel.side === 'ext', `导出模型：${JSON.stringify(lastExportModel)}`);
check(lastExportModel.at === null, '未填 at 时导出不应带落点（走放置态）');
check(String(lastExportModel.expr).startsWith('SPLINE EX'), `导出模型应带表达式：${lastExportModel.expr}`);
check(closed === true, '出表成功后应自动关窗');
check((el('status').textContent || '').includes('stub 已生成'), `成功提示：${el('status').textContent}`);

// ⑥.5 有 at → 直插请求带 at/rot；at 只填一个 → 动态提示、不导出
closed = false;
el('atX').value = '10';
el('atY').value = '20';
el('rot').value = '30';
el('ok').click();
await tick(6);
check(JSON.stringify(lastExportModel.at) === '[10,20]', `at 应进模型：${JSON.stringify(lastExportModel.at)}`);
check(Number(lastExportModel.rot) === 30, `rot 应进模型：${lastExportModel.rot}`);
lastExportModel = null;
el('atX').value = '10';
el('atY').value = '';
el('ok').click();
await tick(6);
check(lastExportModel === null, '半截 at 不应导出');
check((el('status').textContent || '').includes('需要 x 与 y 都填'), `半截 at 提示：${el('status').textContent}`);
el('atX').value = '';
el('atY').value = '';

// ⑥.8 ★ 用户实测回归：表达式 IN + 卡片方向「外」→ 点「出表」
// → 400 的具体原因必须出现在界面可见文本里（不得只进 console）
el('expr').value = 'SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30';
el('expr')._fire('input', el('expr'));
switchCard('花键参数表_外');
await tick();
lastExportModel = null;
el('ok').click();
await tick(6);
const stKind400 = el('status').textContent || '';
check(stKind400.includes('与卡片方向'), `出表 400 应显示具体原因：${stKind400}`);
check(stKind400.includes('IN'), `400 文案应原样透出（含 KIND/IN）：${stKind400}`);
check(!!lastExportModel, '导出请求应已发出');
// ★ 样式断言：该提示必须是醒目的 `.bad` 红框（灰字 = 没说）
check(el('status').className === 'bad', `报错应带 .bad 类：${el('status').className}`);
check(String(el('status').style.background).toLowerCase() === '#fdecea',
  `报错应有红色底纹：${el('status').style.background}`);
check(String(el('status').style.color).toLowerCase() === '#b3261e',
  `报错文字应为红色：${el('status').style.color}`);

// ⑦ 旧插件 404：直白提示、不关窗
forceExport404 = true;
closed = false;
consoleErrors.length = 0;
el('ok').click();
await tick(6);
check(!closed, '404 时不应关窗');
const st404 = el('status').textContent || '';
check(st404.includes('404') && st404.includes('重开 OCS'), `404 诊断提示：${st404}`);
check(consoleErrors.some((e) => e.includes('/api/spline_export') && e.includes('404') && e.includes('not found')),
  `console.error 应带路径+状态码+响应文本：${consoleErrors.join(' | ')}`);
forceExport404 = false;
// 预览 404 同样友好
forcePreview404 = true;
consoleErrors.length = 0;
await H.refresh();
check((el('status').textContent || '').includes('404'), `预览 404 提示：${el('status').textContent}`);
forcePreview404 = false;
// 表达式为空 → 动态提示（不需手填的常显文案）
el('expr').value = '';
el('expr')._fire('input', el('expr'));
await tick();
check((el('status').textContent || '').includes('表达式'), `空表达式动态提示：${el('status').textContent}`);

function report() {
  if (errors.length) {
    realError('智能卡片 GUI 冒烟失败：');
    for (const e of errors) realError(' - ' + e);
    process.exit(1);
  }
  console.log('智能卡片 GUI 冒烟通过');
}
report();
