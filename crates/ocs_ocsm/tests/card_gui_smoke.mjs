// 智能卡片冒烟（node 最小 DOM 垫片 + fetch 桩）：锁住「表驱动 GUI 统一 + 七张卡 + NF 公差」契约。
//
//   ① 统一外观：页面不再手写齿轮/ANSI/NF/DIN 面板；全部由 `card_types[].form` 驱动，
//      与 GB 面板同一套分区（顶部参数 / 中部主输入+读数 / 结果卡 / 底部按钮）；
//      顶部常显不得再出现「本期六张卡…」说明（用户截图红框）；
//   ② 字段顺序/控件类型：顶部参数区控件顺序 = form.fields 顺序（只跳过 textarea 主输入）；
//      首项卡名 = GB 花键参数表；⑤ 每张卡名带标准号、下拉 title 含全名；
//   ③ 齿轮卡：表达式 + 配对齿数/图号/精度等级/中心距 → /api/card_preview，19 项、缺项「—」；
//   ④ ANSI 卡：方向/齿廓（选项表下发）+ P/z；纯中/纯英两个卡类型；
//   ⑤ NF 内卡：A/m/z/定心/齿根/配合（6 字段）→ 18 项；公差 = R7/H7/p29 E；
//      ⑤b NF 外卡：A/m/z/定心/齿根/配合 → 18 项；模板锚点 Dee/Die/K/W + h12/H7/p29 外花键 E；
//   ⑥ DIN 卡：12 字段（覆盖项独立成格）→ 26 项；缺口 m=5 → 公差「—」；
//   ⑦ 出表：非花键卡走 /api/card_export；无 at → 待放置件；有 at → 直插；
//   ⑧ 预览 404 → 红框可见（共享助手；不关窗）；信息分层负断言；④ 版面类名/顺序断言。
//
// 用法：node card_gui_smoke.mjs <spline_gui.html 路径>

import fs from 'node:fs';
import { renderZh } from './i18n_zh_fixture.mjs';

const htmlPath = process.argv[2];
if (!htmlPath) {
  console.error('usage: node card_gui_smoke.mjs <spline_gui.html>');
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

// ── 选项表桩：六张卡 + 体系 + 齿轮/ANSI/NF/DIN 选项 + 表驱动 form 字段清单 ──
const FIELD = (key, label, kind, extra = {}) => Object.assign({
  key, label, kind, placeholder: '', default: '', title: 'stub 口径',
  options: [], options_from: '', min: 0, step: 0, required: false,
}, extra);
const GEAR_FORM = {
  fields: [
    FIELD('expr', '齿轮齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）', 'textarea',
      { default: 'GEAR EX M3 Z20 ALPHA20 X0 DA66 DF52.5 BETA0 H30', required: true }),
    FIELD('mate_z', '配对齿轮齿数 z₂', 'number', { min: 2, step: 1 }),
    FIELD('mate_dwg', '配对齿轮图号', 'text'),
    FIELD('grade', '精度等级', 'text'),
    FIELD('center', '中心距 a', 'number', { step: 0.001 }),
  ],
  note: 'stub 齿轮提示',
  missing_note: 'stub 缺项说明（GB/T 10095 未收）',
};
const ANSI_FORM = {
  fields: [
    FIELD('expr', '齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）', 'textarea'),
    FIELD('profile', '齿廓', 'select', { options_from: 'ansi_profiles' }),
    FIELD('p', '径节 P', 'number', { default: '16', min: 2.5, step: 0.5, required: true }),
    FIELD('z', '齿数 N', 'number', { default: '20', min: 3, step: 1, required: true }),
  ],
  note: 'stub ANSI 说明',
  missing_note: 'stub ANSI 缺项（Table 4/5 未收）',
};
const NF_FORM = {
  fields: [
    FIELD('expr', '齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）', 'textarea'),
    FIELD('a', '公称直径 A', 'number', { default: '300', step: 0.001, required: true }),
    FIELD('m', '模数 m', 'number', { default: '7.5', step: 0.001, required: true }),
    FIELD('z', '齿数 z', 'number', { default: '38', min: 3, step: 1 }),
    FIELD('centering', '定心方式', 'select', { options_from: 'nf_centering' }),
    FIELD('root', '齿根样式', 'select', { options_from: 'nf_roots' }),
    FIELD('fit', '配合类别', 'select', {
      options: [
        { value: 'loose', label: '松动' }, { value: 'slide', label: '滑动' },
        { value: 'fixed', label: '固定' }, { value: 'press', label: '压' },
      ],
      default: 'fixed',
    }),
  ],
  note: 'stub NF 说明（公差 p28 R7/H7 + p29 E）',
  missing_note: 'stub NF 缺项（(m,A) 不在 p29 或 ISO 档缺）',
};
const NF_EXT_FORM = {
  fields: [
    FIELD('expr', '齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）', 'textarea'),
    FIELD('a', '公称直径 A', 'number', { default: '300', step: 0.001, required: true }),
    FIELD('m', '模数 m', 'number', { default: '7.5', step: 0.001, required: true }),
    FIELD('z', '齿数 z', 'number', { default: '38', min: 3, step: 1 }),
    FIELD('centering', '定心方式', 'select', { options_from: 'nf_ext_centering', default: 'flank' }),
    FIELD('root', '齿根样式', 'select', { options_from: 'nf_ext_roots', default: 'flat' }),
    FIELD('fit', '配合类别', 'select', {
      options: [
        { value: 'loose', label: '松动' }, { value: 'slide', label: '滑动' },
        { value: 'fixed', label: '固定' }, { value: 'press', label: '压' },
      ],
      default: 'fixed',
    }),
  ],
  note: 'stub NF 外说明（公差 h12/H7 + p29 外花键 E）',
  missing_note: 'stub NF 外缺项（h12/H7 + p29 表外/ISO 档缺）',
};
const DIN_FORM_INT = {
  fields: [
    FIELD('expr', '齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）', 'textarea'),
    FIELD('m', '模数 m', 'number', { default: '3', step: 0.001, required: true }),
    FIELD('z', '齿数 z', 'number', { default: '38', min: 3, step: 1, required: true }),
    FIELD('d_b', '基准直径 d_B', 'number', { default: '120', step: 0.001, required: true }),
    FIELD('hub', '内花键配合', 'text', { default: '9H' }),
    FIELD('e2', 'e₂=s₁ 覆盖', 'number', { step: 0.001 }),
    FIELD('ae', 'Ae 覆盖', 'number', { step: 0.001 }),
    FIELD('as_', 'As 覆盖', 'number', { step: 0.001 }),
    FIELD('tact_n', 'Tact(N) 覆盖', 'number', { step: 0.0001 }),
    FIELD('teff_n', 'Teff(N) 覆盖', 'number', { step: 0.0001 }),
  ],
  note: 'stub DIN 内说明',
  missing_note: 'stub DIN 缺项（Table 7 缺口）',
};
const DIN_FORM_EXT = {
  fields: [
    FIELD('expr', '齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）', 'textarea'),
    FIELD('m', '模数 m', 'number', { default: '3', step: 0.001, required: true }),
    FIELD('z', '齿数 z', 'number', { default: '38', min: 3, step: 1, required: true }),
    FIELD('d_b', '基准直径 d_B', 'number', { default: '120', step: 0.001, required: true }),
    FIELD('shaft', '外花键配合', 'text', { default: '8f' }),
    FIELD('e2', 'e₂=s₁ 覆盖', 'number', { step: 0.001 }),
    FIELD('ae', 'Ae 覆盖', 'number', { step: 0.001 }),
    FIELD('as_', 'As 覆盖', 'number', { step: 0.001 }),
    FIELD('tact_w', 'Tact(W) 覆盖', 'number', { step: 0.0001 }),
    FIELD('teff_w', 'Teff(W) 覆盖', 'number', { step: 0.0001 }),
  ],
  note: 'stub DIN 外说明',
  missing_note: 'stub DIN 缺项（Table 7 缺口）',
};
const LITE_FORM_INT = {
  fields: [
    FIELD('expr', '齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）', 'textarea',
      { default: 'SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30', required: true }),
    FIELD('grade', '公差等级（只影响 Md 计算；卡面不显示公差）', 'number',
      { default: '6', min: 4, step: 1, required: true }),
    FIELD('fit', '配合类别（只影响 Md 计算；卡面不显示公差）', 'select',
      { options: [{ value: 'H', label: 'H' }], default: 'H' }),
    FIELD('root', '齿根样式', 'select', {
      options: [{ value: 'auto', label: '自动（按表达式反解）' }, { value: 'flat', label: '平齿根' }, { value: 'fillet', label: '圆齿根' }],
      default: 'auto',
    }),
    FIELD('dp', '量棒直径 Dp（留空 = 标准 R40 自动选）', 'number'),
  ],
  note: 'stub 精简内说明（不含公差列）',
  missing_note: 'stub 精简内缺项（卡面固定不含公差）',
};
const LITE_FORM_EXT = {
  fields: [
    FIELD('expr', '齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）', 'textarea',
      { default: 'SPLINE EX M3 Z20 ALPHA30 X0 DA66 DF52.5 BETA0 H30', required: true }),
    FIELD('grade', '公差等级（只影响 Wn 计算；卡面不显示公差）', 'number',
      { default: '6', min: 4, step: 1, required: true }),
    FIELD('fit', '配合类别（只影响 Wn 计算；卡面不显示公差）', 'select', {
      options: [{ value: 'h', label: 'h' }, { value: 'k', label: 'k' }], default: 'h',
    }),
    FIELD('root', '齿根样式', 'select', {
      options: [{ value: 'auto', label: '自动（按表达式反解）' }, { value: 'flat', label: '平齿根' }, { value: 'fillet', label: '圆齿根' }],
      default: 'auto',
    }),
  ],
  note: 'stub 精简外说明（不含公差列）',
  missing_note: 'stub 精简外缺项（卡面固定不含公差）',
};
const CARD_TYPES = [
  { id: '花键参数表', aliases: ['spline', 'gb'], label: 'GB 花键参数表（内）', summary: 'stub GB', systems: [{ id: 'gb3478', label: 'GB', standard: 'stub' }], direction: 'int', renderer: 'spline_table', form: null },
  { id: '花键参数表_外', aliases: ['splineext'], label: 'GB 花键参数表（外）', summary: 'stub GB ext', systems: [{ id: 'gb3478', label: 'GB', standard: 'stub' }], direction: 'ext', renderer: 'spline_table', form: null },
  { id: '齿轮参数表', aliases: ['gear'], label: '齿轮参数表（GB/T 10095）', summary: 'stub gear', systems: [], direction: null, renderer: 'gear_table', form: GEAR_FORM },
  { id: 'ANSI花键参数表_中文', aliases: ['ansicn'], label: 'ANSI 花键参数表（内·纯中文）', summary: 'stub ansi cn', systems: [], direction: 'int', renderer: 'ansi_table_cn', form: ANSI_FORM },
  { id: 'ANSI花键参数表_外_中文', aliases: ['ansicnext'], label: 'ANSI 花键参数表（外·纯中文）', summary: 'stub ansi cn ext', systems: [], direction: 'ext', renderer: 'ansi_table_cn', form: ANSI_FORM },
  { id: 'ANSI花键参数表_英文', aliases: ['ansien'], label: 'ANSI 花键参数表（内·纯英文）', summary: 'stub ansi en', systems: [], direction: 'int', renderer: 'ansi_table_en', form: ANSI_FORM },
  { id: 'ANSI花键参数表_外_英文', aliases: ['ansienext'], label: 'ANSI 花键参数表（外·纯英文）', summary: 'stub ansi en ext', systems: [], direction: 'ext', renderer: 'ansi_table_en', form: ANSI_FORM },
  { id: 'NF内花键参数表', aliases: ['nf'], label: 'NF E22-141 内花键参数表', summary: 'stub nf', systems: [], direction: 'int', renderer: 'nf_table', form: NF_FORM },
  { id: 'NF外花键参数表', aliases: ['nfext'], label: 'NF E22-141 外花键参数表', summary: 'stub nf ext', systems: [], direction: 'ext', renderer: 'nf_ext_table', form: NF_EXT_FORM },
  { id: 'DIN花键参数表', aliases: ['din'], label: 'DIN 5480 内花键参数表', summary: 'stub din', systems: [], direction: 'int', renderer: 'din_table', form: DIN_FORM_INT },
  { id: 'DIN花键参数表_外', aliases: ['dinext'], label: 'DIN 5480 外花键参数表', group: null, summary: 'stub din ext', systems: [], direction: 'ext', renderer: 'din_table', form: DIN_FORM_EXT },
  { id: 'GB花键精简表_内', aliases: ['gb精简'], label: 'GB 花键参数表（内·精简版）', group: '精简版', summary: 'stub lite int', systems: [{ id: 'gb3478', label: 'GB', standard: 'stub' }], direction: 'int', renderer: 'spline_lite', form: LITE_FORM_INT },
  { id: 'GB花键精简表_外', aliases: ['gb精简外'], label: 'GB 花键参数表（外·精简版）', group: '精简版', summary: 'stub lite ext', systems: [{ id: 'gb3478', label: 'GB', standard: 'stub' }], direction: 'ext', renderer: 'spline_lite', form: LITE_FORM_EXT },
  { id: 'NF花键精简表_内', aliases: ['nf精简'], label: 'NF E22-141 内花键参数表（精简版）', group: '精简版', summary: 'stub NF lite int（不含公差列）', systems: [], direction: 'int', renderer: 'card_lite', form: NF_FORM },
  { id: 'NF花键精简表_外', aliases: ['nf精简外'], label: 'NF E22-141 外花键参数表（精简版）', group: '精简版', summary: 'stub NF lite ext（不含公差列）', systems: [], direction: 'ext', renderer: 'card_lite', form: NF_EXT_FORM },
  { id: 'DIN花键精简表_内', aliases: ['din精简'], label: 'DIN 5480 内花键参数表（精简版）', group: '精简版', summary: 'stub DIN lite int（不含公差列）', systems: [], direction: 'int', renderer: 'card_lite', form: DIN_FORM_INT },
  { id: 'DIN花键精简表_外', aliases: ['din精简外'], label: 'DIN 5480 外花键参数表（精简版）', group: '精简版', summary: 'stub DIN lite ext（不含公差列）', systems: [], direction: 'ext', renderer: 'card_lite', form: DIN_FORM_EXT },
  { id: 'ANSI花键精简表_内_中文', aliases: ['ansi精简'], label: 'ANSI 花键参数表（内·纯中文·精简版）', group: '精简版', summary: 'stub ANSI lite cn int（不含公差列）', systems: [], direction: 'int', renderer: 'card_lite', form: ANSI_FORM },
  { id: 'ANSI花键精简表_外_中文', aliases: ['ansi精简外中'], label: 'ANSI 花键参数表（外·纯中文·精简版）', group: '精简版', summary: 'stub ANSI lite cn ext（不含公差列）', systems: [], direction: 'ext', renderer: 'card_lite', form: ANSI_FORM },
  { id: 'ANSI花键精简表_内_英文', aliases: ['ansi精简内英'], label: 'ANSI 花键参数表（内·纯英文·精简版）', group: '精简版', summary: 'stub ANSI lite en int（不含公差列）', systems: [], direction: 'int', renderer: 'card_lite', form: ANSI_FORM },
  { id: 'ANSI花键精简表_外_英文', aliases: ['ansi精简外英'], label: 'ANSI 花键参数表（外·纯英文·精简版）', group: '精简版', summary: 'stub ANSI lite en ext（不含公差列）', systems: [], direction: 'ext', renderer: 'card_lite', form: ANSI_FORM },
  { id: '齿轮精简表', aliases: ['gear精简'], label: '齿轮参数表（GB/T 10095·精简版）', group: '精简版', summary: 'stub gear lite（不含公差列）', systems: [], direction: null, renderer: 'card_lite', form: GEAR_FORM },
];
const GB_COLUMNS = Array.from({ length: 21 }, (_, i) => ({
  tag: `(内)t${i}`, label: `项${i}`, unit: '', formula: 'stub', source: 'stub',
}));
const SYSTEMS = [{
  id: 'gb3478', aliases: ['gb'], label: 'GB', standard: 'GB', note: 'stub', x_note: 'stub',
  sides: [{
    id: 'int', label: '内花键', title: 'stub', grade_note: 'stub', alpha_note: 'stub',
    grades: [6], fits: [{ fit: 'H', code: 'H', label: 'H', preferred_45: true, memo: 'stub' }],
    roots: [{ key: 'flat', label: '平齿根', alphas: [30], note: 'stub' }],
    alphas: [30], pin: { applicable: true, label: 'Dp', formula: 'stub', md_formula: 'stub', standard: 'stub' },
    columns: GB_COLUMNS,
  }, {
    id: 'ext', label: '外花键', title: 'stub 外', grade_note: 'stub', alpha_note: 'stub',
    grades: [6], fits: [{ fit: 'h', code: 'h', label: 'h', preferred_45: false, memo: 'stub' }],
    roots: [{ key: 'flat', label: '平齿根', alphas: [30], note: 'stub' }],
    alphas: [30], pin: { applicable: false, reason: 'stub 外花键不含量棒' },
    columns: GB_COLUMNS,
  }],
}];
const GEAR_LABELS = [
  '法向模数', '齿数', '齿形角', '齿顶高系数', '螺旋角', '螺旋方向', '径向变位系数', '全齿高',
  '精度等级', '中心距及极限偏差', '配对齿轮图号', '配对齿轮齿数', '齿圈径向跳动公差',
  '公法线长度公差', '齿形公差', '齿距极限偏差', '齿向公差', '公法线', '公法线K',
];
const ANSI_INT = [
  '花键类型', '齿数', '径节', '压力角', '基圆直径', '节圆直径', '大径上差', '大径', '大径下差',
  '有效直径', '小径', '实际齿厚最大值', '作用齿厚最小值', '跨棒距上差', '跨棒距', '跨棒距下差', '量棒直径',
];
const ANSI_EXT = [
  '花键类型', '齿数', '径节', '压力角', '基圆直径', '节圆直径', '大径上差', '大径', '大径下差',
  '渐开线终止圆直径', '小径', '作用齿厚最大值', '实际齿厚最小值', '公法线上差', '公法线长度', '公法线下差', '跨测齿数',
];
const OPTIONS = {
  ok: true,
  card_types: CARD_TYPES,
  systems: SYSTEMS,
  pin_series: [1.0],
  gear_card: {
    expression_example: 'GEAR EX M3 Z20 ALPHA20 X0 DA66 DF52.5 BETA0 H30',
    expression_note: 'stub 表达式口径',
    missing_note: 'stub 缺项说明（GB/T 10095 未收）',
    columns: GEAR_LABELS.map((label) => ({ tag: label, label, unit: '', formula: 'stub', source: 'stub' })),
  },
  ansi_card: {
    profiles: [
      { id: 'ANSI30平齿根齿侧', code: 'ANSI30P', alpha: 30 },
      { id: 'ANSI30圆齿根齿侧', code: 'ANSI30R', alpha: 30 },
      { id: 'ANSI45圆齿根齿侧', code: 'ANSI45R', alpha: 45 },
    ],
    columns: { int: [], ext: [] },
    langs: [{ id: 'cn', label: '纯中文' }, { id: 'en', label: '纯英文' }],
    missing_note: 'stub ANSI 缺项（Table 4/5 未收）',
    note: 'stub ANSI 说明',
  },
  nf_card: {
    columns: Array.from({ length: 18 }, (_, i) => ({ tag: `nf${i}`, label: `NF项${i}`, unit: '', formula: 'stub', source: 'stub' })),
    modules: [0.5, 1, 2.5, 7.5],
    centering: [{ id: 'outer', label: '外径定心（Az=A）' }, { id: 'flank', label: '齿面定心（Az=A+0.3m）' }],
    roots: [{ id: 'flat', label: '平齿根' }, { id: 'fillet', label: '圆齿根' }],
    fits: [{ id: 'loose', label: '松动' }, { id: 'slide', label: '滑动' }, { id: 'fixed', label: '固定' }, { id: 'press', label: '压' }],
    missing_note: 'stub NF 缺项（p29 表外/ISO 档缺）',
    note: 'stub NF 说明',
  },
  nf_ext_card: {
    columns: Array.from({ length: 18 }, (_, i) => ({ tag: `nfe${i}`, label: `NFE项${i}`, unit: '', formula: 'stub', source: 'stub' })),
    modules: [0.5, 1, 2.5, 7.5],
    centering: [{ id: 'flank', label: '齿面定心（Dee=A−0.2m，模板）' }, { id: 'outer', label: '外径定心（Dee=A）' }],
    roots: [{ id: 'flat', label: '平齿根' }, { id: 'fillet', label: '圆齿根' }],
    fits: [{ id: 'loose', label: '松动' }, { id: 'slide', label: '滑动' }, { id: 'fixed', label: '固定' }, { id: 'press', label: '压' }],
    missing_note: 'stub NF 外缺项（p29 表外/ISO 档缺）',
    note: 'stub NF 外说明',
  },
  din_card: {
    columns: Array.from({ length: 26 }, (_, i) => ({ tag: `din${i}`, label: `DIN项${i}`, unit: '', formula: 'stub', source: 'stub' })),
    grades: [6, 7, 8, 9, 10, 11, 12],
    default: { m: 3, z: 38, d_b: 120, hub: '9H', shaft: '8f' },
    missing_note: 'stub DIN 缺项（Table 7 缺口）',
    note: 'stub DIN 说明',
  },
};

// ── fetch 桩 ─────────────────────────────────────────────────────
let lastPreviewModel = null;
let lastExportModel = null;
let lastExportUrl = '';
let lastReportModel = null;
let forcePreview404 = false;
let forceReportError = false;
const consoleErrors = [];
const realError = console.error.bind(console);
console.error = (...a) => { consoleErrors.push(a.join(' ')); };
function jsonResp(obj, status = 200) {
  return { ok: status < 400, status, json: async () => obj, text: async () => JSON.stringify(obj) };
}
function text404(path) {
  return {
    ok: false, status: 404,
    text: async () => 'not found',
    json: async () => { throw new SyntaxError(`${path} not valid JSON`); },
  };
}
// ── 表达式桩：从 stub 模型里的九字段表达式取 M/Z/ALPHA/X（够测回填）──
function stubExpr(mm) {
  const s = String(mm.expr == null ? '' : mm.expr).trim();
  if (!s) return null;
  const t = s.split(/\s+/);
  const num = (re, skip) => {
    const hit = t.find((x) => re.test(x));
    return hit ? Number(hit.slice(skip)) : NaN;
  };
  return { m: num(/^M/i, 1), z: num(/^Z/i, 1), alpha: num(/^ALPHA/i, 5), x: num(/^X/i, 1) || 0 };
}
function gearPreview(mm) {
  const missingSet = new Set(['精度等级', '齿圈径向跳动公差', '公法线长度公差', '齿形公差', '齿距极限偏差', '齿向公差']);
  const items = GEAR_LABELS.map((label) => ({
    tag: label, label, unit: '', value: missingSet.has(label) ? '—' : (label === '中心距及极限偏差'
      ? (mm.mate_z ? String(2 * (20 + Number(mm.mate_z)) / 2) : '—')
      : `v-${label}`),
    formula: 'stub 公式', source: 'stub 来源', missing: missingSet.has(label) || (label === '中心距及极限偏差' && !mm.mate_z),
  }));
  return jsonResp({ ok: true, card: '齿轮参数表', renderer: 'gear_table', readout: [{ k: '模数 m', v: '3' }], items, missing_note: 'stub 缺项说明' });
}
function ansiPreview(mm) {
  // 一卡一方向：方向由卡类型固定（旧请求显式 side 仍收）
  const side = /_外/.test(String(mm.card)) ? 'ext' : (mm.side || 'int');
  const labels = side === 'ext' ? ANSI_EXT : ANSI_INT;
  const missingSet = new Set(['跨棒距', '量棒直径', '公法线长度', '跨测齿数', '大径上差', '大径下差']);
  const typeText = mm.card.includes('英文') ? 'FLAT ROOT SIDE FIT' : '30°平齿根齿侧配合';
  // 表达式反解桩：P=25.4/m，齿廓按 α（30 → 列 A；45 → 45° 圆齿根齿侧）
  const ex = stubExpr(mm);
  if (ex && /^GEAR/i.test(String(mm.expr).trim())) {
    return jsonResp({ ok: false, error: 'ANSI 花键参数表：表达式 MARK 写的是 GEAR（齿轮），与卡片体系「SPLINE（花键）」不一致' }, 400);
  }
  if (ex && side === 'int' && /^SPLINE\s+EX\b/i.test(String(mm.expr).trim())) {
    return jsonResp({ ok: false, error: 'ANSI 花键参数表：表达式 KIND 写的是 EX（外），与卡片方向「内」不一致' }, 400);
  }
  if (ex && side === 'ext' && /^SPLINE\s+IN\b/i.test(String(mm.expr).trim())) {
    return jsonResp({ ok: false, error: 'ANSI 花键参数表：表达式 KIND 写的是 IN（内），与卡片方向「外」不一致' }, 400);
  }
  let fields = null;
  if (ex && ex.m > 0) {
    const prof = Math.abs(ex.alpha - 45) < 1e-9 ? 'ANSI45圆齿根齿侧' : 'ANSI30平齿根齿侧';
    fields = { p: 25.4 / ex.m, z: ex.z, profile: prof };
  }
  const items = labels.map((label) => ({
    tag: label, label, unit: '', value: missingSet.has(label) ? '—' : (label === '花键类型' ? typeText : `v-${label}`),
    formula: 'stub 公式', source: 'stub 来源', missing: missingSet.has(label),
  }));
  return jsonResp({
    ok: true, card: mm.card, side,
    renderer: mm.card.includes('英文') ? 'ansi_table_en' : 'ansi_table_cn',
    expr: ex ? String(mm.expr).trim() : null,
    fields,
    readout: [{ k: '径节 P/Ps', v: '16/32' }], items, missing_note: 'stub ANSI 缺项',
  });
}
const NF_ITEMS = [
  '执行标准', '定心方式', '模数', '齿数', '压力角', '齿根样式', '加工方法', '大径Az', '小径D',
  '基准尺寸', '量棒直径V', '跨棒距G', '大径上差', '大径下差', '小径上差', '小径下差', '跨棒距上差', '跨棒距下差',
];
function nfPreview(mm) {
  const ex = stubExpr(mm);
  const aVal = ex ? ex.m * (ex.z + 0.4 + 2 * ex.x) : Number(mm.a);
  const fields = ex ? { a: aVal, m: ex.m, z: ex.z } : null;
  const gap = Number(mm.a) === 210 || (ex && Math.abs(aVal - 210) < 1e-9);
  const missingSet = gap ? new Set(['齿数', '量棒直径V', '跨棒距G', '跨棒距上差', '跨棒距下差']) : new Set();
  const fitLabel = { loose: '松动', slide: '滑动', fixed: '固定', press: '压' }[mm.fit || 'fixed'];
  const press = (mm.fit || 'fixed') === 'press';
  const val = (tag) => tag === '大径Az' ? (mm.centering === 'flank' ? '302.25' : '300')
    : tag === '小径D' ? '285' : tag === '量棒直径V' ? '15' : tag === '跨棒距G' ? '270.508'
    : tag === '定心方式' ? (mm.centering === 'flank' ? '齿面定心' : '外径定心')
    : tag === '大径上差' ? '-0.078' : tag === '大径下差' ? '-0.130'
    : tag === '小径上差' ? '+0.052' : tag === '小径下差' ? '0'
    : tag === '跨棒距上差' ? (gap ? '—' : (press ? '+0.138' : '+0.052'))
    : tag === '跨棒距下差' ? (gap ? '—' : (press ? '+0.054' : '0'))
    : missingSet.has(tag) ? '—' : `v-${tag}`;
  const items = NF_ITEMS.map((tag) => ({
    tag, label: tag, unit: '', value: val(tag),
    formula: 'stub 公式', source: 'stub 来源', missing: missingSet.has(tag),
  }));
  const readout = gap
    ? [{ k: 'p29 内花键偏差（µm）', v: '—' }]
    : [
      { k: 'p29 内花键偏差（µm）', v: 'E +52/+0（+0.052/0 mm）；xm +76/+0（µm）' },
      { k: `配对外花键·${fitLabel}偏差（µm；p29）`, v: press ? 'E +138/+54；xm +202/+79' : 'E +42/-42；xm +61/-61' },
    ];
  return jsonResp({
    ok: true, card: mm.card, renderer: 'nf_table', fit: mm.fit || 'fixed',
    expr: ex ? String(mm.expr).trim() : null,
    fields,
    readout, items, missing: [...missingSet], missing_note: 'stub NF 缺项（p29 表外/ISO 档缺）',
  });
}
const DIN_INT_TAGS = ['N标记', 'N齿数', 'N模数', 'N压力角', 'N齿根圆', 'N齿根成形圆', 'N齿顶圆', 'N槽宽max', 'N槽宽min', 'N槽宽eff', 'N量圆', 'N量距max', 'N量距min'];
const DIN_EXT_TAGS = ['W标记', 'W齿数', 'W模数', 'W压力角', 'W齿顶圆', 'W齿根成形圆', 'W齿根圆', 'W齿厚svmax', 'W齿厚smax', 'W齿厚smin', 'W量圆', 'W量距max', 'W量距min'];
const NFE_TAGS = [
  '执行标准', '定心方式', '模数', '齿数', '压力角', '齿根样式', '加工方法', '大径Dee', '小径Die',
  '基准尺寸', '跨测齿数K', '公法线W', '大径上差', '大径下差', '小径上差', '小径下差', '公法线上差', '公法线下差',
];
function nfExtPreview(mm) {
  const rawExpr = String(mm.expr == null ? '' : mm.expr).trim();
  if (/^GEAR/i.test(rawExpr)) {
    return jsonResp({ ok: false, error: 'NF 外花键参数表：表达式 MARK 写的是 GEAR（齿轮），与卡片体系「SPLINE（花键）」不一致' }, 400);
  }
  if (/^SPLINE\s+IN\b/i.test(rawExpr)) {
    return jsonResp({ ok: false, error: 'NF 外花键参数表：表达式 KIND 写的是 IN（内），与卡片方向「外」不一致' }, 400);
  }
  const ex = stubExpr(mm);
  const aVal = ex ? ex.m * (ex.z + 0.4 + 2 * ex.x) : Number(mm.a);
  const fields = ex ? { a: aVal, m: ex.m, z: ex.z } : null;
  const gap = Number(mm.a) === 210 || (ex && Math.abs(aVal - 210) < 1e-9);
  const missingSet = gap ? new Set(['齿数', '跨测齿数K', '公法线W', '公法线上差', '公法线下差']) : new Set();
  const fitLabel = { loose: '松动', slide: '滑动', fixed: '固定', press: '压' }[mm.fit || 'fixed'];
  const fitE = { loose: '-0.11/-0.194', slide: '-0.02/-0.104', fixed: '+0.042/-0.042', press: '+0.138/+0.054' }[mm.fit || 'fixed'];
  const flank = (mm.centering || 'flank') !== 'outer';
  const fillet = mm.root === 'fillet';
  const val = (tag) => tag === '大径Dee' ? (flank ? '298.5' : '300')
    : tag === '小径Die' ? (fillet ? '279.795' : '282')
    : tag === '定心方式' ? (flank ? '齿面定心' : '外径定心')
    : tag === '加工方法' ? '滚齿'
    : tag === '齿根样式' ? (fillet ? '圆齿根' : '平齿根')
    : tag === '跨测齿数K' ? '6' : tag === '公法线W' ? '129.871'
    : tag === '大径下差' ? '-0.52' : tag === '小径上差' ? '+0.052'
    : tag === '公法线上差' ? (gap ? '—' : fitE.split('/')[0])
    : tag === '公法线下差' ? (gap ? '—' : fitE.split('/')[1])
    : missingSet.has(tag) ? '—' : `v-${tag}`;
  const items = NFE_TAGS.map((tag) => ({
    tag, label: tag, unit: '', value: val(tag),
    formula: 'stub 公式', source: 'stub 来源', missing: missingSet.has(tag),
  }));
  return jsonResp({
    ok: true, card: mm.card, renderer: 'nf_ext_table', fit: mm.fit || 'fixed',
    expr: ex ? String(mm.expr).trim() : null,
    fields,
    readout: gap ? [{ k: 'p29 外花键偏差（µm）', v: '—' }]
      : [{ k: `p29 外花键·${fitLabel}偏差（µm）`, v: `E ${fitE}；xm +61/-61` },
         { k: '跨测齿数 K / 公法线 W', v: '6 / 129.871' }],
    items, missing: [...missingSet], missing_note: 'stub NF 外缺项（p29 表外/ISO 档缺）',
  });
}
function dinPreview(mm) {
  // 一卡一方向：DIN 内/外两张单栏卡（旧请求显式 side 仍收）
  const ext = /_外/.test(String(mm.card)) || mm.side === 'ext';
  const rawExpr = String(mm.expr == null ? '' : mm.expr).trim();
  if (/^GEAR/i.test(rawExpr)) {
    return jsonResp({ ok: false, error: 'DIN 5480 花键参数表：表达式 MARK 写的是 GEAR（齿轮），与卡片体系「SPLINE（花键）」不一致' }, 400);
  }
  if (!ext && /^SPLINE\s+EX\b/i.test(rawExpr)) {
    return jsonResp({ ok: false, error: 'DIN 5480 内花键参数表：表达式 KIND 写的是 EX（外），与卡片方向「内」不一致' }, 400);
  }
  if (ext && /^SPLINE\s+IN\b/i.test(rawExpr)) {
    return jsonResp({ ok: false, error: 'DIN 5480 外花键参数表：表达式 KIND 写的是 IN（内），与卡片方向「外」不一致' }, 400);
  }
  const tags = ext ? DIN_EXT_TAGS : DIN_INT_TAGS;
  const ex = stubExpr(mm);
  const dbVal = ex ? ex.m * (ex.z + 1.1 + 2 * ex.x) : Number(mm.d_b);
  const fields = ex ? { m: ex.m, z: ex.z, d_b: dbVal } : null;
  const missingSet = new Set();
  const gap = Number(mm.m) >= 5 || (ex && ex.m >= 5);
  const gapTags = ext
    ? ['W齿厚svmax', 'W齿厚smax', 'W齿厚smin']
    : ['N槽宽max', 'N槽宽min', 'N槽宽eff'];
  if (gap) { for (const t of gapTags) missingSet.add(t); }
  const design = ext ? 'W120×3×38×8f' : 'N120×3×38×9H';
  const designLabel = ext ? 'Welle DIN 5480' : 'Nabe DIN 5480';
  const val = (tag) => tag.endsWith('标记') ? design
    : tag === 'N槽宽max' ? (gap ? '—' : '6.361') : tag === 'N槽宽eff' ? (gap ? '—' : '6.271')
    : tag === 'W齿厚svmax' ? (gap ? '—' : '6.243') : tag === 'W齿厚smin' ? (gap ? '—' : '6.18')
    : `v-${tag}`;
  const items = tags.map((tag) => ({
    tag, label: tag.endsWith('标记') ? designLabel : tag, unit: '', value: val(tag),
    formula: 'stub 公式', source: 'stub 来源', missing: missingSet.has(tag),
  }));
  return jsonResp({
    ok: true, card: mm.card, side: ext ? 'ext' : 'int', renderer: 'din_table', anchor: !gap,
    expr: ex ? String(mm.expr).trim() : null,
    fields,
    readout: [{ k: 'e₂ = s₁（名义）', v: gap ? '—' : '6.271' }], items,
    missing: [...missingSet], missing_note: 'stub DIN 缺项',
  });
}
// 精简卡：9 项固定、无任何公差列；表达式反解复用 stubExpr，回填 fields。
const LITE_INT_TAGS = ['执行标准', '模数 m', '齿数 z', '齿形角 αD', '齿根样式', '大径 Dei', '小径 Dii', '量棒直径 Dp', '测量跨棒距 Md'];
const LITE_EXT_TAGS = ['执行标准', '模数 m', '齿数 z', '齿形角 αD', '齿根样式', '大径 Dee', '小径 Die', '跨测齿数 Kn', '公法线长度 Wn'];
function cardLitePreview(mm) {
  const spec = LITE_STUB[mm.card] || { n: 9, labels: [] };
  const labels = spec.labels.length ? spec.labels
    : Array.from({ length: spec.n }, (_, i) => `精简项${i + 1}`);
  const items = labels.map((label, i) => ({
    tag: `(简stub)${label}`, label, unit: '', value: `v-${i + 1}`,
    formula: 'stub 公式', source: 'stub 来源', missing: false,
  }));
  return jsonResp({
    ok: true, card: mm.card, renderer: 'card_lite',
    readout: [{ k: '基本参数', v: 'stub' }], items, missing: [],
    missing_note: 'stub 精简卡：不含任何公差列',
  });
}
// 精简版扩体系：每卡字段数/标签（stub 与生产表同构；仅供 DOM 断言）
const LITE_STUB = {
  'NF花键精简表_内': { n: 11, labels: ['执行标准', '定心方式', '模数 m', '齿数 z', '压力角 a', '齿根样式', '加工方法', '大径 Az', '小径 D', '量棒直径 V', '跨棒距 G'] },
  'NF花键精简表_外': { n: 11, labels: ['执行标准', '定心方式', '模数 m', '齿数 z', '压力角 a', '齿根样式', '加工方法', '大径 Dee', '小径 Die', '跨测齿数 K', '公法线 W'] },
  'DIN花键精简表_内': { n: 10, labels: ['标记 N', '齿数 z', '模数 m', '压力角 α', '齿根圆 d_f2', '齿根成形圆 d_Ff2', '齿顶圆 d_a2', '量圆 D_M', '量圆距 M2_max', '量圆距 M2_min'] },
  'DIN花键精简表_外': { n: 10, labels: ['标记 W', '齿数 z', '模数 m', '压力角 α', '齿顶圆 d_a1', '齿根成形圆 d_Ff1', '齿根圆 d_f1', '量圆 D_M', '量圆距 M1_max', '量圆距 M1_min'] },
  'ANSI花键精简表_内_中文': { n: 10, labels: ['花键类型', '齿数 z', '径节 P', '压力角 α', '基圆直径 Db', '节圆直径 D', '大径', '小径', '跨棒距 M', '量棒直径 Dp'] },
  'ANSI花键精简表_外_中文': { n: 10, labels: ['花键类型', '齿数 z', '径节 P', '压力角 α', '基圆直径 Db', '节圆直径 D', '大径', '小径', '公法线 W', '跨测齿数 K'] },
  'ANSI花键精简表_内_英文': { n: 10, labels: ['SPLINE TYPE', 'TEETH z', 'PITCH P', 'ALPHA', 'BASE DIA. Db', 'PITCH DIA. D', 'MAJOR DIA.', 'MINOR DIA.', 'PIN DIST. M', 'PIN DIA. Dp'] },
  'ANSI花键精简表_外_英文': { n: 10, labels: ['SPLINE TYPE', 'TEETH z', 'PITCH P', 'ALPHA', 'BASE DIA. Db', 'PITCH DIA. D', 'MAJOR DIA.', 'MINOR DIA.', 'BASE TANG. W', 'SPAN TEETH K'] },
  '齿轮精简表': { n: 9, labels: ['模数 m', '齿数 z', '压力角 α', '变位系数 x', '分度圆 d', '齿顶圆 da', '齿根圆 df', '公法线 W', '跨齿数 K'] },
};
function litePreview(mm) {
  const ext = /_外/.test(String(mm.card));
  const tags = ext ? LITE_EXT_TAGS : LITE_INT_TAGS;
  const ex = stubExpr(mm);
  if (!ex) return jsonResp({ ok: false, error: 'GB 花键精简卡：表达式缺失' }, 400);
  const val = (tag) => tag === '执行标准' ? 'GB/T 3478.1-2008'
    : tag === '模数 m' ? String(ex.m) : tag === '齿数 z' ? String(ex.z)
    : tag === '齿形角 αD' ? ex.alpha + '°' : tag === '齿根样式' ? '平齿根'
    : tag === '大径 Dei' ? '65.4' : tag === '小径 Dii' ? '57.344'
    : tag === '量棒直径 Dp' ? '4.5' : tag === '测量跨棒距 Md' ? '61.5'
    : tag === '大径 Dee' ? '66' : tag === '小径 Die' ? '52.5'
    : tag === '跨测齿数 Kn' ? '3' : tag === '公法线长度 Wn' ? '22.981'
    : `v-${tag}`;
  const items = tags.map((tag) => ({
    tag: '(简)' + tag, label: tag, unit: '', value: val(tag),
    formula: 'stub 公式', source: 'stub 来源', missing: false,
  }));
  return jsonResp({
    ok: true, card: ext ? 'GB花键精简表_外' : 'GB花键精简表_内', renderer: 'spline_lite',
    expr: String(mm.expr).trim(),
    fields: { grade: mm.grade, fit: mm.fit, root: 'flat' },
    readout: [{ k: '模数 m', v: String(ex.m) }, { k: '齿数 z', v: String(ex.z) }],
    items, missing: [], missing_note: 'stub 精简卡：不含任何公差列',
  });
}
global.fetch = async (u, opts = {}) => {
  const url = String(u);
  if (url.startsWith('/api/spline_options')) return jsonResp(OPTIONS);
  if (url.startsWith('/api/card_preview')) {
    if (forcePreview404) return text404('/api/card_preview');
    const model = JSON.parse(opts.body || '{}');
    lastPreviewModel = model;
    if (model.card === '齿轮参数表') return gearPreview(model);
    if (String(model.card).startsWith('ANSI')) return ansiPreview(model);
    if (model.card === 'NF内花键参数表') return nfPreview(model);
    if (model.card === 'NF外花键参数表') return nfExtPreview(model);
    if (String(model.card).startsWith('GB花键精简表')) return litePreview(model);
    if (LITE_STUB[model.card]) return cardLitePreview(model);
    if (String(model.card).startsWith('DIN')) return dinPreview(model);
    return jsonResp({ ok: false, error: 'stub 只支持新卡' }, 400);
  }
  if (url.startsWith('/api/card_report')) {
    lastReportModel = JSON.parse(opts.body || '{}');
    if (forceReportError) return jsonResp({ ok: false, error: 'stub 计算书错误' }, 400);
    return jsonResp({
      ok: true,
      title: `${lastReportModel.card}计算书`,
      markdown: `# ${lastReportModel.card} 计算书\n\n- 同源：与卡片同一份计算\n\n| 项 | 值 |\n|---|---|\n| 模数 m | 3 |\n`,
    });
  }
  if (url.startsWith('/api/card_export')) {
    lastExportUrl = url;
    lastExportModel = JSON.parse(opts.body || '{}');
    const v = String(lastExportModel.card).startsWith('ANSI') ? ansiPreview(lastExportModel)
      : lastExportModel.card === '齿轮参数表' ? gearPreview(lastExportModel)
      : lastExportModel.card === 'NF内花键参数表' ? nfPreview(lastExportModel)
      : lastExportModel.card === 'NF外花键参数表' ? nfExtPreview(lastExportModel)
      : String(lastExportModel.card).startsWith('GB花键精简表') ? litePreview(lastExportModel)
      : LITE_STUB[lastExportModel.card] ? cardLitePreview(lastExportModel)
      : String(lastExportModel.card).startsWith('DIN') ? dinPreview(lastExportModel)
      : jsonResp({ ok: false, error: 'bad card' }, 400);
    if (!v.ok) return v;
    return jsonResp({ ok: true, message: 'stub 已生成', pending: lastExportModel.at == null });
  }
  return jsonResp({ ok: true });
};

// ── 最小 DOM 垫片（照 spline_gui_smoke.mjs）────────────────────────
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
      else if (this.tagName === 'SELECT' && c.tagName === 'OPTGROUP') {
        c._parent = this;
        this.children.push(c);
      } else if (this.tagName === 'OPTGROUP' && c.tagName === 'OPTION') {
        this.options.push(c);
        if (this._parent) this._parent.options.push(c);
      } else this.children.push(c);
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
const SELECT_IDS = new Set(['cardType', 'sys', 'grade', 'fit', 'alpha', 'root']);
const TEXTAREA_IDS = new Set(['expr']);
global.document = {
  getElementById(id) {
    if (!els.has(id)) {
      const e = mkEl(id);
      if (SELECT_IDS.has(id)) e.tagName = 'SELECT';
      if (TEXTAREA_IDS.has(id)) e.tagName = 'TEXTAREA';
      els.set(id, e);
    }
    return els.get(id);
  },
  createElement(tag) { const e = mkEl('dyn'); e.tagName = String(tag).toUpperCase(); return e; },
  createTextNode(t) { const e = mkEl('text'); e.textContent = String(t); return e; },
  querySelectorAll() { return []; },
  querySelector() { return null; },
  addEventListener() {},
  activeElement: null,
};
let closed = false;
global.window = { addEventListener() {}, close() { closed = true; } };
global.location = { search: '', href: 'http://127.0.0.1:9/spline' };
global.setTimeout = (fn) => { if (typeof fn === 'function') fn(); return 0; };
global.clearTimeout = () => {};
global.setInterval = () => 0;
global.URLSearchParams = class { constructor() {} get() { return null; } };

// ── 跑页面脚本 + 探针 ─────────────────────────────────────────────
const probed = script.replace(/\}\)\(\);\s*$/, `;globalThis.__card = {
  get opt() { return OPT; },
  get card() { return CARD; },
  get sys() { return SYS; },
  get side() { return SIDE; },
  panelOf, applyPanels, formOf, renderCardForm, currentCardModel, refresh, schedulePreview,
  formControl: (k) => FORM_CTL.get(k),
};
})();`);
try {
  (0, eval)(renderZh(fs.readFileSync(htmlPath.replace(/[^/]+$/, 'ocsm_gui_common.js'), 'utf8')));
  (0, eval)(probed);
} catch (e) {
  errors.push('脚本求值异常: ' + (e && e.stack ? e.stack : e));
}
const tick = async (n = 6) => { for (let i = 0; i < n; i++) await new Promise((r) => setImmediate(r)); };
await tick();

const H = globalThis.__card;
const el = (id) => document.getElementById(id);
check(!!H, '探针 __card 未挂上（脚本初始化崩溃？）');
if (!H) report();

// ── ① 统一外观：表驱动骨架 + GB 面板同款分区 ──────────────────────
check(el('cardType').options.length === 22, `卡类型应 22 项：${el('cardType').options.map((o) => o.value)}`);
check(H.card && H.card.id === '花键参数表', `默认卡类型：${H.card && H.card.id}`);
// ② 卡名（下拉闭合态）：首项带 GB；⑤ 每张卡名带标准号，一眼看出哪套标准
check(el('cardType').options[0].textContent === 'GB 花键参数表（内）', `首项卡名：${el('cardType').options[0].textContent}`);
for (const o of el('cardType').options) {
  check(/GB|ANSI|NF|DIN/.test(o.textContent), `卡名应带标准号：${o.textContent}`);
}
// 下拉分组（精简版）：11 张精简卡收进 <optgroup label="精简版">；普通卡平铺。
const liteGroups = el('cardType').children.filter((c) => c.tagName === 'OPTGROUP');
check(liteGroups.length === 1 && liteGroups[0].label === '精简版' && liteGroups[0].options.length === 11,
  `精简版分组：${liteGroups.map((g) => `${g.label}(${g.options.length})`)}`);
// ⑤ 截断兜底：hover title = 全名 + 口径（不展开也能确认）
check((el('cardType').title || '').includes('GB 花键参数表'), `卡类型 title 应含全名：${el('cardType').title}`);
check(html.includes('class="card typebar"'), '卡类型条应有 typebar 类（独占一行自适应）');
check(html.indexOf('id="cardType"') < html.indexOf('id="splinePanel"'), '卡类型下拉应在面板之前');
// ④ 版面：表单+读数同列、结果预览在另一列（不再左侧大片留白）
check(html.includes('class="card mid formmid"'), '通用卡片中部应单列（读数在表单下方）');
check(html.includes('class="card itemsCard"'), '结果预览卡应有 itemsCard 类（右侧栏）');
check(html.indexOf('id="formMain"') < html.indexOf('id="cardReadout"'), '结果读数应在表单之后（同列）');
check(html.indexOf('id="cardReadout"') < html.indexOf('id="items"'), '结果预览应在读数之后（右列）');
check(H.panelOf(H.card) === 'spline', '默认应 spline 面板');
check(el('splinePanel').style.display === '' && el('cardPanel').style.display === 'none',
  '默认只有 GB 面板可见');
check(!html.includes('id="gearExpr"') && !html.includes('id="nfA"') && !html.includes('id="dinM"'),
  '页面不应再有手写卡专属控件 id');
// 通用骨架的分区/类名与 GB 面板同款（静态结构断言；以后加卡自动继承）
check(html.includes('id="cardPanel"') && html.includes('id="formParams"') && html.includes('id="formMain"'),
  '缺通用卡片面板骨架');
check(html.includes('<div class="card mid">') && html.includes('<div class="card top">') && html.includes('class="card bot"'),
  '分区/按钮位置应保持');
check(!html.includes('id="gearPanel"') && !html.includes('id="ansiPanel"') && !html.includes('id="nfPanel"') && !html.includes('id="dinPanel"'),
  '不应再给每卡写一套 HTML 面板');
check(html.includes('id="cardHint"') && html.includes('id="cardReadout"'), '通用面板缺提示/读数位');
// ①b 计算书（与卡片同源）：按钮 + 面板 + 摘要表落图纸入口
check(html.includes('id="report"') && html.includes('id="reportCard"') && html.includes('id="reportMd"'),
  '缺计算书按钮/面板');
check(html.includes('id="reportSummary"') && html.includes('同一份计算'),
  '摘要表按钮应声明同源（title/说明）');

// ── ② 切到齿轮卡：字段顺序/控件类型 + 19 项 + 配对齿数影响中心距 ──
el('cardType').value = '齿轮参数表';
el('cardType')._fire('change', el('cardType'));
await tick();
check(H.card.id === '齿轮参数表', `切卡后 card=${H.card.id}`);
check(H.panelOf(H.card) === 'form', '齿轮应走通用面板');
check(el('cardPanel').style.display === '' && el('splinePanel').style.display === 'none', '通用面板可见/GB 隐藏');
check(el('cardHint').title.includes('10095'), `缺项说明应进 title：${el('cardHint').title}`);
// 字段顺序 = form 声明顺序；textarea 只出在中部主输入区
const gearSeq = el('formParams').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id);
check(gearSeq.join(',') === 'f_mate_z,f_mate_dwg,f_grade,f_center',
  `齿轮顶部字段顺序：${gearSeq}`);
check(el('formMain').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id).join(',') === 'f_expr',
  '齿轮表达式应在中部主输入区');
check(H.formControl('mate_z').tagName === 'INPUT' && H.formControl('mate_z').type === 'number',
  '配对齿数应为 number 控件');
check(H.formControl('mate_dwg').type === 'text', '图号应为 text 控件');
check(H.formControl('expr').tagName === 'TEXTAREA', '表达式应为 textarea');
check(H.formControl('expr').value.startsWith('GEAR EX'), `齿轮默认表达式：${H.formControl('expr').value}`);
await H.refresh();
check(lastPreviewModel && lastPreviewModel.card === '齿轮参数表', `预览模型 card：${JSON.stringify(lastPreviewModel)}`);
check(String(lastPreviewModel.expr).startsWith('GEAR EX'), `预览应带表达式：${lastPreviewModel.expr}`);
check(el('items').innerHTML.split('class="row"').length - 1 === 19, `齿轮卡应 19 项：${el('items').innerHTML.slice(0, 100)}`);
check(el('items').innerHTML.includes('—'), '缺项应显示「—」');
check(el('items').innerHTML.includes('title="stub 公式'), '公式应在行 title 里');
H.formControl('mate_z').value = '20';
H.formControl('mate_z')._fire('input', H.formControl('mate_z'));
await tick();
check(lastPreviewModel && Number(lastPreviewModel.mate_z) === 20, `配对齿数应进模型：${JSON.stringify(lastPreviewModel)}`);
check(el('items').innerHTML.includes('40'), '给了配对齿数 → 中心距出现在预览里');
H.formControl('grade').value = '7-7-7';
H.formControl('grade')._fire('input', H.formControl('grade'));
await tick();
check(lastPreviewModel.grade === '7-7-7', `精度等级应进模型：${JSON.stringify(lastPreviewModel)}`);

// 齿轮卡出表：无 at → /api/card_export + 待放置；有 at → 直插
el('ok').click();
await tick();
check(lastExportUrl.startsWith('/api/card_export'), `齿轮导出 URL：${lastExportUrl}`);
check(lastExportModel && lastExportModel.card === '齿轮参数表' && lastExportModel.at === null, `导出模型：${JSON.stringify(lastExportModel)}`);
check(closed === true, '出表成功后应自动关窗');
closed = false;
el('atX').value = '10'; el('atY').value = '20'; el('rot').value = '15';
el('ok').click();
await tick();
check(JSON.stringify(lastExportModel.at) === '[10,20]', `at 应进模型：${JSON.stringify(lastExportModel.at)}`);
el('atX').value = ''; el('atY').value = '';

// ── ⑧ 计算书（与预览同一份模型）+ 摘要表落图纸（同一卡导出）+ 红字错误 ──
closed = false; // 上一次出表已置 true；计算书/摘要表都不应关窗
el('report').click();
await tick();
check(lastReportModel && lastReportModel.card === '齿轮参数表', `计算书模型应同卡：${JSON.stringify(lastReportModel)}`);
check(el('reportCard').style.display === '', '计算书面板应常显（不自动关窗）');
check((el('reportMd').textContent || '').includes('# 齿轮参数表 计算书'), `报告正文应进面板：${el('reportMd').textContent}`);
check((el('reportTitle').textContent || '').includes('齿轮'), `报告标题：${el('reportTitle').textContent}`);
check(closed === false, '生成计算书不应关窗');
el('reportSummary').click();
await tick();
check(lastExportUrl.startsWith('/api/card_export'), `摘要表导出 URL：${lastExportUrl}`);
check(lastExportModel && lastExportModel.card === '齿轮参数表', `摘要表模型同卡：${JSON.stringify(lastExportModel)}`);
check(el('reportCard').style.display === '', '摘要表落图纸后面板应保留（可继续看报告）');
forceReportError = true;
el('report').click();
await tick();
check(el('status').className === 'bad' && (el('status').textContent || '').includes('stub 计算书错误'),
  `计算书错误应红框可见：${el('status').className}/${el('status').textContent}`);
forceReportError = false;
el('reportClose').click();
check(el('reportCard').style.display === 'none', '关闭按钮应隐藏报告面板');

// ── ③ ANSI 内·纯中文：齿廓（选项表下发）+ P/z + 17 项（面板不再放方向）──
el('cardType').value = 'ANSI花键参数表_中文';
el('cardType')._fire('change', el('cardType'));
await tick();
check(H.card.id === 'ANSI花键参数表_中文', `ANSI CN 卡：${H.card.id}`);
check(H.panelOf(H.card) === 'form', 'ANSI 面板为通用骨架');
check(el('cardHint').title.includes('Table 4/5') || el('cardHint').title.includes('ANSI'), `ANSI 缺项进 title：${el('cardHint').title}`);
check(H.formControl('side') === undefined, 'ANSI 面板不应再有方向字段（一卡一方向）');
check(H.formControl('profile').options.length === 3, `ANSI 齿廓清单来自选项表：${H.formControl('profile').options.map((o) => o.value)}`);
check(H.formControl('p').value === '16' && H.formControl('z').value === '20', 'ANSI 默认 P16/N20');
const ansiSeq = el('formParams').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id);
check(ansiSeq.join(',') === 'f_profile,f_p,f_z', `ANSI 字段顺序：${ansiSeq}`);
check(el('formMain').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id).join(',') === 'f_expr',
  'ANSI 表达式应在中部主输入区（textarea）');
await H.refresh();
check(lastPreviewModel.card === 'ANSI花键参数表_中文' && !('side' in lastPreviewModel), `ANSI 预览模型不应带 side：${JSON.stringify(lastPreviewModel)}`);
check(Number(lastPreviewModel.p) === 16 && Number(lastPreviewModel.z) === 20, `P/z 应进模型：${JSON.stringify(lastPreviewModel)}`);
check(el('items').innerHTML.split('class="row"').length - 1 === 17, `ANSI 卡应 17 项：${el('items').innerHTML.slice(0, 100)}`);
check(el('items').innerHTML.includes('30°平齿根齿侧配合'), '中文版类型值应为中文');
// 切到「外·纯中文」卡 → 方向由卡决定（面板无 select）
el('cardType').value = 'ANSI花键参数表_外_中文';
el('cardType')._fire('change', el('cardType'));
await tick();
check(H.card.id === 'ANSI花键参数表_外_中文', `ANSI 外卡：${H.card.id}`);
check(H.formControl('side') === undefined, '外卡同样无方向字段');
await H.refresh();
check(!('side' in lastPreviewModel), `外卡模型不应带 side：${JSON.stringify(lastPreviewModel)}`);
check(el('items').innerHTML.includes('渐开线终止圆直径') || el('items').innerHTML.includes('v-渐开线终止圆直径'), '外卡应有渐开线终止圆直径行');
// 回内卡继续其余断言
el('cardType').value = 'ANSI花键参数表_中文';
el('cardType')._fire('change', el('cardType'));
await tick();
H.formControl('profile').value = 'ANSI45圆齿根齿侧';
H.formControl('profile')._fire('change', H.formControl('profile'));
await tick();
check(lastPreviewModel.profile === 'ANSI45圆齿根齿侧', `齿廓应进模型：${JSON.stringify(lastPreviewModel)}`);
// ★ 表达式 → 反解回填：粘入新表达式后 P/N/齿廓控件自动回填（后端 fields）
H.formControl('profile').value = 'ANSI30平齿根齿侧';
H.formControl('expr').value = 'SPLINE IN M3.175 Z22 ALPHA45 X0 BETA0 H30';
H.formControl('expr')._fire('input', H.formControl('expr'));
await tick();
check(String(lastPreviewModel.expr).startsWith('SPLINE IN M3.175'), `表达式应进模型：${JSON.stringify(lastPreviewModel.expr)}`);
check(H.formControl('p').value === '8' && H.formControl('z').value === '22',
  `表达式反解应回填 P=8/N=22：P=${H.formControl('p').value} N=${H.formControl('z').value}`);
check(H.formControl('profile').value === 'ANSI45圆齿根齿侧',
  `表达式 α45 应回填齿廓：${H.formControl('profile').value}`);
// ★ 表达式错误路径 → 红字可见（共享助手；不动已修好的 #status 优先级）
H.formControl('expr').value = 'GEAR IN M1.5875 Z20 ALPHA30 X0 BETA0 H30';
H.formControl('expr')._fire('input', H.formControl('expr'));
await tick();
check((el('status').textContent || '').includes('MARK'), `表达式 MARK 冲突应可见：${el('status').textContent}`);
check(el('status').className === 'bad', `表达式错误应红框：${el('status').className}`);
H.formControl('expr').value = '';
H.formControl('expr')._fire('input', H.formControl('expr'));
await tick();

// ── ④ ANSI 纯英文：同构，card 字段换英文卡 ────────────────────────
el('cardType').value = 'ANSI花键参数表_英文';
el('cardType')._fire('change', el('cardType'));
await tick();
await H.refresh();
check(lastPreviewModel.card === 'ANSI花键参数表_英文', `ANSI EN 卡：${JSON.stringify(lastPreviewModel)}`);
check(el('items').innerHTML.includes('FLAT ROOT SIDE FIT'), '英文版类型值应为英文');
check(el('items').innerHTML.split('class="row"').length - 1 === 17, '英文版仍 17 项');
closed = false;
el('atX').value = '5'; el('atY').value = '6';
el('ok').click();
await tick();
check(lastExportUrl.startsWith('/api/card_export'), `ANSI 导出 URL：${lastExportUrl}`);
check(lastExportModel.card === 'ANSI花键参数表_英文' && JSON.stringify(lastExportModel.at) === '[5,6]', `ANSI 导出模型：${JSON.stringify(lastExportModel)}`);
el('atX').value = ''; el('atY').value = '';

// ── ⑤ NF 内花键参数表：6 字段 + 公差不再整片「—」+ 配合只动读数 ──
el('cardType').value = 'NF内花键参数表';
el('cardType')._fire('change', el('cardType'));
await tick();
check(H.card.id === 'NF内花键参数表', `NF 卡：${H.card.id}`);
check(el('cardHint').title.includes('p29'), `NF 缺项说明应进 title：${el('cardHint').title}`);
const nfSeq = el('formParams').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id);
check(nfSeq.join(',') === 'f_a,f_m,f_z,f_centering,f_root,f_fit', `NF 字段顺序：${nfSeq}`);
check(el('formMain').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id).join(',') === 'f_expr',
  'NF 表达式应在中部主输入区');
check(H.formControl('a').value === '300' && H.formControl('m').value === '7.5' && H.formControl('z').value === '38', 'NF 默认示例 A300/M7.5/Z38');
check(H.formControl('centering').options.length === 2 && H.formControl('root').options.length === 2, 'NF 定心/齿根清单来自选项表');
check(H.formControl('fit').options.length === 4 && H.formControl('fit').value === 'fixed', 'NF 配合类别四档、默认固定');
await H.refresh();
check(lastPreviewModel && lastPreviewModel.card === 'NF内花键参数表', `NF 预览模型：${JSON.stringify(lastPreviewModel)}`);
check(Number(lastPreviewModel.a) === 300 && Number(lastPreviewModel.m) === 7.5 && Number(lastPreviewModel.z) === 38, `A/m/z 应进模型：${JSON.stringify(lastPreviewModel)}`);
check(lastPreviewModel.fit === 'fixed', `NF 配合应进模型：${JSON.stringify(lastPreviewModel)}`);
check(el('items').innerHTML.split('class="row"').length - 1 === 18, `NF 卡应 18 项：${el('items').innerHTML.slice(0, 100)}`);
check(el('items').innerHTML.includes('270.508'), 'NF 锚点跨棒距应出现在预览里');
// 修改②：6 个公差格必须有值（R7/H7/p29 E），不再整片「—」
const tolLabels = ['大径上差', '大径下差', '小径上差', '小径下差', '跨棒距上差', '跨棒距下差'];
for (const t of tolLabels) {
  const row = el('items').innerHTML.split('class="row"').find((r) => r.includes('>' + t + '<')) || '';
  check(row !== '' && !row.includes('>—<'), `NF 公差 ${t} 应有值：${row.slice(0, 120)}`);
}
check(el('items').innerHTML.includes('+0.052') && el('items').innerHTML.includes('-0.078'), 'NF 公差应含 R7/H7/p29 数值');
check(el('cardReadout').innerHTML.includes('p29'), 'NF 读数应含 p29 出处');
check(el('cardReadout').innerHTML.includes('E +52/+0'), 'NF 读数应含内花键 E 偏差');
check(el('cardReadout').innerHTML.includes('固定'), 'NF 读数应含所选配合');
// 配合类别 → 只影响配对外花键读数（不动内花键公差）
H.formControl('fit').value = 'press';
H.formControl('fit')._fire('change', H.formControl('fit'));
await tick();
check(lastPreviewModel.fit === 'press', `配合类别应进模型：${JSON.stringify(lastPreviewModel)}`);
check(el('cardReadout').innerHTML.includes('压') && el('cardReadout').innerHTML.includes('+138/+54'), '换成压配合 → 配对外花键读数随之变');
// 定心方式 → Az
H.formControl('centering').value = 'flank';
H.formControl('centering')._fire('change', H.formControl('centering'));
await tick();
check(lastPreviewModel.centering === 'flank', `定心方式应进模型：${JSON.stringify(lastPreviewModel)}`);
check(el('items').innerHTML.includes('302.25'), '齿面定心 → Az=302.25');
// NF 出表（无 at → 待放置；有 at → 直插）
closed = false;
el('ok').click();
await tick();
check(lastExportUrl.startsWith('/api/card_export'), `NF 导出 URL：${lastExportUrl}`);
check(lastExportModel.card === 'NF内花键参数表' && lastExportModel.at === null, `NF 导出模型：${JSON.stringify(lastExportModel)}`);
check(closed === true, 'NF 出表成功后应自动关窗');
closed = false;
el('atX').value = '7'; el('atY').value = '8';
el('ok').click();
await tick();
check(JSON.stringify(lastExportModel.at) === '[7,8]', `NF at 应进模型：${JSON.stringify(lastExportModel.at)}`);
el('atX').value = ''; el('atY').value = '';
// 表外 A=210 → 跨棒距公差「—」（不外推）
H.formControl('centering').value = 'outer';
H.formControl('centering')._fire('change', H.formControl('centering'));
H.formControl('a').value = '210';
H.formControl('a')._fire('input', H.formControl('a'));
await tick();
check(lastPreviewModel.a === 210, `表外 A 应进模型：${JSON.stringify(lastPreviewModel)}`);
check(el('items').innerHTML.includes('—'), '表外 A=210 → 标缺「—」');
// ★ NF 表达式 → A=m(z+0.4+2x)/m/z 回填
H.formControl('a').value = '300';
H.formControl('a')._fire('input', H.formControl('a'));
await tick();
H.formControl('expr').value = 'SPLINE IN M7.5 Z40 ALPHA20 X0.8 BETA0 H30';
H.formControl('expr')._fire('input', H.formControl('expr'));
await tick();
check(String(lastPreviewModel.expr).startsWith('SPLINE IN M7.5 Z40'), `NF 表达式应进模型：${JSON.stringify(lastPreviewModel.expr)}`);
check(H.formControl('a').value === '315' && H.formControl('m').value === '7.5' && H.formControl('z').value === '40',
  `NF 表达式应回填 A=315/m=7.5/z=40：A=${H.formControl('a').value} m=${H.formControl('m').value} z=${H.formControl('z').value}`);

// ── ⑤b NF 外花键参数表（新卡）：7 字段 + 18 项 + 模板锚点 + 表达式 + 出表 ──
el('cardType').value = 'NF外花键参数表';
el('cardType')._fire('change', el('cardType'));
await tick();
check(H.card.id === 'NF外花键参数表', `NF 外卡：${H.card.id}`);
check(el('cardHint').title.includes('p29') && el('cardHint').title.includes('h12'), `NF 外缺项说明进 title：${el('cardHint').title}`);
const nfeSeq = el('formParams').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id);
check(nfeSeq.join(',') === 'f_a,f_m,f_z,f_centering,f_root,f_fit', `NF 外字段顺序：${nfeSeq}`);
check(el('formMain').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id).join(',') === 'f_expr',
  'NF 外表达式应在中部主输入区');
check(H.formControl('centering').value === 'flank', `NF 外缺省齿面定心：${H.formControl('centering').value}`);
check(H.formControl('centering').options.map((o) => o.value).join(',') === 'flank,outer',
  `NF 外定心清单应来自 nf_ext_card：${H.formControl('centering').options.map((o) => o.value)}`);
check(H.formControl('fit').value === 'fixed' && H.formControl('fit').options.length === 4, 'NF 外配合四档默认固定');
await H.refresh();
check(lastPreviewModel && lastPreviewModel.card === 'NF外花键参数表', `NF 外预览模型：${JSON.stringify(lastPreviewModel)}`);
check(el('items').innerHTML.split('class="row"').length - 1 === 18, `NF 外应 18 项：${el('items').innerHTML.slice(0, 100)}`);
check(el('items').innerHTML.includes('298.5') && el('items').innerHTML.includes('129.871'), 'NF 外模板锚点 Dee/W 应在预览里');
check(el('items').innerHTML.includes('滚齿') && el('items').innerHTML.includes('齿面定心'), 'NF 外应显示滚齿/齿面定心（外花键口径）');
check(el('items').innerHTML.includes('-0.52') && el('items').innerHTML.includes('+0.052'), 'NF 外公差应含 h12/H7 数值');
check(el('cardReadout').innerHTML.includes('p29') && el('cardReadout').innerHTML.includes('129.871'), 'NF 外读数应含 p29 与 W');
// 配合类别 → 只动公法线公差
H.formControl('fit').value = 'press';
H.formControl('fit')._fire('change', H.formControl('fit'));
await tick();
check(lastPreviewModel.fit === 'press', `NF 外配合应进模型：${JSON.stringify(lastPreviewModel)}`);
check(el('items').innerHTML.includes('+0.138'), '压配合 → 公法线上差变化');
// 定心/齿根变体
H.formControl('centering').value = 'outer';
H.formControl('centering')._fire('change', H.formControl('centering'));
await tick();
check(el('items').innerHTML.includes('300') && el('items').innerHTML.includes('外径定心'), '外径定心 → Dee=300');
H.formControl('centering').value = 'flank';
H.formControl('root').value = 'fillet';
H.formControl('root')._fire('change', H.formControl('root'));
await tick();
check(el('items').innerHTML.includes('279.795'), '圆齿根 → Die=279.795');
H.formControl('root').value = 'flat';
H.formControl('root')._fire('change', H.formControl('root'));
await tick();
// NF 外出表（无 at → 待放置；有 at → 直插）
closed = false;
el('ok').click();
await tick();
check(lastExportUrl.startsWith('/api/card_export'), `NF 外导出 URL：${lastExportUrl}`);
check(lastExportModel.card === 'NF外花键参数表' && lastExportModel.at === null, `NF 外导出模型：${JSON.stringify(lastExportModel)}`);
check(closed === true, 'NF 外出表成功后应自动关窗');
closed = false;
el('atX').value = '11'; el('atY').value = '12';
el('ok').click();
await tick();
check(JSON.stringify(lastExportModel.at) === '[11,12]', `NF 外 at 应进模型：${JSON.stringify(lastExportModel.at)}`);
el('atX').value = ''; el('atY').value = '';
// 表外 A=210 → K/W 与公法线公差「—」
H.formControl('a').value = '210';
H.formControl('a')._fire('input', H.formControl('a'));
await tick();
check(el('items').innerHTML.includes('—'), 'NF 外表外 A=210 → 标缺「—」');
H.formControl('a').value = '300';
H.formControl('a')._fire('input', H.formControl('a'));
await tick();
// 表达式 → A/m/z 回填；KIND IN 冲突红框
H.formControl('expr').value = 'SPLINE EX M7.5 Z38 ALPHA20 X0.8 BETA0 H30';
H.formControl('expr')._fire('input', H.formControl('expr'));
await tick();
check(String(lastPreviewModel.expr).startsWith('SPLINE EX M7.5'), `NF 外表达式应进模型：${JSON.stringify(lastPreviewModel.expr)}`);
check(H.formControl('a').value === '300' && H.formControl('m').value === '7.5' && H.formControl('z').value === '38',
  `NF 外表达式应回填 A/m/z：A=${H.formControl('a').value} m=${H.formControl('m').value} z=${H.formControl('z').value}`);
H.formControl('expr').value = 'SPLINE IN M7.5 Z38 ALPHA20 X0.8 BETA0 H30';
H.formControl('expr')._fire('input', H.formControl('expr'));
await tick();
check((el('status').textContent || '').includes('KIND'), `NF 外 KIND 冲突应可见：${el('status').textContent}`);
check(el('status').className === 'bad', `NF 外 KIND 错误应红框：${el('status').className}`);
H.formControl('expr').value = '';
H.formControl('expr')._fire('input', H.formControl('expr'));
await tick();

// ── ⑥ DIN 5480 内/外两张单栏卡：10 字段 + 13 项 + 缺口 + 一卡一方向 ──
el('cardType').value = 'DIN花键参数表';
el('cardType')._fire('change', el('cardType'));
await tick();
check(H.card.id === 'DIN花键参数表', `DIN 内卡：${H.card.id}`);
check(el('cardHint').title.includes('Table 7'), `DIN 缺项说明应进 title：${el('cardHint').title}`);
let dinSeq = el('formParams').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id);
check(dinSeq.join(',') === 'f_m,f_z,f_d_b,f_hub,f_e2,f_ae,f_as_,f_tact_n,f_teff_n', `DIN 内字段顺序：${dinSeq}`);
check(el('formMain').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id).join(',') === 'f_expr',
  'DIN 表达式应在中部主输入区');
check(H.formControl('m').value === '3' && H.formControl('z').value === '38' && H.formControl('d_b').value === '120', 'DIN 默认示例 M3/Z38/B120');
check(H.formControl('hub').value === '9H' && H.formControl('shaft') === undefined, 'DIN 内卡只有孔配合 9H');
await H.refresh();
check(lastPreviewModel && lastPreviewModel.card === 'DIN花键参数表', `DIN 预览模型：${JSON.stringify(lastPreviewModel)}`);
check(!('side' in lastPreviewModel), 'DIN 模型不应带 side（由卡类型决定）');
check(Number(lastPreviewModel.m) === 3 && Number(lastPreviewModel.z) === 38 && Number(lastPreviewModel.d_b) === 120, `DIN m/z/d_B 应进模型：${JSON.stringify(lastPreviewModel)}`);
check(lastPreviewModel.hub === '9H', `DIN 孔配合应进模型：${JSON.stringify(lastPreviewModel)}`);
check(el('items').innerHTML.split('class="row"').length - 1 === 13, `DIN 内卡应 13 项：${el('items').innerHTML.slice(0, 100)}`);
check(el('items').innerHTML.includes('Nabe DIN 5480') && el('items').innerHTML.includes('N120×3×38×9H'), 'DIN 内标记行应拆成标签格/值格');
check(el('items').innerHTML.includes('6.361') && el('items').innerHTML.includes('6.271'), 'DIN 内锚点 e 应出现在预览里');
H.formControl('ae').value = '0';
H.formControl('ae')._fire('input', H.formControl('ae'));
H.formControl('as_').value = '-0.028';
H.formControl('as_')._fire('input', H.formControl('as_'));
await tick();
check(Number(lastPreviewModel.ae) === 0 && Number(lastPreviewModel.as_) < 0, `DIN 覆盖 ae/as 应进模型：${JSON.stringify(lastPreviewModel)}`);
H.formControl('ae').value = '';
H.formControl('ae')._fire('input', H.formControl('ae'));
await tick();
// DIN 内卡出表（无 at → 待放置；有 at → 直插）
closed = false;
el('ok').click();
await tick();
check(lastExportUrl.startsWith('/api/card_export'), `DIN 导出 URL：${lastExportUrl}`);
check(lastExportModel.card === 'DIN花键参数表' && lastExportModel.at === null, `DIN 导出模型：${JSON.stringify(lastExportModel)}`);
check(closed === true, 'DIN 出表成功后应自动关窗');
closed = false;
el('atX').value = '9'; el('atY').value = '10';
el('ok').click();
await tick();
check(JSON.stringify(lastExportModel.at) === '[9,10]', `DIN at 应进模型：${JSON.stringify(lastExportModel.at)}`);
el('atX').value = ''; el('atY').value = '';
// DIN 缺口路径：m=5 → 公差「—」
H.formControl('m').value = '5'; H.formControl('z').value = '16'; H.formControl('d_b').value = '80';
H.formControl('m')._fire('input', H.formControl('m'));
await tick();
check(el('items').innerHTML.includes('—'), 'DIN 模数组缺口应显示「—」');
H.formControl('m').value = '3'; H.formControl('z').value = '38'; H.formControl('d_b').value = '120';
H.formControl('m')._fire('input', H.formControl('m'));
await tick();
// ★ DIN 内卡表达式 → d_B=m(z+1.1+2x)/m/z 回填
H.formControl('expr').value = 'SPLINE IN M4 Z30 ALPHA30 X0.2 BETA0 H30';
H.formControl('expr')._fire('input', H.formControl('expr'));
await tick();
check(String(lastPreviewModel.expr).startsWith('SPLINE IN M4 Z30'), `DIN 表达式应进模型：${JSON.stringify(lastPreviewModel.expr)}`);
check(H.formControl('m').value === '4' && H.formControl('z').value === '30' && H.formControl('d_b').value === '126',
  `DIN 表达式应回填 m=4/z=30/d_B=126：m=${H.formControl('m').value} z=${H.formControl('z').value} d_B=${H.formControl('d_b').value}`);
// ★ 切到 DIN 外卡：只留 Welle 字段与 13 项；EX 表达式 OK、IN 报 KIND
el('cardType').value = 'DIN花键参数表_外';
el('cardType')._fire('change', el('cardType'));
await tick();
check(H.card.id === 'DIN花键参数表_外', `DIN 外卡：${H.card.id}`);
dinSeq = el('formParams').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id);
check(dinSeq.join(',') === 'f_m,f_z,f_d_b,f_shaft,f_e2,f_ae,f_as_,f_tact_w,f_teff_w', `DIN 外字段顺序：${dinSeq}`);
check(H.formControl('shaft').value === '8f' && H.formControl('hub') === undefined, 'DIN 外卡只有轴配合 8f');
H.formControl('expr').value = 'SPLINE EX M3 Z38 ALPHA30 X0.45 DA119.4 DF113.4 BETA0 H30';
H.formControl('expr')._fire('input', H.formControl('expr'));
await tick();
check(el('items').innerHTML.split('class="row"').length - 1 === 13, `DIN 外卡应 13 项：${el('items').innerHTML.slice(0, 100)}`);
check(el('items').innerHTML.includes('Welle DIN 5480') && el('items').innerHTML.includes('W120×3×38×8f'), 'DIN 外标记行应拆成标签格/值格');
check(el('items').innerHTML.includes('6.243'), 'DIN 外锚点 s 应出现在预览里');
H.formControl('expr').value = 'SPLINE IN M3 Z38 ALPHA30 X0.45 DA120 DF114 BETA0 H30';
H.formControl('expr')._fire('input', H.formControl('expr'));
await tick();
check((el('status').textContent || '').includes('KIND'), `DIN 外卡 IN 表达式应报 KIND：${el('status').textContent}`);
check(el('status').className === 'bad', `DIN 方向错误应红框：${el('status').className}`);
H.formControl('expr').value = '';
H.formControl('expr')._fire('input', H.formControl('expr'));
await tick();

// ── ⑨ GB 精简版卡片（新增）：9 项、无公差列、统一骨架、表达式反解、出表/计算书同路径 ──
el('cardType').value = 'GB花键精简表_内';
el('cardType')._fire('change', el('cardType'));
await tick();
check(H.card.id === 'GB花键精简表_内', `精简内卡：${H.card.id}`);
check(H.panelOf(H.card) === 'form', '精简卡应走通用骨架（不另写一套）');
check((el('cardHint').title || '').includes('不含公差') || (el('cardHint').title || '').includes('公差'),
  `精简缺项说明进 title：${el('cardHint').title}`);
let liteSeq = el('formParams').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id);
check(liteSeq.join(',') === 'f_grade,f_fit,f_root,f_dp', `精简内字段顺序：${liteSeq}`);
check(el('formMain').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id).join(',') === 'f_expr',
  '精简表达式应在中部主输入区');
check(H.formControl('grade').value === '6' && H.formControl('fit').value === 'H', '精简内默认 6/H');
await H.refresh();
check(lastPreviewModel && lastPreviewModel.card === 'GB花键精简表_内', `精简预览模型：${JSON.stringify(lastPreviewModel)}`);
check(el('items').innerHTML.split('class="row"').length - 1 === 9, `精简内应 9 项：${el('items').innerHTML.slice(0, 100)}`);
check(!el('items').innerHTML.includes('公差') && !el('items').innerHTML.includes('偏差'),
  '精简卡结果预览不得出现公差/偏差字段');
check(el('items').innerHTML.includes('量棒直径 Dp') && el('items').innerHTML.includes('测量跨棒距 Md'),
  '精简内应含量棒/跨棒距');
// 表达式 → 反解值出现（统一解析器；后端 fields 回填）
H.formControl('expr').value = 'SPLINE IN M0.5 Z30 ALPHA30 X0 DA16 DF13.75 BETA0 H20';
H.formControl('expr')._fire('input', H.formControl('expr'));
await tick();
check(String(lastPreviewModel.expr).startsWith('SPLINE IN M0.5'), `精简表达式应进模型：${lastPreviewModel.expr}`);
check(el('items').innerHTML.includes('0.5') && el('items').innerHTML.includes('30'), '表达式反解值应出现在预览');
// 切精简外卡：字段少一个 dp；9 项含 Kn/Wn；仍无公差
el('cardType').value = 'GB花键精简表_外';
el('cardType')._fire('change', el('cardType'));
await tick();
check(H.card.id === 'GB花键精简表_外', `精简外卡：${H.card.id}`);
liteSeq = el('formParams').children.filter((c) => c.id && c.id.startsWith('f_')).map((c) => c.id);
check(liteSeq.join(',') === 'f_grade,f_fit,f_root', `精简外字段顺序：${liteSeq}`);
check(H.formControl('dp') === undefined, '精简外卡不含量棒字段');
await H.refresh();
check(el('items').innerHTML.split('class="row"').length - 1 === 9, `精简外应 9 项：${el('items').innerHTML.slice(0, 100)}`);
check(el('items').innerHTML.includes('跨测齿数 Kn') && el('items').innerHTML.includes('公法线长度 Wn'),
  '精简外应含 Kn/Wn');
check(!el('items').innerHTML.includes('公差') && !el('items').innerHTML.includes('偏差'),
  '精简外预览不得出现公差/偏差');
// 出表走统一 /api/card_export；计算书同端点同模型
closed = false;
el('ok').click();
await tick();
check(lastExportUrl.startsWith('/api/card_export') && lastExportModel.card === 'GB花键精简表_外',
  `精简外出表：${lastExportUrl} ${JSON.stringify(lastExportModel)}`);
closed = false;
el('report').click();
await tick();
check(lastReportModel && lastReportModel.card === 'GB花键精简表_外', `精简计算书同源模型：${JSON.stringify(lastReportModel)}`);
el('reportClose').click();

// ── ⑩ 精简版扩体系（NF/DIN/ANSI×4/齿轮）：共享 card_lite 引擎 + 统一骨架 ──
el('cardType').value = 'NF花键精简表_内';
el('cardType')._fire('change', el('cardType'));
await tick();
check(H.card.id === 'NF花键精简表_内', `NF 精简内卡：${H.card.id}`);
check(H.panelOf(H.card) === 'form', '精简版扩体系也走统一骨架');
check(H.formControl('expr') != null && H.formControl('a') != null, 'NF 精简卡沿用完整卡字段清单');
await H.refresh();
check(lastPreviewModel && lastPreviewModel.card === 'NF花键精简表_内', `NF 精简预览模型：${JSON.stringify(lastPreviewModel)}`);
check(el('items').innerHTML.split('class="row"').length - 1 === 11, `NF 精简应 11 项：${el('items').innerHTML.slice(0, 100)}`);
check(!el('items').innerHTML.includes('公差') && !el('items').innerHTML.includes('偏差'),
  'NF 精简结果不得出现公差/偏差');
check(el('items').innerHTML.includes('量棒直径 V') && el('items').innerHTML.includes('跨棒距 G'),
  'NF 精简应含 V/G');
// 齿轮精简：三圆 + 公法线/跨齿数（9 项）
el('cardType').value = '齿轮精简表';
el('cardType')._fire('change', el('cardType'));
await tick();
check(H.card.id === '齿轮精简表', `齿轮精简卡：${H.card.id}`);
await H.refresh();
check(el('items').innerHTML.split('class="row"').length - 1 === 9, `齿轮精简应 9 项：${el('items').innerHTML.slice(0, 100)}`);
check(el('items').innerHTML.includes('分度圆 d') && el('items').innerHTML.includes('公法线 W'),
  '齿轮精简应含三圆/公法线');
check(!el('items').innerHTML.includes('公差') && !el('items').innerHTML.includes('偏差'),
  '齿轮精简结果不得出现公差/偏差');
closed = false;
el('ok').click();
await tick();
check(lastExportUrl.startsWith('/api/card_export') && lastExportModel.card === '齿轮精简表',
  `齿轮精简出表：${lastExportUrl} ${JSON.stringify(lastExportModel)}`);

// ── ⑦ 预览 404 → 红框可见（共享助手；不关窗）─────────────────────
forcePreview404 = true;
closed = false;
await H.refresh();
check((el('status').textContent || '').includes('404'), `预览 404 提示：${el('status').textContent}`);
check(el('status').className === 'bad' && String(el('status').style.background).toLowerCase() === '#fdecea',
  `404 应红框：class=${el('status').className} bg=${el('status').style.background}`);
forcePreview404 = false;

// ── ⑧ 信息分层负断言：常显区不包含口径/来源，也不得出现已删的顶部说明 ──
const visibleText = html
  .replace(/<script[\s\S]*?<\/script>/g, ' ')
  .replace(/<style[\s\S]*?<\/style>/g, ' ')
  .replace(/<[^>]+>/g, ' ');
for (const bad of ['Table 4/5', '公式', '来源：', '未臆造']) {
  check(!visibleText.includes(bad), `常显区不应含口径/来源「${bad}」`);
}
// ① 顶部常显说明（用户红框）：“本期六张卡……”整句不得出现在可见文本
check(!visibleText.includes('本期六张卡'), '顶部不应再出现「本期六张卡」常显说明');
check(!visibleText.includes('GB 花键 · 齿轮 · ANSI 花键'), '顶部不应再出现卡清单一览');

function report() {
  if (errors.length) {
    realError('智能卡片冒烟失败：');
    for (const e of errors) realError(' - ' + e);
    process.exit(1);
  }
  console.log('智能卡片冒烟通过');
}
report();
