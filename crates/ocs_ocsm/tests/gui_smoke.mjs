// guide_gui.html 运行时冒烟测试（node 最小 DOM 垫片）。
//
// 目的：catch「node --check 查不出」的 GUI 运行时错误——历史上已出现两次：
//   ① 缺 GDT_POLYS/drawGdtGlyph 定义 → 初始化 IIFE 崩溃；
//   ② wline 定义在 draw() 内部（在 WELD 早返回之后）→ drawWeld 抛
//      ReferenceError → 焊接预览空白；且 WELD 分支漏写 row-weld 显示。
// 做法：垫出 document/window/fetch/location，跑 GUI 脚本，然后点「焊接」
// 类型按钮，断言：无异常、row-weld 可见、预览 SVG 有内容。
//
// 用法：node gui_smoke.mjs <guide_gui.html 路径>

import fs from 'node:fs';
import { renderZh } from './i18n_zh_fixture.mjs';

const htmlPath = process.argv[2];
if (!htmlPath) {
  console.error('usage: node gui_smoke.mjs <guide_gui.html>');
  process.exit(2);
}
const html = renderZh(fs.readFileSync(htmlPath, 'utf8'));
const m = html.match(/<script>([\s\S]*?)<\/script>/);
if (!m) {
  console.error('no <script> found');
  process.exit(2);
}
const script = m[1];

// ── 最小 DOM 垫片 ────────────────────────────────────────────────
const els = new Map();
const containerButtons = new Map(); // 容器 id → 按钮数组

function mkEl(id, dataset = {}) {
  const el = {
    id,
    dataset,
    style: {},
    value: '',
    checked: false,
    disabled: false,
    textContent: '',
    children: [],
    options: [],
    _handlers: {},
    _innerHTML: '',
    classList: {
      _on: new Set(),
      add(c) { this._on.add(c); },
      remove(c) { this._on.delete(c); },
      contains(c) { return this._on.has(c); },
    },
    addEventListener(ev, fn) { (this._handlers[ev] ||= []).push(fn); },
    appendChild(c) {
      this.children.push(c);
      if (c && c._isOption) this.options.push(c); // <select>.options
      return c;
    },
    removeChild(c) { this.children = this.children.filter((x) => x !== c); },
    querySelectorAll(sel) {
      if (sel === 'button') return containerButtons.get(this.id) || [];
      return [];
    },
    querySelector(sel) { return this.querySelectorAll(sel)[0] || null; },
    getBoundingClientRect() { return { left: 0, top: 0, width: 800, height: 600 }; },
    setAttribute() {},
    getContext() { return null; },
    closest() { return null; },
    focus() {},
    remove() {},
    click() { (this._handlers['click'] || []).forEach((f) => f({ target: this })); },
  };
  Object.defineProperty(el, 'innerHTML', {
    get() { return el._innerHTML; },
    set(v) { el._innerHTML = v; if (v === '') el.children = []; },
  });
  return el;
}

// #seg-type 的类型按钮（脚本会按 data-t 过滤/绑定）
const TYPE_NAMES = [
  'LINEAR', 'DIAMETER', 'RADIUS', 'DATUM', 'VIEW',
  'ANGLE', 'SECTION', 'TOLERANCE', 'ARCLEN', 'DETAIL', 'WELD', 'LEADER',
];
const typeButtons = TYPE_NAMES.map((t) => mkEl('seg-type-' + t, { t }));
typeButtons.forEach((b) => b.classList.add('on'));

global.document = {
  getElementById(id) {
    if (!els.has(id)) els.set(id, mkEl(id));
    return els.get(id);
  },
  querySelectorAll(sel) {
    if (sel === '#seg-type button') return typeButtons;
    return [];
  },
  querySelector(sel) {
    if (sel === '#seg-type button[data-t="WELD"]') return typeButtons.find((b) => b.dataset.t === 'WELD');
    if (sel === '#seg-type button[data-t="LEADER"]') return typeButtons.find((b) => b.dataset.t === 'LEADER');
    return null;
  },
  createElementNS() { return mkEl('ns'); },
  createElement() { return mkEl('el'); },
  createTextNode(t) { const e = mkEl('text'); e.textContent = String(t); return e; },
  addEventListener() {},
};
containerButtons.set('seg-type', typeButtons);
// 浏览器内置 Option 构造器（initWeldRows 用它建下拉项）
global.Option = function Option(label, value) {
  const o = mkEl('option');
  o.textContent = String(label);
  o.value = value === undefined ? String(label) : String(value);
  o._isOption = true;
  return o;
};
global.window = { addEventListener() {}, close() {} };
global.location = { search: '?handle=0x6F', port: '23751', href: 'http://127.0.0.1:23751/guide.html?handle=0x6F' };
global.setTimeout = () => 0;
// node 内置 navigator（只读），无需覆盖。

const GUIDE = {
  ok: true,
  handle: '0x6F',
  p1: [0, 0, 0],
  p2: [60, 20, 0],
  pts: [[0, 0, 0], [20, 20, 0], [60, 20, 0]],
  geom: 'pline',
  url: null,
  measurement: 67.1,
  params: {
    type: 'WELD', dist: 0,
    weld: {
      upper: '角焊', lower: '点焊', dash: true, circle: false, half: true,
      flag: true, tail: true, grindUpper: 'zig', grindLower: 'cvx',
      methodUpper: 'C', methodLower: 'U',
      up_thick: '5', up_qty: '100', lo_thick: '3', lo_qty: '50', tail_text: 'N=2',
    },
  },
};
const SYMS = [
  { name: '角焊', cross: false, upper: [['L', 0, 0.125, 3.5, 0.125]], lower: [['L', 0, -0.125, 3.5, -0.125]] },
  { name: '点焊', cross: false, upper: [['C', 2.275, 2.275, 2.275]], lower: [['C', 2.275, -2.275, 2.275]] },
];
global.fetch = async (u) => {
  const t = String(u);
  if (t.startsWith('/api/guide')) return { json: async () => GUIDE };
  if (t.startsWith('/api/weld_syms')) return { json: async () => ({ ok: true, syms: SYMS }) };
  return { json: async () => ({ ok: false }) };
};
global.console = console;

// ── 跑 GUI 脚本 ─────────────────────────────────────────────────
const errors = [];
try {
  // IIFE：直接 eval（全局垫片已就位）
  // eslint-disable-next-line no-eval
  // 末尾追加探针：把 IIFE 内的 setEditMode 暴露给断言用（不改生产代码）
  const probed = script.replace(/\}\)\(\);\s*$/, ';globalThis.__setEditMode = setEditMode;\n})();');
  (0, eval)(probed);
} catch (e) {
  errors.push('脚本求值异常: ' + (e && e.stack ? e.stack : e));
}

// load() 是 async：等一轮微任务/宏任务
await new Promise((r) => setImmediate(r));
await new Promise((r) => setImmediate(r));

// 点「焊接」类型按钮（触发 seg-type 分支 → 显示 row-weld + drawWeld）
const weldBtn = typeButtons.find((b) => b.dataset.t === 'WELD');
try {
  weldBtn.click();
} catch (e) {
  errors.push('点击焊接按钮异常: ' + (e && e.stack ? e.stack : e));
}
await new Promise((r) => setImmediate(r));

// 遍历所有类型页：任何一页崩溃都算失败（覆盖非焊接分支）
for (const b of typeButtons) {
  try {
    b.click();
    await new Promise((r) => setImmediate(r));
  } catch (e) {
    errors.push(`切换类型 ${b.dataset.t} 异常: ` + (e && e.stack ? e.stack : e));
  }
}
// 回到焊接页做面板断言
try { weldBtn.click(); } catch (e) { errors.push('回切焊接异常: ' + e); }
await new Promise((r) => setImmediate(r));

// ── 断言 ───────────────────────────────────────────────────────
const rowWeld = document.getElementById('row-weld');
if (rowWeld.style.display !== '') {
  errors.push(`row-weld 未显示（display=${JSON.stringify(rowWeld.style.display)}）`);
}
const svg = document.getElementById('prev');
if (!svg.children || svg.children.length === 0) {
  errors.push('预览 SVG 为空（drawWeld 未画出任何元素）');
}
// 焊接方法：按侧独立判定——本例 upper=角焊（可用）、lower=点焊（不可用）
if (document.getElementById('w-method-u').disabled) {
  errors.push('w-method-u 不应禁用（upper=角焊 属可用范围）');
}
if (!document.getElementById('w-method-l').disabled) {
  errors.push('w-method-l 应禁用（lower=点焊 不属可用范围）');
}
// 打磨：上下侧独立两个下拉
if (!document.getElementById('w-grind-u') || !document.getElementById('w-grind-l')) {
  errors.push('缺少上/下侧打磨下拉');
}

// ── 引线标注（LEADER）页 ────────────────────────────────────────
// ① 两段（3 顶点）PLINE 场景：「焊接」与「引线」按钮都应可见。
const leaderBtn = typeButtons.find((b) => b.dataset.t === 'LEADER');
if (!leaderBtn) {
  errors.push('缺少「引线」类型按钮');
} else {
  if (leaderBtn.style.display === 'none') errors.push('3 顶点 PLINE 下「引线」按钮应可见');
  if (weldBtn.style.display === 'none') errors.push('3 顶点 PLINE 下「焊接」按钮应可见');
  // ② 切到引线页：面板显示 + 预览非空 + 无异常。
  try {
    leaderBtn.click();
  } catch (e) {
    errors.push('点击引线按钮异常: ' + (e && e.stack ? e.stack : e));
  }
  await new Promise((r) => setImmediate(r));
  const rowLeader = document.getElementById('row-leader');
  if (rowLeader.style.display !== '') {
    errors.push(`row-leader 未显示（display=${JSON.stringify(rowLeader.style.display)}）`);
  }
  if (!svg.children || svg.children.length === 0) {
    errors.push('引线预览 SVG 为空（drawLeader 未画出任何元素）');
  }
  // ③ 填文字后重绘：不抛异常且预览仍在（肩线随文字变长）。
  document.getElementById('l-up').value = '通孔';
  document.getElementById('l-lo').value = '深10';
  const before = svg.children.length;
  try {
    leaderBtn.click();
  } catch (e) {
    errors.push('填文字后重绘异常: ' + (e && e.stack ? e.stack : e));
  }
  await new Promise((r) => setImmediate(r));
  if (!svg.children || svg.children.length === 0) {
    errors.push('填文字后引线预览为空');
  }
  if (before > 0 && svg.children.length === before) {
    // 元素数相同是正常的（线数不变），只要不是 0 即视为重绘成功。
  }
  // ④ 退回焊接页，确认互不干扰。
  try {
    weldBtn.click();
  } catch (e) {
    errors.push('引线→焊接回切异常: ' + (e && e.stack ? e.stack : e));
  }
  await new Promise((r) => setImmediate(r));
  if (rowWeld.style.display !== '') errors.push('回切后 row-weld 未显示');
}
// ⑤ 工艺代号下拉（GB/T 5185）：选中「代号 名称」→ 尾部注释只写代号，
// 且尾部开关自动锁定；再选一个=组合工艺（空格并列）。
{
  const c5185 = document.getElementById('w-c5185');
  if (!c5185) {
    errors.push('缺 w-c5185 工艺代号下拉');
  } else {
    document.getElementById('w-tt').value = '';
    c5185.value = '111';
    (c5185._handlers['change'] || []).forEach((f) => f({ target: c5185 }));
    const tt1 = document.getElementById('w-tt').value;
    if (tt1 !== '111') errors.push(`选「111 焊条电弧焊」尾部应为「111」，实为 ${JSON.stringify(tt1)}`);
    if (document.getElementById('w-tail').checked !== true) {
      errors.push('选工艺代号后尾部开关应自动打开');
    }
    c5185.value = '12';
    (c5185._handlers['change'] || []).forEach((f) => f({ target: c5185 }));
    const tt2 = document.getElementById('w-tt').value;
    if (tt2 !== '111 12') errors.push(`组合工艺应为「111 12」，实为 ${JSON.stringify(tt2)}`);
    if (c5185.value !== '') errors.push('选完后下拉应复位为空');
    // 恢复现场（GUIDE 参数里有 tail_text='N=2'，不影响后续断言）
    document.getElementById('w-tt').value = 'N=2';
    (document.getElementById('w-tt')._handlers['input'] || []).forEach((f) => f({ target: document.getElementById('w-tt') }));
  }
}

// ⑥ 只允许两段 PLINE：「焊接」「引线」必须同在 nV===3 的过滤分支里
//（防止以后误改成所有多段线都显示）。
if (!/nV === 3 *\? *\[.*'WELD'.*'LEADER'/.test(html) &&
    !/nV === 3 *\? *\[.*'LEADER'/.test(html)) {
  errors.push('几何过滤未把 LEADER 限定在 3 顶点 PLINE');
}

// ⑦ A1：编辑模式文案（标题/按钮 → 「更新标注」）
if (typeof globalThis.__setEditMode !== 'function') {
  errors.push('未暴露 setEditMode（探针失败）');
} else {
  globalThis.__setEditMode(true);
  const tt = document.getElementById('title-text').textContent;
  if (tt !== '更新标注') errors.push(`编辑模式标题应为「更新标注」，实为 ${JSON.stringify(tt)}`);
  if (document.title !== '更新标注') {
    errors.push(`编辑模式 document.title 应为「更新标注」，实为 ${JSON.stringify(document.title)}`);
  }
  const btnA = document.getElementById('btn-apply').textContent;
  const btnR = document.getElementById('btn-refresh').textContent;
  if (btnR !== '更新标注') errors.push(`编辑模式主按钮应为「更新标注」，实为 ${JSON.stringify(btnR)}`);
  if (btnA !== '另存为新标注') errors.push(`编辑模式副按钮应为「另存为新标注」，实为 ${JSON.stringify(btnA)}`);
  if (document.getElementById('edit-note').style.display !== '') {
    errors.push('编辑模式提示行 edit-note 应可见');
  }
  globalThis.__setEditMode(false);
  if (document.getElementById('title-text').textContent !== '标注配置') {
    errors.push('退出编辑模式后标题应还原为「标注配置」');
  }
  if (document.getElementById('btn-refresh').textContent !== '应用并刷新') {
    errors.push('退出编辑模式后主按钮应还原为「应用并刷新」');
  }
  if (document.getElementById('edit-note').style.display !== 'none') {
    errors.push('退出编辑模式后提示行应隐藏');
  }
}

if (errors.length) {
  console.error('GUI 冒烟失败：\n- ' + errors.join('\n- '));
  process.exit(1);
}
console.log('GUI 冒烟通过：焊接面板可见、预览非空、引线面板可见、无运行时异常');
