// gear_gui.html 运行时冒烟（node 最小 DOM 垫片 + fetch 桩）。
//
// 目的：锁住「齿轮 / 花键」模式切换的交互契约（node --check 与 el("id") 静态扫描查不出）：
//   ① 默认齿轮模式：标准号/齿廓/d_B/hf/ρf/cF 行隐藏，c/β/倒角行显示；
//   ② 勾「花键模式」+ GB：花键行显示、GB 下 **d_B 行隐藏**（GB 无基准直径概念），
//      **模数 m 行显示**（GB 为 m/z/x 驱动，没模数没法用）；
//   ③ 切 DIN：d_B 行显示、m 行显示（d_B+m）、齿廓切到 DIN30、预设系数写回；
//   ④ 全程无异常，防抖回调能跑。
//
// 用法：node gear_gui_smoke.mjs <gear_gui.html 路径>

import fs from 'node:fs';
import { renderZh } from './i18n_zh_fixture.mjs';
import path from 'node:path';

const htmlPath = process.argv[2];
if (!htmlPath) {
  console.error('usage: node gear_gui_smoke.mjs <gear_gui.html>');
  process.exit(2);
}
const html = renderZh(fs.readFileSync(htmlPath, 'utf8'));
const m = html.match(/<script>([\s\S]*?)<\/script>/);
if (!m) {
  console.error('no <script> found');
  process.exit(2);
}
const script = m[1];

// ── 真实表数据：直接读与 Rust 侧 include_str! 同一批 CSV（避免桩与表漂移）──
// 与 Rust split_csv_line/csv_decimal 同口径：引号内的逗号不拆，十进制逗号转点。
function splitCsvLine(line) {
  const out = []; let cur = ''; let quoted = false;
  for (const ch of line) {
    if (ch === '"') quoted = !quoted;
    else if (ch === ',' && !quoted) { out.push(cur); cur = ''; }
    else cur += ch;
  }
  out.push(cur);
  return out;
}
const dec = (s) => {
  const t = String(s).trim();
  if (t === '') return null;   // 与 Rust csv_decimal 同口径：空格子解析失败（非 0）
  const v = Number(t.replace(',', '.'));
  return Number.isFinite(v) ? v : null;
};
function readCsv(file) {
  return fs.readFileSync(file, 'utf8').split('\n')
    .filter((line) => line.trim() !== '' && !line.startsWith('#'))
    .map((line) => splitCsvLine(line.trimEnd()))
    .filter((f) => f[0].trim() !== 'page');
}
const csvDir = path.join(path.dirname(path.resolve(htmlPath)), '..', 'assets');
// NF E22-141 尺寸表：page,table_no,source,m,A,N,…,x(第 9 列)；x 缺省 = p18 行。
const NF_CSV = readCsv(path.join(csvDir, 'nf_e22141_dims.csv')).map((f) => ({
  a: dec(f[4]), m: dec(f[3]), z: Number(f[5]), x: dec(f[8]), page: Number(f[0]),
}));
// DIN 5480-2 名义表：page,m,table_no,d_B,z,…,x1_m(第 8 列，x_m)；载荷里 x=x_m/m。
const DIN_CSV = readCsv(path.join(csvDir, 'din5480_2_nominal.csv')).map((f) => {
  const m = dec(f[1]);
  return { db: dec(f[3]), m, z: Number(f[4]), x: dec(f[7]) / m, page: Number(f[0]) };
});
// 逐 m 的候选集合（升序去重），测试与页面 datalist 逐项对比。
const byModule = (rows, key) => {
  const map = new Map();
  for (const r of rows) {
    const k = String(Number(r.m));
    if (!map.has(k)) map.set(k, new Set());
    map.get(k).add(Number(r[key]));
  }
  return new Map([...map].map(([k, set]) => [k, [...set].sort((a, b) => a - b)]));
};
const NF_BY_M = byModule(NF_CSV, 'a');
const DIN_BY_M = byModule(DIN_CSV, 'db');

const errors = [];
const check = (cond, msg) => { if (!cond) errors.push(msg); };

// ── 最小 DOM 垫片 ────────────────────────────────────────────────
const els = new Map();
function mkEl(id) {
  const el = {
    id, tagName: 'DIV', type: '', dataset: {}, style: {}, value: '', checked: false,
    disabled: false, textContent: '', children: [], options: [], _handlers: {}, _innerHTML: '',
    classList: {
      _on: new Set(),
      add(c) { this._on.add(c); },
      remove(c) { this._on.delete(c); },
      contains(c) { return this._on.has(c); },
    },
    addEventListener(ev, fn) { (this._handlers[ev] ||= []).push(fn); },
    _fire(ev, target) { (this._handlers[ev] || []).forEach((f) => f({ target, type: ev })); },
    appendChild(c) { this.children.push(c); return c; },
    removeChild(c) { this.children = this.children.filter((x) => x !== c); },
    querySelectorAll() { return []; },
    querySelector() { return null; },
    setAttribute(k, v) { this._attrs = this._attrs || {}; this._attrs[k] = v; },
    closest() { return null; },
    focus() { return; },
    remove() { return; },
    click() { this._fire('click', this); },
  };
  Object.defineProperty(el, 'innerHTML', {
    get() { return el._innerHTML; },
    set(v) { el._innerHTML = String(v); if (v === '') el.children = []; },
  });
  return el;
}
global.document = {
  getElementById(id) { if (!els.has(id)) els.set(id, mkEl(id)); return els.get(id); },
  querySelectorAll() { return []; },
  querySelector() { return null; },
  createElement(tag) { const e = mkEl('el'); e.tagName = String(tag).toUpperCase(); return e; },
  createElementNS() { return mkEl('ns'); },
  createTextNode(t) { const e = mkEl('text'); e.textContent = String(t); return e; },
  addEventListener() {},
};
global.window = { addEventListener() {}, close() {} };
global.location = { search: '', href: 'http://127.0.0.1/gear' };
const timers = [];
global.setTimeout = (fn) => { timers.push(fn); return timers.length; };
global.clearTimeout = () => {};
global.setInterval = () => 0;

// ── /api/parts 桩（预设 + 真实 DIN/NF 名义表，与 assets CSV 同源；引擎数据在顶层 spline_engine） ──
const PARTS = {
  spline_engine: {
      invol_presets: [
        { code: 'GB30P', std: 'GB', profile: '30平齿根', alpha: 30, ha: 0.5, hf: 0.75, rho: 0.2, cf: 0.1 },
        { code: 'GB30R', std: 'GB', profile: '30圆齿根', alpha: 30, ha: 0.5, hf: 0.9, rho: 0.4, cf: 0.1 },
        { code: 'GB375R', std: 'GB', profile: '37.5圆齿根', alpha: 37.5, ha: 0.45, hf: 0.7, rho: 0.3, cf: 0.1 },
        { code: 'GB45R', std: 'GB', profile: '45圆齿根', alpha: 45, ha: 0.4, hf: 0.6, rho: 0.25, cf: 0.1 },
        { code: 'DIN30', std: 'DIN', profile: 'DIN30', alpha: 30, ha: 0.45, hf: 0.55, rho: 0.16, cf: 0.1 },
        { code: 'NFP', std: 'NF', profile: 'NF平齿根', alpha: 20, ha: 0.2, hf: 1.0, rho: 0.3, cf: 0.1 },
        { code: 'NFR', std: 'NF', profile: 'NF圆齿根', alpha: 20, ha: 0.2, hf: 1.147, rho: 0.528, cf: 0.1 },
        { code: 'ANSI30P', std: 'ANSI', profile: 'ANSI30平齿根齿侧', alpha: 30, ha: 0.5, hf: 0.675, rho: 0, cf: 0 },
        { code: 'ANSI30PM', std: 'ANSI', profile: 'ANSI30平齿根外径', alpha: 30, ha: 0.5, hf: 0.5, rho: 0, cf: 0 },
        { code: 'ANSI30R', std: 'ANSI', profile: 'ANSI30圆齿根齿侧', alpha: 30, ha: 0.5, hf: 0.9, rho: 0, cf: 0 },
        { code: 'ANSI375R', std: 'ANSI', profile: 'ANSI37.5圆齿根齿侧', alpha: 37.5, ha: 0.5, hf: 0.65, rho: 0, cf: 0 },
        { code: 'ANSI45R', std: 'ANSI', profile: 'ANSI45圆齿根齿侧', alpha: 45, ha: 0.5, hf: 0.5, rho: 0, cf: 0 },
      ],
      // 各体系候选（与 invol_spline.rs / detail.rs catalog 同形）
      gb_modules: [0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.5, 3, 4, 5, 6, 8, 10],
      din_modules: [0.5, 0.6, 0.75, 0.8, 1, 1.25, 1.5, 1.75, 2, 2.5, 3, 4, 5, 6, 8, 10],
      nf_modules: [0.5, 0.75, 1, 1.25, 1.667, 2.5, 3.75, 5, 7.5, 10],
      din_nominal: DIN_CSV,
      nf_nominal: NF_CSV,
  },
};
const INFO = {
  ok: true, mode: 'spline', kind: 'internal', kind_label: '内花键', view: 'front',
  std: 'GB', std_code: 'GB/T 3478.1-2008', profile: '30圆齿根', m: 3, z: 20, x: 0, alpha: 30,
  d: 60, db: 51.9615, da: 65.4, df: 57.344, d_b: null, origin: null, rho: 1.2, cf: 0.3,
  internal_major: 65.4, internal_minor: 57.344, block: 'B', spec: 'S', notes: ['内花键无侧视图（同内齿轮：剖视 + 端视）'],
};
// 花键模式的 info 桩：按查询串回填 m/z/x/kind/std，并给出统一齿形表达式（真实数值由 Rust 往返测试覆盖）。
const SPLINE_INFO_BASE = {
  ok: true, error: null, mode: 'spline', view: 'front', view_label: '端视图',
  std_code: 'GB/T 3478.1-2008', profile: '30圆齿根', d: 60, db: 51.9615,
  d_b: null, a: null, origin: null, rho: 1.2, cf: 0.3, pitch: null, ps: null, pitch_label: null,
  dfe: null, dfi: null, sv_min: null, internal_major: 65.4, internal_minor: 57.3436,
  inspection: null, block: 'B', spec: 'S', notes: [],
};
function splineInfoFor(url) {
  const q = new URL(url, 'http://x').searchParams;
  const kind = q.get('kind') || 'external';
  const std = q.get('std') || 'GB';
  const m = Number(q.get('m')) || 3;
  const z = Number(q.get('z')) || 20;
  const x = Number(q.get('x')) || 0;
  const alpha = Number(q.get('alpha')) || 30;
  const h = Number(q.get('h')) || 30;
  const internal = kind === 'internal';
  const da = internal ? 65.4 : 63;
  const df = internal ? 57.3436 : 54.6;
  return {
    ...SPLINE_INFO_BASE, kind, kind_label: internal ? '内花键' : '外花键', std, m, z, x, alpha,
    da, df,
    expr: 'SPLINE ' + (internal ? 'IN' : 'EX') + ' M' + m + ' Z' + z + ' ALPHA' + alpha
      + ' X' + x + ' DA' + da + ' DF' + df + ' BETA0 H' + h,
  };
}
const fetchLog = [];
global.fetch = async (u, opts) => {
  const t = String(u);
  fetchLog.push({ url: t, body: opts && opts.body ? String(opts.body) : null });
  if (t.startsWith('/api/parts')) {
    return { ok: true, json: async () => PARTS, text: async () => JSON.stringify(PARTS) };
  }
  if (t.startsWith('/api/gear_info')) {
    const info = t.includes('mode=spline') ? splineInfoFor(t) : INFO;
    return { ok: true, json: async () => info, text: async () => JSON.stringify(info) };
  }
  if (t.startsWith('/api/gear_svg')) return { ok: true, text: async () => '<svg></svg>' };
  if (t === '/api/gear_export') {
    return { ok: true, json: async () => ({ ok: true, message: '已生成' }), text: async () => '{"ok":true}' };
  }
  if (t.startsWith('/api/page_ping')) return { ok: true, json: async () => ({}), text: async () => '{}' };
  return { ok: false, json: async () => ({ ok: false }), text: async () => '' };
};

// ── 跑 GUI 脚本 ─────────────────────────────────────────────────
try {
  // 页面外链的共享助手（/ocsm_gui_common.js）：Node 里按同一目录文件先求值
  (0, eval)(renderZh(fs.readFileSync(htmlPath.replace(/[^/]+$/, 'ocsm_gui_common.js'), 'utf8')));
  (0, eval)(script);
} catch (e) {
  errors.push('脚本求值异常: ' + (e && e.stack ? e.stack : e));
}
const flush = async () => {
  for (let i = 0; i < 6; i++) await new Promise((r) => setImmediate(r));
};
await flush();

const el = (id) => document.getElementById(id);
const numOf = (id) => parseFloat(el(id).value);

// ⓪ 信息分层（照 hole_gui 口径）：常显区（可见文本；剥标签、title 悬停不计）不得含
//    公式/口径/来源/预先警告；公式与口径进 title（原生 title，不自造 tooltip）。
const visibleText = html
  .replace(/<script[\s\S]*?<\/script>/g, ' ')
  .replace(/<style[\s\S]*?<\/style>/g, ' ')
  .replace(/<[^>]+>/g, ' ');
for (const bad of [
  '会明确报错', 'NF 已入库', 'ANSI 已接入', 'd_B 只属于 DIN 5480', 'GB/T 3478 下不给',
  'M 与 DP 只能选一个', '径节系列下拉暂不做', '0.38m', '不随 α 自动变',
  '右旋为正', '三条细实线', '默认 10m', '默认 15m', 'round(0.6m)',
  'ByLayer', '不含尺寸标注', '统一齿形表达式', '最小口径', '不存在侧视图',
]) {
  check(!visibleText.includes(bad), `常显区不应含公式/口径/预先警告「${bad}」`);
}
// 正向：公式/口径确实进了 title（静态控件 + 动态写入）。
check(html.includes('title="勾选 = 渐开线花键'), '花键模式说明应进 splineMode 的 title');
check(html.includes('title="m = 25.4/DP'), 'DP 换算式 m=25.4/DP 应进 dp 的 title');
check(html.includes('title="自动 = round(0.6m)'), '倒角公式应进 chamfer 的 title');
check(html.includes('title="模数制用 m；径节制用 DP'), 'M/DP 口径应进 sysSel 的 title');
check(html.includes('title="生成物：轮廓'), '生成物/图层口径应进「生成到图纸」title');

// ① 默认齿轮模式：c/beta/chamfer/sys 行在，花键行不在；M 体系显示模数行、隐藏径节行
check(el('stdLabel').style.display === 'none', '齿轮模式下标准号行应隐藏');
check(el('cLabel').style.display !== 'none', '齿轮模式下顶隙行应显示');
check(el('hfLabel').style.display === 'none', '齿轮模式下 hf 行应隐藏');
check(el('sysLabel').style.display !== 'none', '齿轮模式下体系行应显示');
check(el('mLabel').style.display !== 'none', 'M 体系应显示模数行');
check(el('dpLabel').style.display === 'none', 'M 体系应隐藏径节行');

// ①.5 切 DP：径节行显示、模数行隐藏；倒角按 m=25.4/DP 换算
el('sysSel').value = 'DP';
el('dp').value = '8';
try { el('sysSel')._fire('change', el('sysSel')); } catch (e) { errors.push('体系切换异常: ' + e); }
await flush();
check(el('dpLabel').style.display !== 'none', 'DP 体系应显示径节行');
check(el('mLabel').style.display === 'none', 'DP 体系应隐藏模数行');
check(Math.abs(numOf('m') - 3.175) < 1e-9, 'DP8 应换算 m=3.175，实际 ' + numOf('m'));
check(el('chamfer').value === '2', 'DP8 → C=round(0.6×3.175)=2，实际 ' + el('chamfer').value);
// 切回 M
el('sysSel').value = 'M';
try { el('sysSel')._fire('change', el('sysSel')); } catch (e) { errors.push('体系回切异常: ' + e); }
await flush();
check(el('mLabel').style.display !== 'none' && el('dpLabel').style.display === 'none', '切回 M 应恢复模数行');

// ①.6 轴生成器表达式（齿轮模式）：轴段语法一行、不带 view/at；复制按钮写入剪贴板
// （DOM 垫片不会读 HTML 默认值，这里补上齿轮模式的其余默认项）
el('ha').value = '1';
el('c').value = '0.25';
el('betaD').value = '0';
el('betaM').value = '0';
el('betaS').value = '0';
el('x').value = '0';
el('m').value = '3';
el('z').value = '20';
el('h').value = '30';
el('alpha').value = '20';
try { el('m')._fire('input', el('m')); } catch (e) { errors.push('表达式同步异常: ' + e); }
await flush();
check(el('exprPreview').value === 'GEAR EX M3 Z20 ALPHA20 X0 DA66 DF52.5 BETA0 H30',
  '齿轮表达式应为统一格式 GEAR EX … DA66 DF52.5 …，实际 ' + JSON.stringify(el('exprPreview').value));
check(el('exprHint').textContent.includes('可直接粘贴'), '齿轮表达式提示应说明可直接粘贴：' + el('exprHint').textContent);
check(!/view|\bat\b/i.test(el('exprPreview').value), '表达式不应带 view/at：' + el('exprPreview').value);
Object.defineProperty(globalThis, 'navigator', {
  value: { clipboard: { writeText: async (t) => { globalThis.__copied = t; } } },
  configurable: true,
});
globalThis.__copied = '';
el('exprCopy')._fire('click', el('exprCopy'));
await flush();
check(globalThis.__copied === 'GEAR EX M3 Z20 ALPHA20 X0 DA66 DF52.5 BETA0 H30',
  '复制按钮应写入剪贴板，实际 ' + JSON.stringify(globalThis.__copied));

// ①.7 DP 径节制：轴段 GEAR 只认 M，表达式按 m = 25.4/DP 换算
el('sysSel').value = 'DP';
el('dp').value = '8';
try { el('sysSel')._fire('change', el('sysSel')); } catch (e) { errors.push('DP 表达式同步异常: ' + e); }
await flush();
check(el('exprPreview').value === 'GEAR EX M3.175 Z20 ALPHA20 X0 DA69.85 DF55.5625 BETA0 H31.75',
  'DP 表达式应按 m=25.4/DP 换算成 M，实际 ' + JSON.stringify(el('exprPreview').value));
check(el('exprHint').textContent.includes('已换算成模数 M'), 'DP 表达式提示应说明换算：' + el('exprHint').textContent);
check(html.includes('title="m = 25.4/DP'), 'DP 换算公式应进 dp 的 title（静态）');

// ② 花键模式 + GB：花键行显示、d_B 行隐藏
el('stdSel').value = 'GB';
el('splineMode').checked = true;
try { el('splineMode')._fire('change', el('splineMode')); } catch (e) { errors.push('模式切换异常: ' + e); }
await flush();
check(el('alpha').readOnly === true, '花键模式 α 应由齿廓预设决定（只读）');
check(el('hf').readOnly === true, '花键模式 hf 应由齿廓预设决定（只读）');
check(el('x').readOnly === true, 'GB 下 x 应锁死（GB/T 3478 基本齿廓不含变位）');
check(el('x').value === '0', 'GB 下 x 应恒为 0，实际 ' + el('x').value);
check((el('x').title || '').includes('GB/T 3478') && (el('x').title || '').includes('不可编辑'),
  'GB 下「不变位、x 不可编辑」口径应进 x 的 title：' + el('x').title);
check(el('stdLabel').style.display !== 'none', '花键模式下标准号行应显示');
check(el('dbLabel').style.display === 'none', 'GB 下 d_B 行应隐藏（本体系不用 d_B）');
check(el('cLabel').style.display === 'none', '花键模式下顶隙行应隐藏');
check(el('profileSel').value === 'GB30R', 'GB 默认齿廓应为 GB30R，实际 ' + el('profileSel').value);
check(Math.abs(numOf('hf') - 0.9) < 1e-9, 'GB30R hf 预设应 0.9，实际 ' + numOf('hf'));
check(el('mLabel').style.display !== 'none', 'GB 花键必须显示模数 m 行（m/z/x 驱动）');
check(el('mLabel').textContent.includes('模数'), 'GB 花键模数标签应为模数[m/Mn]：' + el('mLabel').textContent);

// ③ 切 DIN：d_B 行显示；齿廓 DIN30 + 系数预设
el('stdSel').value = 'DIN';
try { el('stdSel')._fire('change', el('stdSel')); } catch (e) { errors.push('标准切换异常: ' + e); }
await flush();
check(el('dbLabel').style.display !== 'none', 'DIN 下 d_B 行应显示');
check(el('mLabel').style.display !== 'none', 'DIN 下模数 m 行应显示（d_B + m 两参数体系）');
check(el('profileSel').value === 'DIN30', 'DIN 齿廓应 DIN30，实际 ' + el('profileSel').value);
check(Math.abs(numOf('hf') - 0.55) < 1e-9, 'DIN30 hf 预设应 0.55');

// ③.5 NF：A 行显示、标签为公称直径 A；齿廓 NFP、预设系数；A=80 自动带出 m/z。
el('stdSel').value = 'NF';
try { el('stdSel')._fire('change', el('stdSel')); } catch (e) { errors.push('NF 切换异常: ' + e); }
await flush();
check(el('dbLabel').style.display !== 'none', 'NF 下 A（基准直径）行应显示');
check(el('dbLabel').textContent.includes('公称直径 A'), 'NF 标签应为公称直径 A：' + el('dbLabel').textContent);
check((el('stdSel').title || '').includes('A 主参数'), 'NF 标准口径应进 stdSel 的 title：' + el('stdSel').title);
check(el('profileSel').value === 'NFP', 'NF 默认齿廓应为 NFP，实际 ' + el('profileSel').value);
check(Math.abs(numOf('hf') - 1.0) < 1e-9, 'NFP hf 预设应 1.0，实际 ' + numOf('hf'));
check(el('mLabel').style.display !== 'none', 'NF 下模数 m 行应显示（A + m 两参数体系）');
// 新联动顺序：先定 m（下拉），A 候选只列该 m 的表内值；A=80 在 m=3.75 下带出 z=19、表值 x=0.967。
el('mSel').value = '3.75';
try { el('mSel')._fire('change', el('mSel')); } catch (e) { errors.push('NF m 选择异常: ' + e); }
await flush();
check(Math.abs(numOf('m') - 3.75) < 1e-9, 'mSel 选 3.75 应填 m=3.75，实际 ' + el('m').value);
el('db').value = '80';
try { el('db')._fire('change', el('db')); } catch (e) { errors.push('NF A 联动异常: ' + e); }
await flush();
check(el('m').value === '3.75' && el('z').value === '19',
  'NF m=3.75/A=80 应带出 z=19，实际 ' + el('m').value + '/' + el('z').value);
check(Math.abs(numOf('x') - 0.967) < 1e-3,
  'NF A=80/m=3.75/N=19 应显示表值 x=0.967（p18 空 x 行不得盖成 0），实际 ' + el('x').value);
// 按 m 过滤：datalist 只列当前 m 的 A（m=3.75 → 21 个 30..150；不含 m=5 的 160）。
const a375 = el('nfAList').children.map((o) => Number(o.value)).sort((a, b) => a - b);
check(a375.length === 21 && a375[0] === 30 && a375[20] === 150,
  'NF m=3.75 的 A 候选应 21 个（30..150），实际 ' + a375.length + '：' + a375.join('|'));
check(a375.indexOf(160) < 0, 'NF m=3.75 的 A 候选不应含 m=5 的 160：' + a375.join('|'));
check(el('dbListHint').textContent.includes('候选 21 个'),
  '候选提示应写出 m=3.75 的 21 个：' + el('dbListHint').textContent);
// 花键统一表达式（用户报的「花键模式没有轴生成器表达式」）：info 到达后非空、可复制。
try { el('m')._fire('blur', el('m')); } catch (e) { errors.push('NF 表达式刷新异常: ' + e); }
await flush();
check(el('exprPreview').value.startsWith('SPLINE ') && el('exprPreview').value.includes('Z19'),
  'NF 花键应给出统一齿形表达式（含当前 z=19），实际 ' + JSON.stringify(el('exprPreview').value));
check(el('exprCopy').disabled === false, 'NF 花键表达式非空时复制按钮应可用');
check((el('exprPreview').title || '').includes('统一齿形表达式'), 'NF 花键应提示统一齿形表达式（title）：' + el('exprPreview').title);
// ANSI：基准直径行隐藏；径节输入切到 17 项 A/B 下拉 + 自定义；hf/rho/cf 隐藏。
el('stdSel').value = 'ANSI';
try { el('stdSel')._fire('change', el('stdSel')); } catch (e) { errors.push('ANSI 切换异常: ' + e); }
await flush();
check(el('dbLabel').style.display === 'none', 'ANSI 下基准直径行应隐藏');
check((el('stdSel').title || '').includes('径节 P'), 'ANSI 标准口径应进 stdSel 的 title：' + el('stdSel').title);
check(!(el('stdSel').title || '').includes('未实现'), 'ANSI 口径不应再含「未实现」：' + el('stdSel').title);
check(el('mLabel').style.display !== 'none', 'ANSI 下应显示径节输入行（m 行）');
check(el('mLabel').textContent.includes('径节[P]'), 'ANSI 标签应为径节[P]：' + el('mLabel').textContent);
check(el('hfLabel').style.display === 'none', 'ANSI 下 hf 行应隐藏（Table 2 公式算）');
check(el('rhoLabel').style.display === 'none', 'ANSI 下 rho 行应隐藏');
check(el('cfLabel').style.display === 'none', 'ANSI 下 cf 行应隐藏');
check(el('profileSel').value === 'ANSI30P', 'ANSI 默认齿廓应为 ANSI30P，实际 ' + el('profileSel').value);
// 下拉按当前列过滤（用户定案 A）：默认 ANSI30P（A 列 2.5/5—32/64）应 12 项。
const pitchOpts = el('mSel').children.map((o) => o.value).filter((v) => v !== 'custom');
check(pitchOpts.length === 12, 'ANSI30P 径节下拉应按列过滤为 12 项，实际 ' + pitchOpts.length);
check(pitchOpts[0] === '2.5/5' && pitchOpts[11] === '32/64',
  'ANSI30P 下拉应 2.5/5…32/64，实际 ' + pitchOpts[0] + '|' + pitchOpts[pitchOpts.length - 1]);
// 列界外自定义 P → 与后端同一文案；生成按钮禁用
el('m').value = '40';
el('m')._fire('input', el('m'));
await flush();
check(el('warn').textContent.includes('ANSI B92.1 Table 2')
  && el('warn').textContent.includes('30° 平齿根 / 齿侧配合')
  && el('warn').textContent.includes('2.5/5 — 32/64')
  && el('warn').textContent.includes('40/80') && el('warn').textContent.includes('超出上限'),
  'A 列 P=40/80 应红字点名 Table 2 列与范围：' + el('warn').textContent);
check(el('out').disabled === true, '列界外应禁用生成按钮');
// 换 45° 列（E 10/20—128/256，11 项）；8/16 低于下限
el('profileSel').value = 'ANSI45R';
el('profileSel')._fire('change', el('profileSel'));
await flush();
const opts45 = el('mSel').children.map((o) => o.value).filter((v) => v !== 'custom');
check(opts45.length === 11 && opts45[0] === '10/20' && opts45[10] === '128/256',
  'ANSI45R 下拉应过滤为 11 项 10/20…128/256，实际 ' + opts45.length + '：' + opts45.join('|'));
el('m').value = '8';
el('m')._fire('input', el('m'));
await flush();
check(el('warn').textContent.includes('10/20 — 128/256') && el('warn').textContent.includes('8/16')
  && el('warn').textContent.includes('低于下限'), '45° 列 P=8/16 应低于下限：' + el('warn').textContent);
// 回 A 列继续原流程
el('profileSel').value = 'ANSI30P';
el('profileSel')._fire('change', el('profileSel'));
await flush();
el('mSel').value = '5/10';
el('mSel')._fire('change', el('mSel'));
await flush();
check(Math.abs(numOf('m') - 5) < 1e-9, '选 5/10 应把数字框填 P=5，实际 ' + el('m').value);
check(el('ansiPitchHint').textContent.includes('P=5') && el('ansiPitchHint').textContent.includes('Ps=10'),
  'ANSI hint 应显示 P=5（Ps=10）：' + el('ansiPitchHint').textContent);
check(el('ansiPitchHint').textContent.includes('换算模数 m=5.08'),
  'ANSI hint 应显示换算模数 m=5.08：' + el('ansiPitchHint').textContent);
check((el('m').title || '').includes('25.4/P'), 'ANSI 换算公式应进 m 的 title：' + el('m').title);
// 系列外自定义 P → collect 红字列 17 项
el('m').value = '3.75';
el('m')._fire('input', el('m'));
await flush();
check(el('warn').textContent.includes('标准系列') && el('warn').textContent.includes('2.5/5'),
  '系列外应红字列 17 项：' + el('warn').textContent);
// ANSI 表达式：DP 写径节原值 A/B（不再换算成 M）
el('m').value = '5';
el('m')._fire('input', el('m'));
await flush();
// ANSI 统一表达式：info.expr（引擎 DA/DF）而非旧的空表达式。
try { el('m')._fire('blur', el('m')); } catch (e) { errors.push('ANSI 表达式刷新异常: ' + e); }
await flush();
check(el('exprPreview').value.startsWith('SPLINE '),
  'ANSI 花键应给出统一齿形表达式，实际 ' + JSON.stringify(el('exprPreview').value));
check(el('exprCopy').disabled === false, '花键表达式非空时复制按钮应可用');
check((el('exprPreview').title || '').includes('统一齿形表达式'),
  '花键应提示统一齿形表达式（title）：' + el('exprPreview').title);

// ── ANSI 接线回归（用户实测「缺径节 P」的断点）：选完径节后，预览/信息查询串与导出 body
// 必须带 A/B 原值 `pitch=5/10`，而不是被 collect/paramQS/导出 schema 丢在半路。
fetchLog.length = 0;   // 只看这次「选完径节」之后的请求
try { el('m')._fire('blur', el('m')); } catch (e) { errors.push('ANSI 预览触发异常: ' + e); }
await flush();
const ansiSvg = fetchLog.filter((f) => f.url.startsWith('/api/gear_svg')).pop();
check(!!ansiSvg, 'ANSI 选完径节后应发起预览请求（collect 不能拦成缺径节）');
check(!!ansiSvg && !ansiSvg.url.includes('%2F') && ansiSvg.url.includes('pitch=5/10'),
  '预览查询串应带 pitch=5/10（A/B 原值），实际 ' + (ansiSvg ? ansiSvg.url : '(无请求)'));
const ansiInfo = fetchLog.filter((f) => f.url.startsWith('/api/gear_info')).pop();
check(!!ansiInfo && ansiInfo.url.includes('pitch=5/10'),
  '派生值查询串应带 pitch=5/10，实际 ' + (ansiInfo ? ansiInfo.url : '(无请求)'));
check(!el('warn').textContent.includes('缺径节') && !el('perr').textContent.includes('缺径节'),
  '选完径节不应再出现「缺径节 P」：warn=' + el('warn').textContent + ' perr=' + el('perr').textContent);
// 导出 body：与 collect()/paramQS 同一把钥匙（此前 Req 无 pitch 字段，后端收到 None 才报缺径节）
try { el('out')._fire('click', el('out')); } catch (e) { errors.push('ANSI 导出触发异常: ' + e); }
await flush();
const ansiExport = fetchLog.filter((f) => f.url === '/api/gear_export').pop();
check(!!ansiExport, 'ANSI 导出应发起 POST /api/gear_export');
if (ansiExport) {
  let body = null;
  try { body = JSON.parse(ansiExport.body); } catch (e) { /* 下面断言报错 */ }
  check(!!body && body.pitch === '5/10', '导出 body 应带 "pitch":"5/10"，实际 ' + ansiExport.body);
  check(!!body && body.m === undefined, 'ANSI 不应把 P 塞进 m 槽位（避免两侧各写一套）');
}

// ③.9 花键视图按钮（用户定案：内花键同内齿轮，无侧视图）
// 先回 GB（视图规则与体系无关），锁外花键 = 剖视/侧视/端视。
// 同时验证：DIN/NF 里回填的 x 不泄漏到 GB —— 切回 GB 后 x 复位 0 并锁死。
el('stdSel').value = 'GB';
try { el('stdSel')._fire('change', el('stdSel')); } catch (e) { errors.push('GB 回切异常: ' + e); }
await flush();
check(el('x').readOnly === true, '从 DIN/NF 切回 GB 后 x 仍应锁死');
check(el('x').value === '0', '切回 GB 后 x 应复位为 0，实际 ' + el('x').value);
check((el('x').title || '').includes('不可编辑'), '切回 GB 后 x 口径应恢复（title）：' + el('x').title);

// ③.95 轴生成器表达式（花键模式）：INVOLSPLINE 轴段已撤 → 不再给表达式（护栏）
el('m').value = '3';
el('z').value = '20';
el('h').value = '30';
try { el('m')._fire('input', el('m')); } catch (e) { errors.push('花键表达式同步异常: ' + e); }
try { el('m')._fire('blur', el('m')); } catch (e) { errors.push('花键表达式刷新异常: ' + e); }
await flush();
// 花键统一表达式（用户报的「花键模式没有轴生成器表达式」）：非空、可复制、提示统一语法。
check(el('exprPreview').value.startsWith('SPLINE '),
  'GB 花键应给出统一齿形表达式，实际 ' + JSON.stringify(el('exprPreview').value));
check(el('exprCopy').disabled === false, '花键表达式非空时复制按钮应可用');
check((el('exprPreview').title || '').includes('统一齿形表达式'),
  '应提示统一齿形表达式（title）：' + el('exprPreview').title);
globalThis.__copied = '';
el('exprCopy')._fire('click', el('exprCopy'));
await flush();
check(globalThis.__copied.startsWith('SPLINE ') && globalThis.__copied === el('exprPreview').value,
  '花键表达式应可复制且与预览一致，实际 ' + JSON.stringify(globalThis.__copied));
const viewBtns = () => el('viewRow').children.map((b) => b.textContent);
const kindBtns = () => el('kindRow').children;
check(viewBtns().join('|') === '剖视图|侧视图|端视图', '外花键应有三视图按钮，实际 ' + viewBtns().join('|'));
// 选中侧视图 → 切内花键：不可用视图自动回剖视，按钮只剩 剖视 + 端视
const sideBtn = el('viewRow').children.find((b) => b.textContent === '侧视图');
check(!!sideBtn, '外花键应有侧视图按钮');
if (sideBtn) {
  sideBtn.click();
  await flush();
  check(el('viewName').textContent === '侧视图', '点侧视图后当前视图应为侧视图，实际 ' + el('viewName').textContent);
}
const intKindBtn = kindBtns().find((b) => b.dataset.k === 'internal');
check(!!intKindBtn, '应有内花键（内齿轮）种类按钮');
if (intKindBtn) {
  intKindBtn.click();
  await flush();
  check(viewBtns().join('|') === '剖视图（齿圈内齿不剖）|端视图', '内花键应只留 剖视图 + 端视图，实际 ' + viewBtns().join('|'));
  check(el('viewName').textContent === '剖视图（齿圈内齿不剖）', '内花键不可用侧视图时应自动回剖视图，实际 ' + el('viewName').textContent);
  const kindTitles = kindBtns().map((b) => b.title || '').join('|');
  check(kindTitles.includes('无侧视图'), '内花键「无侧视图」口径应进种类按钮 title：' + kindTitles);
  check((el('exprPreview').title || '').includes('最小口径'), '轴上最小绘制口径应进表达式框 title：' + el('exprPreview').title);
  check(el('exprPreview').value.startsWith('SPLINE IN '), '内花键也应给出统一表达式（SPLINE IN）：' + el('exprPreview').value);
}
// 切回外花键：恢复三视图
const extKindBtn = kindBtns().find((b) => b.dataset.k === 'external');
if (extKindBtn) {
  extKindBtn.click();
  await flush();
  check(viewBtns().join('|') === '剖视图|侧视图|端视图', '切回外花键应恢复三视图，实际 ' + viewBtns().join('|'));
}

// ③.8 ② 齿数锁定：DIN/NF 下 z 只读且由 (d_B/A, m) 联动；GB/ANSI 不锁。
// ③  ③ 模数候选按体系取表：GB 表 2 15 种 / DIN 名义表 / NF 尺寸表（含 1.667/3.75/7.5）。
// 先回 GB：z 不锁 + 15 项候选（不含 GB/T 1357 的 0.7）
el('stdSel').value = 'GB';
try { el('stdSel')._fire('change', el('stdSel')); } catch (e) { errors.push('GB 回切异常: ' + e); }
await flush();
check(el('z').readOnly !== true, 'GB 下 z 不应只读');
check(!el('zLabel').textContent.includes('决定'), 'GB 下 z 标签不应带“决定”提示：' + el('zLabel').textContent);
let mods = el('mSel').children.map((o) => o.value).filter((v) => v !== 'custom');
check(mods.length === 15, 'GB 花键模数候选应 15 项，实际 ' + mods.length + '：' + mods.join('|'));
check(mods.indexOf('0.7') < 0, 'GB 候选不应含 GB/T 1357 的 0.7');
check(mods.includes('0.25') && mods.includes('10'), 'GB 候选应含 0.25 与 10：' + mods.join('|'));
// DIN：z 只读；d_B=40 + m=2 → z=18；改 m → z 跟随重算
el('stdSel').value = 'DIN';
try { el('stdSel')._fire('change', el('stdSel')); } catch (e) { errors.push('DIN 锁定切换异常: ' + e); }
await flush();
check(el('z').readOnly === true, 'DIN 下 z 应只读');
check(el('x').readOnly === true, 'DIN 下 x 是因变量（由 d_B、m、z 解出），应只读');
check(el('zLabel').textContent.includes('决定'), 'DIN 下 z 标签应说明由 d_B 与 m 决定：' + el('zLabel').textContent);
mods = el('mSel').children.map((o) => o.value).filter((v) => v !== 'custom');
check(mods.includes('0.6') && mods.includes('0.8'), 'DIN 候选应含名义表实际档 0.6/0.8：' + mods.join('|'));
check(mods.indexOf('1.667') < 0, 'DIN 候选不应含 NF 特有档 1.667');
el('db').value = '40';
el('m').value = '2';
try { el('db')._fire('change', el('db')); } catch (e) { errors.push('DIN d_B 联动异常: ' + e); }
await flush();
check(el('z').value === '18', 'DIN d_B=40/m=2 应联动 z=18，实际 ' + el('z').value);
check(Math.abs(numOf('x') - 0.45) < 1e-9, 'DIN d_B=40/m=2/z=18 应显示表值 x=0.45，实际 ' + el('x').value);
el('m').value = '2.5';
try { el('m')._fire('input', el('m')); } catch (e) { errors.push('DIN m 联动异常: ' + e); }
await flush();
check(el('z').value === '14', 'DIN d_B=40/m=2.5 应联动表内 z=14，实际 ' + el('z').value);
check(el('z').readOnly === true, 'DIN 改 m 后 z 仍应只读');
// DIN 逐 m 断言：16 档 m 的 d_B 候选集合 == CSV 该 m 的 d_B 集合（721 行 / 89 个去重）。
for (const [mk, wantDb] of DIN_BY_M) {
  el('m').value = mk;
  try { el('m')._fire('change', el('m')); } catch (e) { errors.push('DIN m=' + mk + ' 切换异常: ' + e); }
  await flush();
  const gotDb = el('dinDbList').children.map((o) => Number(o.value)).sort((a, b) => a - b);
  check(JSON.stringify(gotDb) === JSON.stringify(wantDb),
    'DIN m=' + mk + ' 的 d_B 候选应 CSV 集合 ' + wantDb.join('|') + '，实际 ' + gotDb.join('|'));
}
// DIN 回落：m=0.5 的 d_B 最大 40，d_B=41 应回落到 40 + 可见提示。
el('m').value = '2';
el('m')._fire('change', el('m'));
await flush();
el('db').value = '41';
el('db')._fire('change', el('db'));
await flush();
check(el('db').value === '41', 'A/d_B 输入完成后不立即回落（回落只在 m 变化时）：' + el('db').value);
el('m').value = '0.5';
el('m')._fire('change', el('m'));
await flush();
check(el('db').value === '40', 'DIN m=0.5 时 d_B=41 应回落到最近合法值 40，实际 ' + el('db').value);
check(el('dbListHint').textContent.includes('回落') && el('dbListHint').textContent.includes('d_B=41')
  && el('dbListHint').textContent.includes('d_B=40'), 'DIN 回落应有可见提示：' + el('dbListHint').textContent);
// NF：候选含 1.667/3.75/7.5 且不含 GB/DIN 特有档；A=80 带出 m=3.75/z=19；
// 体系外模数（3）红字指出来源表。
el('stdSel').value = 'NF';
try { el('stdSel')._fire('change', el('stdSel')); } catch (e) { errors.push('NF 锁定切换异常: ' + e); }
await flush();
check(el('z').readOnly === true, 'NF 下 z 应只读');
check(el('x').readOnly === true, 'NF 下 x 是因变量（由 A、m、N 解出），应只读');
mods = el('mSel').children.map((o) => o.value).filter((v) => v !== 'custom');
check(mods.includes('1.667') && mods.includes('3.75') && mods.includes('7.5'),
  'NF 候选应含 1.667/3.75/7.5：' + mods.join('|'));
for (const ban of ['0.6', '0.8', '1.5', '1.75', '3', '4', '6', '8']) {
  check(mods.indexOf(ban) < 0, 'NF 候选不应含 GB/DIN 特有档 ' + ban);
}
el('mSel').value = '3.75';
try { el('mSel')._fire('change', el('mSel')); } catch (e) { errors.push('NF m 选择异常: ' + e); }
await flush();
el('db').value = '80';
try { el('db')._fire('change', el('db')); } catch (e) { errors.push('NF A 联动异常: ' + e); }
await flush();
check(el('m').value === '3.75' && el('z').value === '19',
  'NF m=3.75/A=80 应带出 z=19，实际 ' + el('m').value + '/' + el('z').value);
check(Math.abs(numOf('x') - 0.967) < 1e-3,
  'NF A=80/m=3.75/N=19 应显示表值 x=0.967（p18 空 x 行不得盖成 0），实际 ' + el('x').value);
// NF 逐 m 断言：10 档 m 的 A 候选集合 == CSV 该 m 的 A 集合（含 1.667/3.75/7.5，不许插值/自编）。
for (const [mk, wantA] of NF_BY_M) {
  el('m').value = mk;
  try { el('m')._fire('change', el('m')); } catch (e) { errors.push('NF m=' + mk + ' 切换异常: ' + e); }
  await flush();
  const gotA = el('nfAList').children.map((o) => Number(o.value)).sort((a, b) => a - b);
  check(JSON.stringify(gotA) === JSON.stringify(wantA),
    'NF m=' + mk + ' 的 A 候选应 CSV 集合 ' + wantA.join('|') + '，实际 ' + gotA.join('|'));
}
// NF 回落：A=81 不在表内 → m 改到 1.667 时回落到该 m 最近的合法值 60 + 可见提示。
el('m').value = '3.75';
el('m')._fire('change', el('m'));
await flush();
el('db').value = '81';
el('db')._fire('change', el('db'));
await flush();
check(el('db').value === '81', 'A 输入完成后不立即回落（回落只在 m 变化时）：' + el('db').value);
el('m').value = '1.667';
el('m')._fire('change', el('m'));
await flush();
check(el('db').value === '60', 'NF m=1.667 时 A=81 应回落到最近合法值 60，实际 ' + el('db').value);
check(el('dbListHint').textContent.includes('回落') && el('dbListHint').textContent.includes('A=81')
  && el('dbListHint').textContent.includes('A=60'), 'NF 回落应有可见提示：' + el('dbListHint').textContent);
check(el('z').value !== '' && Number.isFinite(Number(el('z').value)),
  '回落后的 z 仍应是派生数字：' + el('z').value);
el('db').value = '45';
el('db')._fire('change', el('db'));
await flush();
check(!el('dbListHint').textContent.includes('回落'), '选回合法 A 后不应保留回落提示：' + el('dbListHint').textContent);
el('m').value = '3';
try { el('m')._fire('input', el('m')); } catch (e) { errors.push('NF 体系外模数异常: ' + e); }
await flush();
check(el('warn').textContent.includes('不在') && el('warn').textContent.includes('NF E22-141'),
  '体系外模数应红字指出来源表：' + el('warn').textContent);
check(el('warn').textContent.includes('1.667'), '报错应列出 NF 可用值：' + el('warn').textContent);
// ANSI：z 不锁
el('stdSel').value = 'ANSI';
try { el('stdSel')._fire('change', el('stdSel')); } catch (e) { errors.push('ANSI 锁定切换异常: ' + e); }
await flush();
check(el('z').readOnly !== true, 'ANSI 下 z 不应只读');
check(!el('zLabel').textContent.includes('决定'), 'ANSI 下 z 标签不应带“决定”提示：' + el('zLabel').textContent);
check(el('x').readOnly !== true, 'ANSI 下 x 不应锁死（仅 GB 花键锁 x）');

// GB 的 x 锁定只在花键模式生效：退出花键模式（回齿轮）后 x 解禁可编辑，提示清空。
el('splineMode').checked = false;
try { el('splineMode')._fire('change', el('splineMode')); } catch (e) { errors.push('退出花键模式异常: ' + e); }
await flush();
check(el('x').readOnly !== true, '齿轮模式 x 不应锁死（GB 锁 x 只在花键模式生效）');
check((el('x').title || '') === '', '齿轮模式 x 不应再带花键口径 title：' + el('x').title);

// ④ 防抖回调可跑（collect/renderInfo 不抛）
for (const t of timers.splice(0)) {
  try { t(); } catch (e) { errors.push('防抖回调异常: ' + (e && e.stack ? e.stack : e)); }
}
await flush();

if (errors.length) {
  console.error('FAIL:\n' + errors.join('\n'));
  process.exit(1);
}
console.log('GEAR_GUI_SMOKE_OK');
