// OCSM GUI 共享助手（唯一实现；各页面用 `<script src="/ocsm_gui_common.js"></script>` 引入）。
//
// 由来（2026-09-25 用户实测）：智能卡片把「方向与表达式 KIND 不一致」的后端 400 写进了
// `#status`，但 `#status { color: #555 }`（ID 选择器）盖过了 `.bad { color: #d1242f }`
// （类选择器）→ 报错是**灰色**的，等于没说。本助手把错误提示统一成
// **红框红字**（基准 = hole_gui.html 的 `.hint.bad`：文字 #b3261e、底 #fdecea、边 #f3b7b3），
// 并用**内联样式**兜底 —— 不受各页面 `#status` 自带 color 的影响。
//
// 口径（跨模块共享实现 > 各写一份；hole_gui.html 既有 readApi 是基准）：
//   ① 错误一律走 `ocsmStatus(..., "bad")`：红框红字 + 滚入视野；
//   ② 后端 `{error}` 文案**原样**透出，不改写成含糊的“参数错误”；
//   ③ 非 2xx 先 `text()` 再 `JSON.parse`（404 是纯文本 "not found"，直接 `.json()` 会抛 SyntaxError）；
//   ④ 旧请求的成功响应不得清掉新错误（`statusSeq` 守卫：清状态要带上自己的序号）。
(function (g) {
  "use strict";

  // 与 hole_gui.html `.hint.bad` 同一套配色（唯一基准，别各页另发明）。
  var BAD_FG = "#b3261e";
  var BAD_BG = "#fdecea";
  var BAD_BD = "#f3b7b3";
  var OK_FG = "#1a7f37";
  var WARN_FG = "#b26a00";

  var statusSeq = 0;

  function statusNode() {
    return g.document && g.document.getElementById ? g.document.getElementById("status") : null;
  }

  // 当前状态序号：发起请求前记下，成功回来用它清（只清自己那一次）。
  function ocsmSeq() { return statusSeq; }

  // 写动态提示（该出现时才出现；不常显）。kind: "bad"（默认醒目红框）/ "ok" / "warn" / ""。
  // 返回本次序号，交给 `ocsmClearStatus(seq)` 防串台。
  function ocsmStatus(text, kind) {
    statusSeq += 1;
    var node = statusNode();
    if (!node) return statusSeq;
    node.className = kind || "";
    node.textContent = text == null ? "" : String(text);
    var st = node.style;
    if (st) {
      if (kind === "bad") {
        st.color = BAD_FG;
        st.background = BAD_BG;
        st.border = "1px solid " + BAD_BD;
        st.borderRadius = "4px";
        st.padding = "4px 6px";
      } else {
        st.color = kind === "ok" ? OK_FG : (kind === "warn" ? WARN_FG : "");
        st.background = "";
        st.border = "";
        st.borderRadius = "";
        st.padding = "";
      }
    }
    if (kind === "bad" && typeof node.scrollIntoView === "function") {
      try { node.scrollIntoView({ block: "nearest" }); } catch (e) { /* 老浏览器忽略 */ }
    }
    return statusSeq;
  }

  function ocsmStatusError(text) { return ocsmStatus(text, "bad"); }
  function ocsmStatusOk(text) { return ocsmStatus(text, "ok"); }

  // 清状态：省略 seq = 无条件清；给了 seq = 只有“最新一次写入”还是它才清。
  function ocsmClearStatus(seq) {
    if (seq !== undefined && seq !== statusSeq) return;
    var node = statusNode();
    if (!node) return;
    node.className = "";
    node.textContent = "";
    var st = node.style;
    if (st) {
      st.color = "";
      st.background = "";
      st.border = "";
      st.borderRadius = "";
      st.padding = "";
    }
  }

  function pageLabel() {
    var t = g.document && g.document.title;
    return t ? "[" + t + "]" : "[OCSM]";
  }

  // 统一读响应：非 2xx 先读文本再解析；返回 {ok,data} / {ok:false,error}。
  // （与 hole_gui 既有 readApi 同判据/同文案；404 专门指路「插件未重启/版本不匹配」。）
  // 文案全部走服务端注入的 catalog（i18njs 占位符），本文件不写中文兜底。
async function readApi(resp, path) {
    var prefix = pageLabel();
    var text = await resp.text();
    var j = null;
    try { j = text ? JSON.parse(text) : null; } catch (e) { j = null; }
    if (!resp.ok) {
      var snip = text.slice(0, 200);
      console.error(prefix + " " + path + " → HTTP " + resp.status + "：" + snip);
      var err = resp.status === 404
        ? "{{i18njs:gui.common.err.404}}".replace("{path}", path)
        : "{{i18njs:gui.common.err.http}}".replace("{status}", String(resp.status))
            .replace("{detail}", (j && (j.error || j.message)) || snip || "{{i18njs:gui.common.err.no_body}}");
      return { ok: false, error: err };
    }
    if (j === null) {
      console.error(prefix + " " + path + " → HTTP " + resp.status + "：not JSON：" + text.slice(0, 200));
      var nerr = "{{i18njs:gui.common.err.not_json}}"
        .replace("{status}", String(resp.status))
        .replace("{text}", text.slice(0, 200));
      return { ok: false, error: nerr };
    }
    if (j.ok === false) return { ok: false, error: j.error || j.message || "{{i18njs:gui.common.err.request_failed}}" };
    return { ok: true, data: j };
  }

  // ★ 取数唯一入口（JSON 端点）：fetch + readApi；任何错误都**直写可见 #status（红框）**，
  // 调用方即使忘了处理返回也不会“只进 console”。
  async function fetchApi(url, opts, path) {
    var resp;
    try {
      resp = await fetch(url, opts);
    } catch (e) {
      var nerr = "{{i18njs:gui.common.err.net}}".replace("{err}", String(e));
      console.error(pageLabel() + " " + path + " → " + nerr);
      ocsmStatusError(nerr);
      return { ok: false, error: nerr };
    }
    var r = await readApi(resp, path);
    if (!r.ok) ocsmStatusError(r.error);
    return r;
  }

  // 文本端点（/api/*_svg 等非 JSON）：只判 ok，成功给 text。
  async function fetchText(url, opts, path) {
    var resp;
    try {
      resp = await fetch(url, opts);
    } catch (e) {
      var nerr = "{{i18njs:gui.common.err.net}}".replace("{err}", String(e));
      console.error(pageLabel() + " " + path + " → " + nerr);
      ocsmStatusError(nerr);
      return { ok: false, error: nerr };
    }
    var text = await resp.text();
    if (!resp.ok) {
      var j = null;
      try { j = text ? JSON.parse(text) : null; } catch (e) { j = null; }
      var snip = text.slice(0, 200);
      console.error(pageLabel() + " " + path + " → HTTP " + resp.status + "：" + snip);
      var err = resp.status === 404
        ? "{{i18njs:gui.common.err.404}}".replace("{path}", path)
        : "{{i18njs:gui.common.err.http}}".replace("{status}", String(resp.status))
            .replace("{detail}", (j && (j.error || j.message)) || snip || "{{i18njs:gui.common.err.no_body}}");
      ocsmStatusError(err);
      return { ok: false, error: err };
    }
    return { ok: true, data: text };
  }

  // 单位后缀（预览通用规则）：值**已自带单位/符号**时不得再追加单位（如 `20°` + `°` → 不再叠成 `20° °`）。
  // 判定：① 值里已含单位串（`20°` 含 `°`、`1.5mm` 含 `mm`）；② 符号类单位（°/%）看结尾符号；
  //       ③ 占位缺项（「—」/?）不加单位。各卡预览共用，不在某一格里打补丁。
  function ocsmUnitSuffix(value, unit) {
    var u = unit == null ? "" : String(unit).trim();
    if (!u) return "";
    var v = value == null ? "" : String(value).trim();
    if (!v || v === "—" || v === "?" || v === "-") return "";
    if (v.indexOf(u) >= 0) return "";
    if (u === "°" && /[°º⁰]$/.test(v)) return "";
    if (u === "%" && /%$/.test(v)) return "";
    if (u === "mm" && /mm$/i.test(v)) return "";
    return u;
  }

  g.ocsmSeq = ocsmSeq;
  g.ocsmStatus = ocsmStatus;
  g.ocsmStatusError = ocsmStatusError;
  g.ocsmStatusOk = ocsmStatusOk;
  g.ocsmClearStatus = ocsmClearStatus;
  g.ocsmUnitSuffix = ocsmUnitSuffix;
  g.readApi = readApi;
  g.fetchApi = fetchApi;
  g.fetchText = fetchText;
})(typeof globalThis !== "undefined" ? globalThis : this);
