# 06 Leader Annotation (`GDIM` / `OCSMDIMGULIDE`)

> English translation of `handbook/06-引导线标注.md` (the Chinese original is the source of truth).

> OCSM's annotation backbone: **first draw "intent geometry" on `10引导线层`, then let the command generate GB annotation from it**.
> Dimensions, section views, auxiliary views, detail views, angles, arc lengths, welds, leaders, geometric tolerances and roughness all go this way.

## 1. Flow

```
1) draw the leader line (on 10引导线层; linetype/colour do not matter, the layer is off by default and does not print)
   different types need different geometry (see the table below)
2) select that leader line
3) GDIM            (= OCSMDIMGULIDE) → opens the "标注配置" (annotation settings) window
4) in the window: pick type/style/text/alignment parameters → look at the preview
   "应用"        = only writes the parameters back onto the leader line (the leader carries the hyperlink URL)
   "应用并刷新"  = generates the real annotation entities (7标注层 / symbols go to 8符号标注层)
5) to change it: select the annotation → ME (= OCSMEDIT) → change parameters in the same window → apply & refresh (deletes the old and rebuilds)
```

- Nothing selected → `OCSM: 请先选中一条引导线（10引导线层 的直线或两段多段线）。`
  (OCSM: select a leader line first (a line or two-segment polyline on `10引导线层`).)
- Leader shape not supported → `OCSM: 引导线必须是直线（LINE）、多段线（PLINE，2 顶点以上）、圆（CIRCLE）、圆弧（ARC）或矩形（RECTANG）。`
  (OCSM: the leader must be a line (`LINE`), a polyline (`PLINE`, 2 or more vertices), a circle (`CIRCLE`), an arc (`ARC`) or a rectangle (`RECTANG`).)

## 2. Leader-Geometry Contract (**what that line must be drawn as**)

| Type (TYPE) | Leader geometry | Semantics / parameters |
| --- | --- | --- |
| `LINEAR` linear/aligned | 1 **LINE** | the two endpoints = the dimension points; parameters: alignment `A/H/V` + offset |
| `RADIAL` radius | 1 **LINE** | start = circle centre, end = radial direction; parameters: offset + text |
| `DIAMETER` diameter | 1 **LINE** | start = circle centre, end = radial direction (may cross over to the opposite side) |
| `ARCLEN` arc length | 1 **ARC (the arc itself)** | the arc supplies centre/radius/start-end angles by itself → arc length and included angle are self-sufficient |
| `ANGLE` angle | 1 **two-segment PLINE** | the bend point = the angle's vertex, the two segments = the two sides (can dimension a minor or a reflex angle) |
| `SECTION` section | 1 **PLINE (may be a polyline)** | the cutting path; supports revolved/stepped sections; parameters: letter + placement coordinates of the section view |
| `DETAIL` detail view | **CIRCLE / rectangle / PLINE** | the circled region; parameters: leader rotation angle + placement coordinates of the detail view + magnification + letter |
| `VIEW` auxiliary view | 1 **LINE** | the arrow direction line; parameters: letter + direction |
| `WELD` weld | 1 **two-segment PLINE (3 vertices)** | vertex 1 = weld arrow point, vertex 2 = bend point, vertex 3 = end of the datum line (snaps to the four main axes) |
| `LEADER` leader | 1 **two-segment PLINE (3 vertices)** | vertex 1 = arrow point, 2 = bend point, 3 = end of the shoulder line |
| `BALLOON` item number | 1 **two-segment PLINE (3 vertices)** | vertex 1 = pointer point (draws the dot), 2 = bend point, 3 = end of the shoulder line (**must be horizontal**); parameters: item-number list + extension direction (horizontal/vertical) + insert-on-conflict switch; one group may hold several item numbers (a horizontal row joined by a V-shaped polyline / a vertical column joined by a vertical line). See `14-item-numbers.md` |
| `RGH` roughness | block reference (INSERT) | the roughness symbol library; parameters: shape + `Ra` value |
| `GD&T` geometric tolerance | 1 **PLINE** | position of the tolerance frame; parameters: symbol + tolerance value + datum (URL query JSON) |
| `DATUM` datum | 1 piece of leader geometry | datum symbol (the two drawing conventions, 1996/2008) |
| `TOLERANCE` dimensional tolerance | attaches to a dimension annotation | code mode (`H7`) or limit-deviation mode (`+0.021/0`), fit table ISO 286 |

> One-line mnemonic: **the two endpoints of a line are the start of the extension lines; a two-segment polyline is a leader-type symbol "arrow → bend → shoulder"**.

## 3. What Is Generated and Onto Which Layer

| Output | Lands on | Notes |
| --- | --- | --- |
| dimension annotation (linear/radius/diameter/arc length/angle) | `7标注层` | uses the `OCSM_GB` (or `OCSM_GB_xN`) dimension style |
| symbol type (roughness/weld/leader/geometric tolerance/datum/auxiliary view/section letter/detail view) | `8符号标注层` | mostly an **anonymous block** `*X{n}` + INSERT |
| the leader line itself | `10引导线层` | kept (carries the URL) and can be "apply & refresh"-ed over and over; this layer does not print |

## 4. AI-Side Recipe

```jsonc
// 1) draw the leader line (line type, two points in model space)
{ "op": "run", "request_id": "g1", "document_id": 1, "revision": 20, "cmd": "10" }        // make the leader layer current
{ "op": "run", "request_id": "g2", "document_id": 1, "revision": 20, "cmd": "LINE 0,0 45,0" }
// 2) select it (by handle)
{ "op": "read", "op": "query", "parameters": { "type": "Line", "layer": "10引导线层" } }   // get the handle
{ "op": "run", "request_id": "g3", "document_id": 1, "revision": 21, "cmd": "SELECT 2A" } // or let the host set the selection set
// 3) open the settings window (a human fixes type/text/offset in the window → apply & refresh)
{ "op": "run", "request_id": "g4", "document_id": 1, "revision": 21, "cmd": "GDIM" }
// 4) verify: the annotation entity + the hyperlink (the URL on the leader line)
{ "op": "read", "op": "query", "parameters": { "type": "Dimension" } }
```

- HTTP layer (the interface the window actually uses): `GET /guide.html?handle=<hex handle>&tab=<drawing id>` (the settings page, `tab=` = which drawing this window is pinned to), `GET /api/guide?handle=…[&tab=N]` (read the record),
  `POST /api/apply` (writes the hyperlink only), `POST /api/apply_refresh` (generates/refreshes the annotation); both POSTs may carry `"tab":N` (that is how the page sends them).
- **AI/script direct connection (the host routing has been fixed since 2026-09-15: the plugin's HTTP looks at "the currently active drawing")**:
  `GET /api/mcp` sends `{"method":"get_guide","params":{"handle":"6F"}}` to read geometry/parameters;
  `POST /api/apply_refresh` `{"handle":"6F","url":"http://127.0.0.1:23751/DIM/LEADER/0?lu=…"}` to generate.
  Note that `list_guides` **only lists leader lines whose parameters have been configured (whose `PE_URL` carries `/DIM/`)** — a bare leader line that was just drawn is not among them —
  so the AI routine is: `ocs_read op:"query"` (`10引导线层`) to get the handle → `get_guide` → `apply_refresh`.
- If an AI wants "fully automatic annotation": it can write the leader line itself + directly `POST /api/apply_refresh` with the parameter JSON (equivalent to a human clicking in the window).
- **Weld process-code (GB/T 5185) quick dropdown (2026-09-16)**: at the bottom of the weld panel, under the tail note, the "工艺代号" (process code) dropdown lists
  every code in GB/T 5185-2005 (10 groups, 145 items: 1 arc welding / 2 resistance welding / 3 gas welding / 4 pressure welding / 5 high-energy-beam welding / 7 other / 8 cutting & gouging / 9 brazing / old codes);
  an option shows "code name" (e.g. "111 焊条电弧焊" (111 manual metal arc welding)), and after picking one the **tail note contains only the numeric code** (e.g. `111`);
  picking another one appends it after an automatic space (a combined process, e.g. `111 12`), and the tail switch turns on and locks automatically; after picking, the dropdown resets and the tail note can still be edited by hand.
  Preview without restarting: `python3 tools/guide_gui_proxy.py` (port 23752, the HTML comes from the newest repo version, `/api` is passed through to the running plugin).

## 5. Pitfalls

1. **The leader line must be on `10引导线层` first**: `GDIM` only accepts that layer (a line on the wrong layer reports "请先选中一条引导线" (select a leader line first)).
2. **Type and geometry disagree**: e.g. using a 1-segment PLINE as a weld leader (a weld needs 3 vertices / two segments) → it errors out or draws crooked.
3. **Forgetting "apply & refresh" after changing parameters**: "应用" (apply) alone only writes the hyperlink, and the drawing still shows the old annotation (or none at all).
4. **Changes are replace-style**: `ME` + apply & refresh = delete the old annotation and regenerate it from the original leader geometry → **do not edit a generated annotation by hand**,
   the next refresh wipes it.
5. **After swapping the frame for a different scale**: old annotations still use the old style, new ones use the new style. To unify them → use `D2G` or regenerate.
6. **Text moved outwards** (dragging linear-dimension text outside the dimension line): the dimension line is extended along the axis to below the text (the GB/T 4458.4 drawing method, already implemented).
7. **The settings window is "pinned to a drawing" (fixed 2026-09-16)**: the plugin process (and its port) outlives a single drawing, so when a window opens it writes the drawing id into the URL
   (`/guide.html?handle=0x2A&tab=3`), and from then on every read/write is delivered only to **that drawing**.
   The same rule applies to the parts library (`/parts`), bolt joint assembly (`/joint`), surface roughness (`/rough.html`) and the BOM editor (`/bom.html`, command `BOMEDIT`) windows.
   - **`Ctrl+click` can only land on a browser tab that has an address bar** (the host's `sys::open_url` → `xdg-open`, which the plugin cannot intercept):
     when the plugin sees this kind of "external open" it **adds an app window of its own** and replaces that tab with a "已在新窗口打开" (opened in a new window) notice page
     (if the same window is already open it is not opened a second time). So the final effect of Ctrl+click is an independent window as well.
   - **After a drawing is closed, its old window reports on the very first action**: "这个窗口对应的图纸（标签页 N）已经关闭了 —— 请重新运行 GDIM / ME 打开新窗口。"
     (the drawing this window belongs to (tab N) has been closed — run GDIM / ME again to open a new window) (HTTP 409). This is **deliberate**: the old window will no longer (after waiting 5 s)
     write the annotation onto some other drawing.
   - Several drawings can each have their own window open, with no crosstalk.
   - **A request without a drawing id** (a `Ctrl+click` on an annotation link, an AI/MCP direct `/api/guide`, `/api/apply_refresh` etc.)
     follows the **currently active drawing** — the plugin listens to the host's `SelectionChangedV4` / `DocumentTabClosed` notifications,
     so you just switch tabs (no command needed) and it knows which drawing you are looking at.
     One precondition: the plugin can only send requests to a tab that has **run at least one OCSM (or any) command**; when you
     `Ctrl+click` right after opening one it may report "当前活跃图纸还没跑过 OCSM 命令" (the currently active drawing has not run an OCSM command yet) (HTTP 409) — just
     run any command on that drawing (e.g. select an annotation and `ME`) and everything works from then on.

## 6. Verification Checklist

- [ ] The leader line is on `10引导线层` and carries a URL hyperlink (the hyperlink in the `read` record).
- [ ] After "apply & refresh" the annotation/symbol entities appear, on the right layer, with style `OCSM_GB*`.
- [ ] Selecting a generated annotation opens the same window again with `ME` (which means it carries an `OCSM_EDIT` record).
- [ ] Take a screenshot and check the symbol orientation/text position (GB requires: readable from the top, the leader does not cross the text).
