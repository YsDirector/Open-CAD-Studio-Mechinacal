// parts_gui.html 运行时/行为冒烟（node 最小 DOM 垫片 + 最小 fetch 桩）。
//
// 目的：锁住平键「型别下拉（A/B/C）」的交互契约（node --check / el("id") 静态扫描查不出）：
//   ① 目录族带 `type_group` → 显示型别下拉，选项 = 该组族（A/B/C 或 A/B），当前值 = 族 id；
//   ② change 型别 → 切到对应族（cur.family/树高亮/规格下拉全部联动），不新增窗口/不重载；
//   ③ 无 `type_group` 的族（例：C 级六角螺栓）→ 型别下拉隐藏；
//   ④ 结构要素/普通标准件的既有行为不被型别逻辑带坏（螺栓族仍显示直径/长度）。
//
// 用法：node parts_gui_smoke.mjs <parts_gui.html 路径>

import fs from 'node:fs';
import { renderZh } from './i18n_zh_fixture.mjs';

const htmlPath = process.argv[2];
if (!htmlPath) {
  console.error('usage: node parts_gui_smoke.mjs <parts_gui.html>');
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

// ── 最小 DOM 垫片 ────────────────────────────────────────────────
function mkEl(tag) {
  const el = {
    tagName: String(tag).toUpperCase(),
    id: '',
    type: '',
    dataset: {},
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
      toggle(c, force) {
        const on = force === undefined ? !this._on.has(c) : !!force;
        if (on) this._on.add(c); else this._on.delete(c);
        return on;
      },
    },
    addEventListener(ev, fn) { (this._handlers[ev] ||= []).push(fn); },
    _fire(ev, target, extra) {
      const e = Object.assign({ target, type: ev }, extra || {});
      (this._handlers[ev] || []).forEach((f) => f(e));
    },
    appendChild(c) {
      this.children.push(c);
      if (this.tagName === 'SELECT') this.options.push(c); // 真实浏览器 select.options 是活集合
      return c;
    },
    removeChild(c) { this.children = this.children.filter((x) => x !== c); },
    querySelectorAll(sel) {
      if (sel === 'button') return this.children.filter((c) => c.tagName === 'BUTTON');
      return [];
    },
    querySelector(sel) {
      if (sel === '.tw') return this.children.find((c) => c.classList.contains('tw')) || null;
      return null;
    },
    remove() {},
    click() { this._fire('click', this); },
  };
  Object.defineProperty(el, 'innerHTML', {
    get() { return el._innerHTML; },
    set(v) {
      el._innerHTML = String(v);
      if (el._innerHTML === '') {
        el.children = [];
        el.options = [];
      }
    },
  });
  return el;
}

const els = new Map();
global.document = {
  getElementById(id) {
    if (!els.has(id)) {
      const e = mkEl('div');
      e.id = id;
      els.set(id, e);
    }
    return els.get(id);
  },
  createElement(tag) { return mkEl(tag); },
  createTextNode(t) { const e = mkEl('text'); e.textContent = String(t); return e; },
  querySelectorAll() { return []; },
  querySelector() { return null; },
  addEventListener() {},
};
global.window = { addEventListener() {}, close() {} };
global.location = { search: '', href: 'http://127.0.0.1:9/parts' };
// 页面里的 <select>/<input> 预注册真实 tag（shim 的 getElementById 默认建 div）。
for (const [id, tag] of [['typeSel', 'select'], ['dia', 'select'], ['len', 'select'], ['specSel', 'select'], ['dnum', 'input'], ['b1num', 'input'], ['specText', 'input']]) {
  const e = mkEl(tag);
  e.id = id;
  els.set(id, e);
}
global.setTimeout = (fn) => { fn(); return 0; };
global.setInterval = () => 0;
global.clearInterval = () => {};

// ── 目录桩（结构紧凑：键两族 + 螺栓一笔，覆盖有/无 type_group 两条路）──
const KEY_1096_SIZES = [
  { d: 2, label: 'b=2（h=2）', pitch: 0, l_min: 6, l_max: 18, lengths: [6, 8, 10, 12, 14, 16, 18] },
  { d: 4, label: 'b=4（h=4）', pitch: 0, l_min: 6, l_max: 32, lengths: [8, 6, 10, 12] },
];
const KEY_1097_SIZES = [
  { d: 8, label: 'b=8（h=7）', pitch: 0, l_min: 25, l_max: 70, lengths: [70, 25, 28, 63] },
  { d: 45, label: 'b=45（h=25）', pitch: 0, l_min: 25, l_max: 400, lengths: [100, 400] },
];
const CATALOG = {
  tree: [
    {
      name: '零件库',
      children: [
        {
          name: '键',
          children: [
            { name: '平键', children: [
              { name: '圆头普通平键 A型 GB/T 1096-2003', family: 'key_1096_a', implemented: true },
              { name: '圆头普通平键 B型 GB/T 1096-2003', family: 'key_1096_b', implemented: true },
              { name: '导向平键 A型 GB/T 1097-2003', family: 'key_1097_a', implemented: true },
            ] },
          ],
        },
        {
          name: '螺栓',
          children: [
            { name: '六角头螺栓 C级 GB/T 5780-2016', family: 'hex_bolt_c', implemented: true },
          ],
        },
      ],
    },
  ],
  families: {
    key_1096_a: {
      id: 'key_1096_a', name: '圆头普通平键 A型', code: 'GB/T 1096-2003', iso: '—',
      implemented: true, views: [
        { id: 'main', name: '主视图' }, { id: 'top', name: '俯视图' }, { id: 'section', name: '剖视图' },
      ],
      sizes: KEY_1096_SIZES, len_label: '长度 L', base_hint: '基点 = 左端面对称轴；d 槽位承载 b',
      type_group: [
        { id: 'key_1096_a', name: 'A型（双圆头）' },
        { id: 'key_1096_b', name: 'B型（双平头）' },
        { id: 'key_1096_c', name: 'C型（单圆头）' },
      ],
      kind: 'key',
    },
    key_1096_b: {
      id: 'key_1096_b', name: '圆头普通平键 B型', code: 'GB/T 1096-2003', iso: '—',
      implemented: true, views: [
        { id: 'main', name: '主视图' }, { id: 'top', name: '俯视图' }, { id: 'section', name: '剖视图' },
      ],
      sizes: KEY_1096_SIZES, len_label: '长度 L', base_hint: '基点 = 左端面对称轴；d 槽位承载 b',
      type_group: [
        { id: 'key_1096_a', name: 'A型（双圆头）' },
        { id: 'key_1096_b', name: 'B型（双平头）' },
        { id: 'key_1096_c', name: 'C型（单圆头）' },
      ],
      kind: 'key',
    },
    key_1097_a: {
      id: 'key_1097_a', name: '导向平键 A型', code: 'GB/T 1097-2003', iso: '—',
      implemented: true, views: [{ id: 'main', name: '主视图' }, { id: 'top', name: '俯视图' }],
      sizes: KEY_1097_SIZES, len_label: '长度 L',
      base_hint: '基点 = 左端面对称轴（俯视图）/ 左端面×底面（主视图）；d 槽位承载 b',
      type_group: [
        { id: 'key_1097_a', name: 'A型（双圆头）' },
        { id: 'key_1097_b', name: 'B型（双平头）' },
      ],
      kind: 'key',
    },
    hex_bolt_c: {
      id: 'hex_bolt_c', name: '六角头螺栓 C级', code: 'GB/T 5780-2016', iso: '—',
      implemented: true, views: [{ id: 'main', name: '主视图' }],
      sizes: [{ d: 10, label: 'M10', pitch: 0, l_min: 40, l_max: 100, lengths: [40, 50, 60] }],
      len_label: '长度 l', base_hint: '基点 = 头部支承面 × 轴线', kind: 'bolt',
    },
  },
};

function jsonResp(obj, status = 200) {
  return { ok: status < 400, status, json: async () => obj, text: async () => JSON.stringify(obj) };
}
global.fetch = async (u) => {
  const url = String(u);
  if (url.startsWith('/api/parts')) return jsonResp(CATALOG);
  return jsonResp({ ok: true });
};

// ── 跑 GUI 脚本（末尾探针暴露 IIFE 内部状态；不改生产代码）──────────
const probed = script.replace(/\}\)\(\);\s*$/, `;globalThis.__parts = {
  get catalog() { return catalog; },
  get cur() { return cur; },
  get treeRows() { return treeRows; },
  pickFamily,
};
})();`);
try {
  // 页面外链的共享助手（/ocsm_gui_common.js）：Node 里按同一目录文件先求值
  (0, eval)(renderZh(fs.readFileSync(htmlPath.replace(/[^/]+$/, 'ocsm_gui_common.js'), 'utf8')));
  (0, eval)(probed);
} catch (e) {
  errors.push('脚本求值异常: ' + (e && e.stack ? e.stack : e));
}
await new Promise((r) => setImmediate(r));
await new Promise((r) => setImmediate(r));
await new Promise((r) => setImmediate(r));

const P = globalThis.__parts;
const typeFields = document.getElementById('typeFields');
const typeSel = document.getElementById('typeSel');
const dia = document.getElementById('dia');
const viewRow = document.getElementById('viewRow');
check(!!P, '探针 __parts 未挂上（脚本初始化崩溃？）');
if (!P) report();

// ① 启动自动选中第一族（键 A 型）→ 型别下拉可见、3 项、当前值 = 族 id；规格下拉已填充
check(P.cur.family === 'key_1096_a', `启动应自动选中 key_1096_a，实为 ${P.cur.family}`);
check(typeFields.style.display === '', '平键族应显示型别下拉');
check(typeSel.options.length === 3, `A/B/C 型别应有 3 项，实为 ${typeSel.options.length}`);
check(typeSel.options.map((o) => o.value).join(',') === 'key_1096_a,key_1096_b,key_1096_c', '型别选项顺序 A/B/C');
check(typeSel.value === 'key_1096_a', `型别当前值应 = 族 id，实为 ${typeSel.value}`);
check(dia.options.length === KEY_1096_SIZES.length, `直径下拉应有 ${KEY_1096_SIZES.length} 项`);
check(viewRow.children.length === 3, `1096 应 3 个视图按钮，实为 ${viewRow.children.length}`);

// ② change 型别 A→B：切族、树高亮、视图/规格联动，型别下拉不重建消失
typeSel.value = 'key_1096_b';
typeSel._fire('change', typeSel);
check(P.cur.family === 'key_1096_b', `切型别后应到 key_1096_b，实为 ${P.cur.family}`);
check(typeSel.value === 'key_1096_b', '型别下拉应保持新值');
check(P.treeRows['key_1096_b'].classList.contains('sel'), '左侧树应高亮 B 型节点');
check(!P.treeRows['key_1096_a'].classList.contains('sel'), 'A 型节点高亮应移除');
check(viewRow.children.length === 3, '切换后视图按钮应仍是 3 个（重渲染）');

// ③ 经树选中 1097 A 型 → 型别下拉只剩 A/B 两项，当前值 = 1097 族 id；规格 = 14 档桩的 2 项
P.pickFamily('key_1097_a');
check(P.cur.family === 'key_1097_a', '应切到 key_1097_a');
check(typeFields.style.display === '', '1097 平键族应显示型别下拉');
check(typeSel.options.length === 2, `1097 型别应有 2 项，实为 ${typeSel.options.length}`);
check(typeSel.options.map((o) => o.value).join(',') === 'key_1097_a,key_1097_b', '1097 型别选项 A/B');
check(typeSel.value === 'key_1097_a', '型别当前值应为 key_1097_a');
check(viewRow.children.length === 2, '1097 应 2 个视图按钮');
check(dia.options.length === KEY_1097_SIZES.length, '1097 直径下拉项数');

// ④ 无 type_group 的族（螺栓）→ 型别下拉隐藏，且长度下拉仍按族数据填充
P.pickFamily('hex_bolt_c');
check(P.cur.family === 'hex_bolt_c', '应切到 hex_bolt_c');
check(typeFields.style.display === 'none', '螺栓族应隐藏型别下拉');
check(typeSel.options.length === 2 && typeSel.value === 'key_1097_a', '隐藏时不应清掉上次选项（只是 display:none）');
check(viewRow.children.length === 1 && dia.options.length === 1, '螺栓族视图/规格联动仍正常');

report();

function report() {
  if (errors.length) {
    console.error('标准件库 GUI 冒烟失败：\n- ' + errors.join('\n- '));
    process.exit(1);
  }
  console.log('标准件库 GUI 冒烟通过：型别下拉 A/B/C 渲染 / change 切族联动树高亮 / 非平键族隐藏 / 规格-视图联动');
  process.exit(0);
}
