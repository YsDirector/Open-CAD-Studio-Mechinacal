// 冒烟脚本专用：把页面 / 公共 JS 里的 i18n 占位符渲染成中文。
//
// 产品路径不存在「中文兜底」——服务端 `guide_server::render_i18n_text` 按当前语言
// 把 `{{i18n:key}}` / `{{i18njs:key}}` 替换掉；本文件只为直接跑 `node tests/*.mjs
// src/*.html` 的冒烟脚本复刻同一渲染（只收断言会碰到的 key，与 `src/i18n.rs`
// 的 zh 列同步；改词条时这里也要跟着改）。
const ZH = {
  // 公共错误（ocsm_gui_common.js；`{...}` 由页面 JS `.replace()` 填）
  "gui.common.err.404":
    "端点 {path} 不存在（HTTP 404）：可能是插件未重启或版本不匹配 —— 请完全退出并重开 OCS",
  "gui.common.err.http": "HTTP {status}：{detail}",
  "gui.common.err.no_body": "无响应体",
  "gui.common.err.not_json": "响应不是 JSON（HTTP {status}）：{text}",
  "gui.common.err.net": "请求失败（网络层）：{err}",
  "gui.common.err.request_failed": "请求失败",
  // 标注配置（guide_gui.html）编辑态文案
  "gui.guide.h1": "标注配置",
  "gui.guide.update": "更新标注",
  "gui.guide.save_new": "另存为新标注",
  "gui.guide.apply": "应用",
  "gui.guide.refresh": "应用并刷新",
};

const KEY_RE = /\{\{i18n(?:js)?:([^}]+)\}\}/g;

export function renderZh(text) {
  return String(text).replace(KEY_RE, (m, key) =>
    Object.prototype.hasOwnProperty.call(ZH, key) ? ZH[key] : m);
}
