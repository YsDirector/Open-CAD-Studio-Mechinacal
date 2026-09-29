// hole_gui.html 运行时/行为冒烟（node 最小 DOM 垫片 + 最小 fetch 桩）。
//
// 目的：锁住孔生成器的交互契约（node --check / el("id") 静态扫描查不出）：
//   ① 打开即「螺纹孔 M10」：自动螺纹长 15（1.5d）、自动孔深 18（有效 15+2P）、
//      读数 D1=10 / D=8.376；螺纹底孔按底孔牙深表 Ø8.5；
//   ② 简单孔：「钻头大小」= 标准麻花钻直径系列（GB/T 6135.3），两个「自动」置灰；
//   ③ 不带螺纹 + 螺栓间隙：配合可选（精/中等/粗装配）；其余子类型配合置灰；
//   ④ 沉头孔：推荐值可选（70.1 / 70.2(缺) / 6190）；钻头/自定义时要「公称」；
//   ⑤ 埋头孔：按 GB/T 152.2-2014（M1.6–M10，90°）；
//   ⑥ 贯通：孔深自动置灰；全长：孔深自动 + 螺纹数值框置灰；
//   ⑦ 确定 → POST /api/hole_export（带当前模型）。
//
// 用法：node hole_gui_smoke.mjs <hole_gui.html 路径>

import fs from 'node:fs';
import { renderZh } from './i18n_zh_fixture.mjs';

const htmlPath = process.argv[2];
if (!htmlPath) {
  console.error('usage: node hole_gui_smoke.mjs <hole_gui.html>');
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

// ── 数据表桩（形状与 /api/hole_sizes 一致）──
const COARSE = [
  { name: 'M4', d: 4, p: 0.7, d1: 3.242, d2: 3.545, fine: false },
  { name: 'M6', d: 6, p: 1.0, d1: 4.917, d2: 5.35, fine: false },
  { name: 'M8', d: 8, p: 1.25, d1: 6.647, d2: 7.188, fine: false },
  { name: 'M10', d: 10, p: 1.5, d1: 8.376, d2: 9.026, fine: false },
  { name: 'M12', d: 12, p: 1.75, d1: 10.106, d2: 10.863, fine: false },
  { name: 'M16', d: 16, p: 2.0, d1: 13.835, d2: 14.701, fine: false },
  { name: 'M20', d: 20, p: 2.5, d1: 17.294, d2: 18.376, fine: false },
];
const FINE = [
  { name: 'M10×1.25', d: 10, p: 1.25, d1: 8.647, d2: 9.188, fine: true },
  { name: 'M10×1', d: 10, p: 1.0, d1: 8.917, d2: 9.35, fine: true },
];
const TAP = [
  { name: 'M4×0.7', d: 4, p: 0.7, drill: 3.3 },
  { name: 'M6×1', d: 6, p: 1.0, drill: 5.0 },
  { name: 'M8×1.25', d: 8, p: 1.25, drill: 6.8 },
  { name: 'M10×1.5', d: 10, p: 1.5, drill: 8.5 },
  { name: 'M10×1.25', d: 10, p: 1.25, drill: 8.8 },
  { name: 'M12×1.75', d: 12, p: 1.75, drill: 10.3 },
  { name: 'M20×2.5', d: 20, p: 2.5, drill: 17.5 },
];
const DRILLS = [0.2, 0.5, 1.0, 2.0, 3.3, 4.2, 5.0, 5.1, 5.2, 6.7, 6.8, 8.5, 9.0, 10.2, 12.0, 20.0];
const CBORE = [
  { table: 'gb70', name: 'M4', d: 4, d2: 8, t: 4.6, d1: 4.5 },
  { table: 'gb70', name: 'M6', d: 6, d2: 11, t: 6.8, d1: 6.6 },
  { table: 'gb70', name: 'M10', d: 10, d2: 18, t: 11, d1: 11 },
  { table: 'gb70', name: 'M12', d: 12, d2: 20, t: 13, d1: 13.5 },
  { table: 'gb70', name: 'M16', d: 16, d2: 26, t: 17.5, d1: 17.5 },
  { table: 'gb70', name: 'M20', d: 20, d2: 33, t: 21.5, d1: 22 },
  { table: 'gb6190', name: 'M10', d: 10, d2: 18, t: 7, d1: 11 },
];
const CSINK = [
  { name: 'M1.6', d: 1.6, d2: 3.7, t: 0.95, d1: 1.8 },
  { name: 'M6', d: 6, d2: 12.85, t: 3.13, d1: 6.6 },
  { name: 'M10', d: 10, d2: 20.3, t: 4.65, d1: 11 },
];
const CLEAR = [
  { name: 'M4', d: 4, close: 4.3, normal: 4.5, loose: 4.8 },
  { name: 'M6', d: 6, close: 6.4, normal: 6.6, loose: 7.0 },
  { name: 'M8', d: 8, close: 8.4, normal: 9.0, loose: 10.0 },
  { name: 'M10', d: 10, close: 10.5, normal: 11, loose: 12 },
  { name: 'M12', d: 12, close: 13, normal: 13.5, loose: 14.5 },
];
// 螺纹体系（/api/hole_sizes.systems 的形状；内容为最小桩，覆盖 M/UN/G/NPT/ACME/Tr）
const SYS_M = {
  key: 'iso724', code: 'M', label: '公制 M（ISO 724 / GB/T 196）', standard: 'ISO 724',
  angle_deg: 60, is_pipe: false, source: 'stub', note: 'stub', units: 'mm',
  groups: [
    { key: 'coarse', label: '粗牙', rows: COARSE },
    { key: 'fine', label: '细牙', rows: FINE },
  ],
};
const SYS_UN = {
  key: 'un', code: 'UN', label: '美制统一 UN（ASME B1.1）', standard: 'ASME B1.1',
  angle_deg: 60, is_pipe: false, source: 'stub', note: 'stub',
  units: 'mm（1 in = 25.4 mm）；P = 25.4/TPI',
  groups: [
    { key: 'unc', label: 'UNC 粗牙', rows: [
      { name: '1/4-20 UNC', d: 6.35, p: 1.27, tpi: 20, d2: 5.525, d1: 4.976, drill: 5.1054 },
      { name: '5/16-18 UNC', d: 7.938, p: 1.411111, tpi: 18, d2: 7.021, d1: 6.411, drill: 6.5287 },
    ] },
    { key: 'unf', label: 'UNF 细牙', rows: [
      { name: '1/4-28 UNF', d: 6.35, p: 0.907143, tpi: 28, d2: 5.761, d1: 5.367, drill: 5.4102 },
    ] },
  ],
};
const SYS_G = {
  key: 'g', code: 'G', label: '管用平行 G（ISO 228-1 / GB/T 7307）', standard: 'ISO 228-1',
  angle_deg: 55, is_pipe: true, source: 'stub', note: 'stub', units: 'mm',
  groups: [{ key: 'standard', label: '标准', rows: [
    { name: 'G1/8', d: 9.728, p: 0.907143, tpi: 28, d2: 9.147, d1: 8.566, drill: 8.7, eff_len: 7.4 },
    { name: 'G5/8', d: 22.911, p: 1.814286, tpi: 14, d2: 21.749, d1: 20.587, drill: 20.8, eff_len: null },
  ] }],
};
const SYS_R = {
  key: 'r', code: 'R', label: '管用锥形 R（ISO 7-1 / GB/T 7306）', standard: 'ISO 7-1',
  angle_deg: 55, is_pipe: true, source: 'stub', note: 'stub', units: 'mm',
  groups: [{ key: 'standard', label: '标准', rows: [
    { name: 'R1/8', d: 9.728, p: 0.907143, tpi: 28, d2: 9.147, d1: 8.566, drill: 8.7, gauge_len: 4.0, makeup: 2.5, eff_ext: 6.5, eff_len: 7.4 },
  ] }],
};
const SYS_NPT = {
  key: 'npt', code: 'NPT', label: '美制锥管 NPT（ASME B1.20.1 / GB/T 12716）', standard: 'ASME B1.20.1',
  angle_deg: 60, is_pipe: true, source: 'stub', note: 'stub', units: 'mm',
  groups: [{ key: 'standard', label: '标准', rows: [
    { name: 'NPT1/8', d: 10.242, p: 0.940741, tpi: 27, d2: 9.489, d1: 8.737, drill: 8.481, gauge_len: 4.102, makeup: 2.822, eff_ext: 6.924, eff_len: 7.865 },
  ] }],
};
const SYS_ACME = {
  key: 'acme', code: 'ACME', label: '美制梯形 ACME（ASME B1.5）', standard: 'ASME B1.5',
  angle_deg: 29, is_pipe: false, source: 'stub', note: 'stub', units: 'mm',
  groups: [
    { key: 'general', label: '一般用途 ACME', rows: [
      { name: '1/4-16 ACME', d: 6.35, p: 1.5875, tpi: 16, d2: 5.556, d1: 4.762, drill: 4.762 },
    ] },
    { key: 'stub', label: '矮牙 Stub ACME', rows: [
      { name: '1/4-16 Stub ACME', d: 6.35, p: 1.5875, tpi: 16, d2: 5.874, d1: 5.398, drill: 5.398 },
    ] },
  ],
};
const SYS_TR = {
  key: 'tr', code: 'Tr', label: '公制梯形 Tr（ISO 2901 / GB/T 5796）', standard: 'ISO 2901',
  angle_deg: 30, is_pipe: false, source: 'stub', note: 'stub', units: 'mm',
  groups: [{ key: 'standard', label: '标准', rows: [
    { name: 'Tr8×1.5', d: 8, p: 1.5, d2: 7.25, d1: 6.5, drill: 6.5 },
  ] }],
};
const SYSTEMS = [SYS_M, SYS_UN, SYS_G, SYS_R, SYS_NPT, SYS_ACME, SYS_TR];
const SIZES = {
  ok: true,
  coarse: COARSE, fine: FINE, tap: TAP,
  systems: SYSTEMS,
  drill: { code: 'GB/T 6135.3-1996', diameters: DRILLS },
  counterbore: { code: 'GB/T 152.3-1988', rows: CBORE },
  countersink: { code: 'GB/T 152.2-2014', rows: CSINK },
  clearance: { code: 'GB/T 5277-1985', rows: CLEAR },
};

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
const SELECT_IDS = new Set(['std', 'subtype', 'size', 'nominal', 'reco', 'fit', 'range']);
global.document = {
  getElementById(id) {
    if (!els.has(id)) {
      const e = mkEl(id);
      if (SELECT_IDS.has(id)) e.tagName = 'SELECT';
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
};
// 「确定」后应像轴/标准件那样自动关窗：window.close 置位 + setTimeout 立即执行
//（范本：shaft_gui.html `exportToDrawing` 的 `window.close()` / parts_gui.html `outOfStock`）。
let closed = false;
global.window = { addEventListener() {}, close() { closed = true; } };
global.location = { search: '', href: 'http://127.0.0.1:9/hole' };
global.setTimeout = (fn) => { if (typeof fn === 'function') fn(); return 0; };
global.clearTimeout = () => {};
global.setInterval = () => 0;
global.URLSearchParams = class {
  constructor() {}
  get() { return null; }
};

// ── fetch 桩：复刻后端规则的最小实现 ─────────────────────────────
let lastPreviewModel = null;
let lastExportModel = null;
let lastExportUrl = '';
let forceExport404 = false;
let forcePreview404 = false;
const consoleErrors = [];
console.error = (...a) => { consoleErrors.push(a.join(' ')); };
function jsonResp(obj, status = 200) {
  return { ok: status < 400, status, json: async () => obj, text: async () => JSON.stringify(obj) };
}
// 旧插件（.so 没有该路由）返回纯文本 404：直接 .json() 会抛 SyntaxError —— GUI 必须先读文本。
function text404(path) {
  return {
    ok: false,
    status: 404,
    text: async () => 'not found',
    json: async () => { throw new SyntaxError(`Unexpected token 'o', "${path}" is not valid JSON`); },
  };
}
function fmt(n) {
  return Number(Number(n).toFixed(3)).toString();
}
function threadedOf(m) {
  return m.kind === 'threaded'
    || ((m.kind === 'counterbore' || m.kind === 'countersink')
        && (m.subtype === 'standard' || m.subtype === 'fine'));
}
function findSpec(m) {
  const sys = SYSTEMS.find((s) => s.key === (m.thread_system || 'iso724')) || SYS_M;
  const groups = sys.groups || [];
  const gk = m.thread_group
    || (sys.key === 'iso724' ? (m.subtype === 'fine' ? 'fine' : 'coarse') : (groups[0] && groups[0].key));
  let cands = [];
  for (const g of groups) if (g.key === gk) cands = cands.concat(g.rows);
  if (!cands.length) for (const g of groups) cands = cands.concat(g.rows);
  const byPitch = cands.find((r) => Number(r.d) === Number(m.d)
    && (m.pitch == null || Math.abs(Number(r.p) - Number(m.pitch)) < 1e-6));
  return byPitch || cands.find((r) => Number(r.d) === Number(m.d)) || null;
}
function computeValues(m) {
  const th = threadedOf(m);
  const sys = m.thread_system || 'iso724';
  let p = 0, minor = m.d, base_d = m.d, base_start = 0;
  let tpi = null, gauge_len = null, eff_len = null;
  const sysKey = m.thread_system || 'iso724';
  let bore_d = null, bore_t = null, sink_d = null, sink_t = null;
  if (th) {
    const row = findSpec(m);
    if (!row) return { error: `螺纹 ${sys} 没有 ${m.d}` };
    p = Number(row.p);
    if (sys === 'iso724') {
      minor = m.d - 1.0825 * p;
      const tap = TAP.find((r) => Number(r.d) === Number(m.d) && Math.abs(Number(r.p) - p) < 1e-6);
      base_d = tap ? Number(tap.drill) : minor;
    } else {
      minor = Number(row.d1);
      base_d = Number(row.drill != null ? row.drill : row.d1);
      tpi = row.tpi != null ? fmt(row.tpi) : null;
      gauge_len = row.gauge_len != null ? fmt(row.gauge_len) : null;
      // 自动有效长度（与后端 HoleValues.eff_len 同口径）
      eff_len = row.eff_len != null ? Number(row.eff_len) : null;
    }
  } else if (m.subtype === 'drill') {
    if (!m.drill_d || !DRILLS.includes(Number(m.drill_d))) return { error: `Ø${m.drill_d} 不在麻花钻系列` };
    base_d = Number(m.drill_d);
  } else if (m.subtype === 'clearance') {
    const row = CLEAR.find((r) => Number(r.d) === Number(m.d));
    if (!row) return { error: `间隙表没有 M${m.d}` };
    base_d = Number(row[m.fit] ?? row.normal);
  } else if (m.subtype === 'custom') {
    base_d = Number(m.custom_d);
    if (!(base_d > 0)) return { error: '自定义孔径必须 > 0' };
  } else {
    return { error: '子类型与孔类型不匹配' };
  }
  if (m.kind === 'counterbore') {
    if (m.reco === 'gb70_2') return { error: 'GB/T 152.3 无 70.2 表（标缺）' };
    const key = m.reco === 'gb6190' ? 'gb6190' : 'gb70';
    const r = CBORE.find((x) => x.table === key && Number(x.d) === Number(m.d));
    if (!r) return { error: `沉头表 ${key} 没有 M${m.d}` };
    bore_d = r.d2; bore_t = r.t; base_start = r.t;
  }
  if (m.kind === 'countersink') {
    const r = CSINK.find((x) => Number(x.d) === Number(m.d));
    if (!r) return { error: `埋头表没有 M${m.d}` };
    sink_d = r.d2; sink_t = r.t; base_start = (r.d2 - base_d) / 2;
  }
  let tl;
  if (!th) {
    tl = 0;
  } else if (m.full_thread) {
    tl = m.hole_depth ?? 0;
  } else if (m.thread_len != null) {
    tl = m.thread_len;
  } else if (sysKey === 'g' || sysKey === 'r' || sysKey === 'npt') {
    // 管螺纹自动「螺纹有效长度」= eff_len
    const row = findSpec(m);
    const v = row && row.eff_len;
    if (v == null) return { error: `${sysKey} 没有 eff_len（有效长度）—— 表外不插值` };
    tl = Number(v);
  } else {
    tl = 1.5 * m.d;
  }
  let hd = m.hole_depth;
  if (hd === null || hd === undefined) {
    if (th && m.range !== 'through') hd = tl + 2 * p;
    else return { error: '孔深不能自动' };
  }
  return {
    size_name: th ? (sys === 'iso724' ? `M${m.d}` : (findSpec(m) ? findSpec(m).name : '')) : `Ø${fmt(base_d)}`,
    threaded: th, major: fmt(th ? m.d : base_d), minor: fmt(th ? minor : base_d),
    pitch: fmt(p), tpi, gauge_len, eff_len: eff_len === null ? null : fmt(eff_len), system: sys,
    base_d: fmt(base_d), base_start: fmt(base_start),
    bore_d: bore_d === null ? null : fmt(bore_d), bore_t: bore_t === null ? null : fmt(bore_t),
    sink_d: sink_d === null ? null : fmt(sink_d), sink_t: sink_t === null ? null : fmt(sink_t),
    thread_len: fmt(tl), hole_depth: fmt(hd),
    cone_height: fmt(m.range === 'blind' ? (base_d / 2) / Math.tan((59 * Math.PI) / 180) : 0),
    warnings: [],
  };
}
global.fetch = async (u, opts = {}) => {
  const url = String(u);
  if (url.startsWith('/api/hole_sizes')) return jsonResp(SIZES);
  if (url.startsWith('/api/hole_preview')) {
    if (forcePreview404) return text404('/api/hole_preview');
    const m = JSON.parse(opts.body || '{}');
    lastPreviewModel = m;
    const v = computeValues(m);
    if (v.error) return jsonResp({ ok: false, error: v.error }, 400);
    return jsonResp({ ok: true, svg: '<svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>', values: v });
  }
  if (url.startsWith('/api/hole_export')) {
    lastExportUrl = url;
    if (forceExport404) return text404('/api/hole_export');
    lastExportModel = JSON.parse(opts.body || '{}');
    return jsonResp({ ok: true, message: 'stub 已生成' });
  }
  return jsonResp({ ok: true });
};

// ── 跑 GUI 脚本（末尾探针暴露内部状态；不改生产代码）──────────────
const probed = script.replace(/\}\)\(\);\s*$/, `;globalThis.__hole = {
  get sizes() { return sizes; },
  get kind() { return kind; },
  currentModel, refresh, syncUi, pickKind,
  setKind(k) { pickKind(k); },
};
})();`);
try {
  // 页面外链的共享助手（/ocsm_gui_common.js）：Node 里按同一目录文件先求值
  (0, eval)(renderZh(fs.readFileSync(htmlPath.replace(/[^/]+$/, 'ocsm_gui_common.js'), 'utf8')));
  (0, eval)(probed);
} catch (e) {
  errors.push('脚本求值异常: ' + (e && e.stack ? e.stack : e));
}
const tick = async (n = 4) => { for (let i = 0; i < n; i++) await new Promise((r) => setImmediate(r)); };
await tick();

const H = globalThis.__hole;
const el = (id) => document.getElementById(id);
check(!!H, '探针 __hole 未挂上（脚本初始化崩溃？）');
if (!H) report();

// ⓪ 视图 = 单选框（侧视/俯视二选一），默认侧视图；结构直接钉 HTML
check(
  html.includes('type="radio" name="view" id="viewSide"') &&
    html.includes('type="radio" name="view" id="viewTop"'),
  '视图应是 name=view 的 radio（不是 checkbox）'
);
check(el('viewSide').checked && !el('viewTop').checked, '视图默认应选中侧视图');
// 常显区（可见文本，不含 title 悬停/注释）不得含公式/术语/预先警告
const visibleText = html
  .replace(/<script[\s\S]*?<\/script>/g, ' ')
  .replace(/<style[\s\S]*?<\/style>/g, ' ')
  .replace(/<[^>]+>/g, ' ');
for (const bad of [
  '管子外径', '不是通径', '不是内径', '管子通径代号', '参考站「ACME」',
  '内、外螺纹同一个量', '两个量分开',
  '1.5×公称直径', '有效长度 + 2×螺距', '2×螺距',
  '无标准值的规格请手填', '表外会报错', '必须手填（板厚/通孔长度）',
]) {
  check(!visibleText.includes(bad), `常显区不应含公式/预先警告「${bad}」`);
}
// 公式/口径改为 title 悬停（原生 title，照 shaft/gear GUI 既有做法）
check(html.includes('title="自动有效长度：M/UN/ACME/Tr = 1.5×公称直径'), '有效长度公式应进 tLen 悬停');
check(html.includes('title="自动孔深 = 螺纹有效长度 + 2P'), '孔深公式应进 hAuto 悬停');
check(html.includes('title="沉头推荐值按螺钉类型查沉孔表'), '埋头说明应收进 reco 悬停');
// 互斥：选俯视图 → 自动取消侧视图；请求里 views 只有一项 true
el('viewTop').checked = true;
el('viewTop')._fire('change', el('viewTop'));
await tick();
check(el('viewTop').checked && !el('viewSide').checked, '选俯视图应自动取消侧视图');
await H.refresh();
check(
  lastPreviewModel.views.top === true && lastPreviewModel.views.side === false,
  `俯视图请求里 view 只带一项：${JSON.stringify(lastPreviewModel.views)}`
);
// 反向：选侧视图 → 自动取消俯视图
el('viewSide').checked = true;
el('viewSide')._fire('change', el('viewSide'));
await tick();
check(el('viewSide').checked && !el('viewTop').checked, '选侧视图应自动取消俯视图');
await H.refresh();
check(
  lastPreviewModel.views.side === true && lastPreviewModel.views.top === false,
  `侧视图请求里 view 只带一项：${JSON.stringify(lastPreviewModel.views)}`
);

// ① 打开即「螺纹孔 M10」：自动 15 / 18，读数 D1=10 / D=8.376
check(H.kind === 'threaded', `默认应为螺纹孔，实为 ${H.kind}`);
check(!!H.sizes && H.sizes.coarse.length > 0, '/api/hole_sizes 未加载');
check(el('subtype').value === 'standard', `默认子类型应为标准螺纹，实为 ${el('subtype').value}`);
check(el('size').value === '10|1.5', `默认大小应为 M10×1.5，实为 ${el('size').value}`);
check(el('fit').value === '6H', `螺纹配合默认 6H，实为 ${el('fit').value}`);
check(el('hAuto').checked && !el('hAuto').disabled, '螺纹盲孔默认应勾选自动孔深');
check(el('hDepth').value === '18', `自动孔深应 18，实为 ${el('hDepth').value}`);
check(el('hDepth').readOnly === true, '自动孔深输入框应只读');
check(el('tAuto').checked, '默认应勾选自动螺纹范围');
check(el('tLen').value === '15', `自动螺纹长应 15，实为 ${el('tLen').value}`);
check(el('majorOut').textContent === '10', `D1 应为 10，实为 ${el('majorOut').textContent}`);
check(el('minorOut').textContent === '8.376', `D 应为 8.376，实为 ${el('minorOut').textContent}`);
await H.refresh();
check(lastPreviewModel.kind === 'threaded' && lastPreviewModel.reco === 'gb70_1', '预览请求应带螺纹模型');
check(lastPreviewModel.thread_system === 'iso724' && lastPreviewModel.thread_group === 'coarse', 'M 模型应带 iso724/coarse');
check(
  el('std').options.map((o) => o.value).join(',') === 'iso724,un,g,r,npt,acme,tr',
  `标准下拉应有全部体系：${el('std').options.map((o) => o.value).join(',')}`
);

// ①.5 切系统联动：UN → 子类型 UNC/UNF、大小换列表、TPI 读数；G → 管螺纹口径提示；ACME/Tr 子类型
el('std').value = 'un';
el('std')._fire('change', el('std'));
await tick();
check(
  el('subtype').options.map((o) => o.value).join(',') === 'unc,unf',
  `UN 子类型应为 unc/unf，实为 ${el('subtype').options.map((o) => o.value).join(',')}`
);
check(el('subtype').value === 'unc', `UN 默认子类型应为 unc，实为 ${el('subtype').value}`);
check(
  el('size').options.map((o) => o.value).includes('6.35|1.27'),
  `UN 大小应含 1/4-20（6.35|1.27）：${el('size').options.map((o) => o.value)}`
);
const unSize = el('size').options.find((o) => o.value === '6.35|1.27');
check(unSize && unSize.textContent === '1/4-20 UNC', `UN 大小名应带系列：${unSize && unSize.textContent}`);
el('size').value = '6.35|1.27';
el('size')._fire('change', el('size'));
await tick();
await H.refresh();
check(lastPreviewModel.thread_system === 'un' && lastPreviewModel.thread_group === 'unc', 'UN 预览模型');
check(lastPreviewModel.subtype === 'standard', `UN 模型子类型应为 standard，实为 ${lastPreviewModel.subtype}`);
check(lastPreviewModel.pitch != null && Math.abs(lastPreviewModel.pitch - 1.27) < 1e-9, 'UN 应带 P=1.27（mm）');
check(el('majorOut').textContent === '6.35', `UN 大径应 6.35，实为 ${el('majorOut').textContent}`);
check(el('minorOut').textContent === '4.976', `UN 小径应 4.976，实为 ${el('minorOut').textContent}`);
check((el('extraOut').textContent || '').includes('20 TPI'), `UN 读数应显示 TPI：${el('extraOut').textContent}`);
check((el('extraOut').textContent || '').includes('底孔 Ø5.105'), `UN 底孔应来自钻表：${el('extraOut').textContent}`);
check(el('fit').options.map((o) => o.value).join(',') === '2B,3B', 'UN 配合应为 2B/3B');
// 切 UNF 子类型
el('subtype').value = 'unf';
el('subtype')._fire('change', el('subtype'));
await tick();
check(el('size').options.map((o) => o.value).includes('6.35|0.907143'), 'UNF 大小应含 1/4-28');
// G：正常规格不常驻提示（公式/口径在控件 title 悬停里）
el('std').value = 'g';
el('std')._fire('change', el('std'));
await tick();
check(el('threadNote').style.display === 'none', 'G1/8 正常规格不应常驻提示');
check(el('size').options.map((o) => o.value).includes('9.728|0.907143'), 'G 大小应含 G1/8');
await H.refresh();
check(el('majorOut').textContent === '9.728', `G1/8 大径应 9.728，实为 ${el('majorOut').textContent}`);
check(el('tLen').value === '7.4', `G1/8 自动有效长度应 = eff_len 7.4，实为 ${el('tLen').value}`);
check(el('hDepth').value === '9.214', `G1/8 自动孔深应 = 有效长度+2P = 9.214，实为 ${el('hDepth').value}`);
check((el('extraOut').textContent || '').includes('螺纹有效长度 7.4'), `G 读数应显示 eff_len=7.4：${el('extraOut').textContent}`);
check((el('extraOut').textContent || '').includes('底孔 Ø8.7'), `G1/8 底孔应 8.7：${el('extraOut').textContent}`);
// 动态警示：选中无标准值的 G5/8 → GUI 层醒目提示 + 模型层预览报错；换回正常规格提示消失
el('size').value = '22.911|1.814286';
el('size')._fire('change', el('size'));
await tick();
check(
  el('threadNote').style.display !== 'none' && String(el('threadNote').className).includes('bad'),
  `G5/8 应醒目弹出无标准值提示：display=${el('threadNote').style.display} cls=${el('threadNote').className}`
);
check((el('threadNote').textContent || '').includes('手填'), `G5/8 提示应说手填：${el('threadNote').textContent}`);
await H.refresh();
check((el('status').textContent || '').includes('有效长度'), `模型层应报错：${el('status').textContent}`);
el('size').value = '9.728|0.907143';
el('size')._fire('change', el('size'));
await tick();
check(el('threadNote').style.display === 'none', '换回正常规格后提示应消失');
// R：自动有效长度 = ISO 7-1 表第16栏 eff_len=7.4；NPT：自动有效长度 = 基准+装配余量+偏差(+1P)=7.865
el('std').value = 'r';
el('std')._fire('change', el('std'));
await tick();
check(el('size').options.map((o) => o.value).includes('9.728|0.907143'), 'R 大小应含 R1/8');
await H.refresh();
check(el('tLen').value === '7.4', `R1/8 自动有效长度应 = 第16栏 7.4，实为 ${el('tLen').value}`);
check(el('threadNote').style.display === 'none', 'R 正常规格不应常驻提示（来源在悬停）');
el('std').value = 'npt';
el('std')._fire('change', el('std'));
await tick();
check(el('size').options.map((o) => o.value).includes('10.242|0.940741'), 'NPT 大小应含 NPT1/8');
await H.refresh();
check(el('tLen').value === '7.865', `NPT1/8 自动有效长度应 = 7.865（含 +1P 偏差），实为 ${el('tLen').value}`);
check(el('hDepth').value === '9.746', `NPT1/8 自动孔深应 = 7.865+2P = 9.746，实为 ${el('hDepth').value}`);
check(el('threadNote').style.display === 'none', 'NPT 正常规格不应常驻提示（来源在悬停）');
// ACME：general/stub 两子类型，Stub 尺寸来自参考站口径
el('std').value = 'acme';
el('std')._fire('change', el('std'));
await tick();
check(
  el('subtype').options.map((o) => o.value).join(',') === 'general,stub',
  `ACME 子类型应为 general/stub：${el('subtype').options.map((o) => o.value).join(',')}`
);
check(
  el('threadNote').style.display === 'none',
  `ACME 不应再显示数据来源类提示：${el('threadNote').innerHTML}`
);
el('subtype').value = 'stub';
el('subtype')._fire('change', el('subtype'));
await tick();
await H.refresh();
check(el('minorOut').textContent === '5.398', `Stub ACME 小径应 5.398，实为 ${el('minorOut').textContent}`);
// Tr：公制梯形，无 TPI
el('std').value = 'tr';
el('std')._fire('change', el('std'));
await tick();
check(
  el('size').options.map((o) => o.value).includes('8|1.5'),
  `Tr 大小应含 Tr8×1.5：${el('size').options.map((o) => o.value)}`
);
await H.refresh();
check(el('majorOut').textContent === '8' && el('minorOut').textContent === '6.5', `Tr8×1.5 读数：${el('majorOut').textContent}/${el('minorOut').textContent}`);
check(!(el('extraOut').textContent || '').includes('TPI'), `Tr 不应显示 TPI：${el('extraOut').textContent}`);
// 恢复 M 默认（后续 ②~⑧ 按既有口径走）
el('std').value = 'iso724';
el('std')._fire('change', el('std'));
el('subtype').value = 'standard';
el('subtype')._fire('change', el('subtype'));
el('size').value = '10|1.5';
el('size')._fire('change', el('size'));
await tick();
check(el('size').value === '10|1.5', `恢复 M 后大小应回 M10×1.5，实为 ${el('size').value}`);

// ② 简单孔：钻头 = 标准麻花钻系列；两个自动置灰
el('kindSimple').click();
await tick();
check(H.kind === 'simple', '点「简单孔」应切类型');
const subs = el('subtype').options.map((o) => o.value);
check(subs.join(',') === 'drill,custom,clearance', `简单孔子类型应为 钻头/自定义/间隙，实为 ${subs.join(',')}`);
check(el('sizeLabel').textContent === '钻头', `钻头子类型下大小标签应为「钻头」，实为 ${el('sizeLabel').textContent}`);
const drillOpts = el('size').options.map((o) => Number(o.value));
check(drillOpts.includes(8.5) && drillOpts.includes(5.1) && drillOpts.includes(6.7), `钻头列表应含 8.5/5.1/6.7：${drillOpts}`);
check(el('hAuto').disabled && !el('hAuto').checked, '简单孔：孔深自动应置灰且取消');
check(el('tAuto').disabled && !el('tAuto').checked, '简单孔：螺纹范围自动应置灰且取消');
check(el('tLen').disabled, '简单孔：螺纹范围数值框应置灰');
check(el('fit').disabled, '简单孔非间隙子类型：配合应置灰');
await H.refresh();
check(
  lastPreviewModel.subtype === 'drill' && lastPreviewModel.drill_d != null && lastPreviewModel.kind === 'simple',
  `预览请求应带 simple/drill+drill_d：${JSON.stringify(lastPreviewModel)}`
);

// ③ 螺栓间隙：配合可选（精/中等/粗）
el('subtype').value = 'clearance';
el('subtype')._fire('change', el('subtype'));
await tick();
const fits = el('fit').options.map((o) => o.value);
check(fits.join(',') === 'close,normal,loose', `间隙配合应为 精/中等/粗，实为 ${fits.join(',')}`);
check(!el('fit').disabled, '螺栓间隙下配合应可选');
el('fit').value = 'normal';
el('fit')._fire('change', el('fit'));
await tick();
check(H.currentModel().fit === 'normal', '配合选择应进入模型');
await H.refresh();
check((el('extraOut').textContent || '').includes('底孔 Ø11'), '读数应显示间隙底孔 Ø11：' + el('extraOut').textContent);

// ④ 沉头孔：推荐值可选；钻头 + 公称两个选择器；沉孔 Ø18×11
H.setKind('counterbore');
await tick();
check(el('threadRow').style.display !== 'none', '沉头孔应显示带螺纹开关');
check(el('threadToggle').checked, '沉头孔默认带螺纹');
check(el('reco').style.display !== 'none', '沉头孔应显示推荐值');
const recoOpts = el('reco').options.map((o) => o.value);
check(recoOpts.includes('gb70_1') && recoOpts.includes('gb70_2'), `推荐值应有 70.1/70.2：${recoOpts}`);
el('threadToggle').checked = false;
el('threadToggle')._fire('change', el('threadToggle'));
await tick();
check(el('subtype').options.map((o) => o.value).join(',') === 'drill,custom,clearance', '不带螺纹子类型应为 钻头/自定义/间隙');
check(el('nominal').style.display !== 'none', '沉头孔 + 钻头应显示公称');
check(Number(el('nominal').value) === 10, `公称默认应 M10，实为 ${el('nominal').value}`);
await H.refresh();
check(lastPreviewModel.kind === 'counterbore' && lastPreviewModel.reco === 'gb70_1', '沉头预览模型应带 reco=gb70_1');
check((el('extraOut').textContent || '').includes('沉孔 Ø18 × 11'), '读数应显示沉孔 Ø18×11：' + el('extraOut').textContent);
// 70.2 → 明确报错（标缺）
el('reco').value = 'gb70_2';
el('reco')._fire('change', el('reco'));
await tick();
await H.refresh();
check((el('status').textContent || '').includes('70.2'), '选 70.2 应显示标缺报错：' + el('status').textContent);
el('reco').value = 'gb70_1';
el('reco')._fire('change', el('reco'));
await tick();

// ⑤ 埋头孔：GB/T 152.2-2014（M1.6–M10，90°）
H.setKind('countersink');
await tick();
check(!html.includes('id="sinkNote"'), '埋头说明不应常驻（已收进 reco 悬停）');
check(el('reco').style.display === 'none', '埋头孔不显示沉头推荐值下拉');
el('threadToggle').checked = false;
el('threadToggle')._fire('change', el('threadToggle'));
await tick();
await H.refresh();
check((el('extraOut').textContent || '').includes('埋头 Ø20.3'), '埋头读数应 Ø20.3：' + el('extraOut').textContent);

// ⑥ 贯通置灰 / 全长置灰（回到螺纹孔）
H.setKind('threaded');
await tick();
el('range').value = 'through';
el('range')._fire('change', el('range'));
await tick();
check(el('hAuto').disabled, '贯通：孔深自动应置灰');
el('range').value = 'blind';
el('range')._fire('change', el('range'));
await tick();
el('tFull').checked = true;
el('tFull')._fire('change', el('tFull'));
await tick();
check(el('hAuto').disabled, '全长：孔深自动应置灰');
check(el('tLen').disabled, '全长：螺纹数值框应置灰');
el('tFull').checked = false;
el('tFull')._fire('change', el('tFull'));
await tick();

// ⑦ 确定 → POST /api/hole_export（放置态）+ 自动关窗（与轴/标准件同款）
el('ok').click();
await tick(6);
check(lastExportUrl.startsWith('/api/hole_export'), `导出 URL 应为 /api/hole_export，实为 ${lastExportUrl}`);
check(!!lastExportModel && lastExportModel.kind === 'threaded', `导出模型 kind 应为 threaded：${JSON.stringify(lastExportModel)}`);
check(lastExportModel.views && lastExportModel.views.side === true, '导出模型应带视图开关');
check(lastExportModel.views.side !== lastExportModel.views.top, '导出模型 views 只应有一项 true');
// 无显式 at → 后端建块 + 登记待放置件（进入放置态，鼠标跟随预览）
check(lastExportModel.at === undefined || lastExportModel.at === null, 'GUI 确定不应带 at（走待放置件/放置态）');
// 自动关窗（axis/parts 同款：成功后 300ms window.close）
check(closed === true, '点「确定」后 GUI 应自动关闭（window.close）');
check((el('status').textContent || '').includes('stub 已生成'), '导出成功提示应显示：' + el('status').textContent);

// ⑧ 旧插件 404（用户实测：POST /api/hole_export → 404 "not found"）：
//   GUI 应读文本给出可诊断提示、不抛 SyntaxError、不关窗（照轴/标准件的 r.ok 判据）。
forceExport404 = true;
closed = false;
consoleErrors.length = 0;
el('ok').click();
await tick(6);
check(!closed, '404 时不应关窗');
const st404 = el('status').textContent || '';
check(st404.includes('404') && st404.includes('重开 OCS'), '404 应显示诊断提示：' + st404);
// ★ 样式断言：错误必须是 `.bad` 红框（共享助手统一；灰字 = 没说）
check(el('status').className === 'bad', '404 提示应带 .bad 类：' + el('status').className);
check(String(el('status').style.background).toLowerCase() === '#fdecea',
  '404 提示应有红色底纹：' + el('status').style.background);
check(String(el('status').style.color).toLowerCase() === '#b3261e',
  '404 提示文字应为红色：' + el('status').style.color);
check(
  consoleErrors.some((e) => e.includes('/api/hole_export') && e.includes('404') && e.includes('not found')),
  'console.error 应带路径+状态码+响应文本：' + consoleErrors.join(' | ')
);
forceExport404 = false;
// 预览端点 404 同样友好（红色样式同样要到位）
forcePreview404 = true;
consoleErrors.length = 0;
await H.refresh();
check((el('status').textContent || '').includes('404'), '预览 404 应显示提示：' + el('status').textContent);
check(el('status').className === 'bad' && String(el('status').style.background).toLowerCase() === '#fdecea',
  '预览 404 也应是 .bad 红框：' + el('status').className + '/' + el('status').style.background);
forcePreview404 = false;

function report() {
  if (errors.length) {
    console.error('孔 GUI 冒烟失败：');
    for (const e of errors) console.error(' - ' + e);
    process.exit(1);
  }
  console.log('孔 GUI 冒烟通过');
}
report();
