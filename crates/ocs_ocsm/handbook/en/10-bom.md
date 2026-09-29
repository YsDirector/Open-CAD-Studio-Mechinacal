# 10 BOM / Parts List (`BOM` / `BOMCFG`)

> English translation of `handbook/10-明细表BOM.md` (the Chinese original is the source of truth).

> Aggregate the XDATA of standard parts into a BOM on the drawing.

## 1. Commands

| Command | Alias | Purpose |
| --- | --- | --- |
| `OCSMBOM` | `BOM` | **build / refresh** the BOM; may take a parameter: `BOM 30` = lay out **30 rows** in the first column this time |
| `OCSMBOMCFG` | `BOMCFG` | **configure** (columns/header/format etc.); also takes the row-count parameter |
| `OCSMBOMEDIT` | `BOMEDIT` | **open the web editor** (a standalone chromium window; see §5) |

- Data source: the **XDATA `OCSM_PART`** on standard-part INSERTs in the drawing (family/spec/standard number/material/weight…)
  → so **do not explode standard parts**, and do not hand-draw bolt outlines (they would not be counted).
- The table body is **the row block inserted repeatedly** (one block instance per row); the header is at the very top of the whole table; rows carry ATTDEF attributes (item no./code/name/qty/material/weight/notes…).

## 2. Placement Rules (measured from the drawing frame; re-measure when the frame changes)

Taking `a3_landscape.dwg` (A3 landscape, **180×45** title block + a vertical side column on the left) as an example:

| Edge | Datum | Measured on A3 landscape |
| --- | --- | --- |
| **Right** | the frame's **inner border right edge** (= title-block right edge) | `x = 390` |
| **Left** | the title block's **left edge** (it does not extend into the left side column) | `x = 210` |
| **Width** | = title-block width (this frame is **180 mm**, note: not 200) | 180 |
| **Bottom** | the title block's **top edge**, flush against it | `y = 45` (first row 45→57) |
| **Row order** | accumulates **bottom-up**: item 1 at the bottom, flush against the title block; the header at the very top | — |
| **Height** | grows upward with the row count (one row occupies 8 mm in the first phase) | — |

General rule: **the right edge aligns with the inner frame's right edge; width = title-block width; the bottom edge hugs the title block's top edge**.

## 3. Item Balloons

- The BOM trio = table + header + **item balloons** (carrying each part's item number onto the assembly drawing).
- The **only coupling point** between balloons and the table **is the item number**: the number is fixed in the table, the balloon points at the matching part; changing a number must be synced on both sides.

## 3.5 Balloon Sync (`BOMSYNC`)

`BOM` "builds the table from the part ledger" (item no. = row no.); once **item balloons** are bound, use **`BOMSYNC`**
to reorder by balloon (it can also run automatically when balloons are created/changed):

- row = each entry of the balloon group + aggregated parts not referenced by any balloon (reusing the old table's item number, otherwise auto-assigned);
- **the same item number referenced by several balloons → qty +1** (total weight follows);
- when ticking "插入序号" (insert item numbers) causes a conflict: the affected existing balloons are all +1 and the table is reordered accordingly;
- row order is by **ascending item number**, bottom-up (item 1 at the bottom).

> In one sentence: **the item number is the only key between balloons and the table**. `BOM` builds the table (ledger view), `BOMSYNC` matches numbers (balloon view).

### Manual-Edit Protection (settled by the user 2026-09-15)

Sync has **replace semantics** (delete the old rows and rebuild), so manual edits must be protected or they would be overwritten by computed values:

| Column | Rule |
| --- | --- |
| **Item no.** | **never locked**: governed by the numbering/balloon rules, sync always recomputes it |
| **Quantity** | **a switchable lock**: unlocked → computed by the rules (part count / balloon references); locked → **the locked value is always used** |
| Drawing no. / name / material / unit weight / notes | **kept once non-empty** (an "unbreakable lock"): once a row exists, non-empty fields are never overwritten; only **blank cells** receive new values |

- **Lock command**: `BOMLOCK <item no.|drawing no.> [qty|off]`
  * `BOMLOCK 5` → lock at the current on-row value (I edited it by hand, do not overwrite)
  * `BOMLOCK 5 3` → set the quantity to 3 and lock it (`总重` (total weight) is automatically = unit weight × 3)
  * `BOMLOCK 5 off` → unlock; the next sync recomputes
- **The other way to reset a lock**: **delete** that row; the next sync rebuilds it by the rules (only then does the "value passed at creation" take effect).
- Total weight is **computed** (unit weight × quantity); it always follows the quantity and does not count as a manual edit.
- Locked rows appear in the sync report: `N 行数量已锁定，未覆盖：…` (N rows have locked quantities and were not overwritten: …) — never silent.
- The xlsx side is in the next section (**implemented**).

## 3.5.5 Automatic Cell-Text Compression (column-width fitting)

BOM column widths are **fixed by the template** (the row block's vertical separators = 0/11/48/81/92/127/138/150/180),
and CAD text **does not reflow** — long content spills into the neighbouring cell. So cells are written with automatic compression to the column width:

| Step | How |
| --- | --- |
| ① measure | measured against the **actual template** (font `OCSM_GB`, text height 5.0, width factor 0.7): CJK **1.15 em**, Latin/digits **0.8 em** (calibrated from on-drawing pixel measurements, conservative values), multiplied by the actual width factor |
| ② compress width only | overlong text compresses **only the width factor**; the text height **does not move (settled by the user 2026-09-16: one text height for the whole table)**; target width = available width × **0.9** (available width = column width − 2.5) and the width factor has **no lower bound** → it always fits the cell and never reaches the next one |
| ③ warn | compressing to **< nominal × 0.7** counts as "too flat": the command line calls it out: `OCSMBOM: … 横向压得偏扁：名称「…」41%（建议加宽该列 / 缩短文本）` (squeezed too flat horizontally: name "…" 41% — consider widening the column / shortening the text) |

Actual cases (template text height 5.0, **uniform for the whole table, never compressed**; only the width factor is computed):

| Cell | Content | Column width | Result (from width factor 0.7 →) |
| --- | --- | --- | --- |
| Item no. | `1` | 11.0 | unchanged |
| Drawing no. | `GB/T 5780-2016` | 37.0 | slightly compressed |
| Name | `六角头螺栓 C级 M8x35` | 33.0 | **0.41 (too flat, called out)** |
| Quantity | `1` | 11.0 | unchanged |
| Unit weight | `0.018` | 11.1 | compressed (too flat, called out) |
| Total weight | `0.018` | 11.9 | compressed (called out) |

- Column widths are read from the **row-block geometry**, not hard-coded — change the template (widen the drawing-no. column, etc.) and it follows automatically.
- Only **each row's attribute entities** (the ATTRIBs inside the INSERT) are touched; **the block definition is not changed** — every row is independent.
- To verify the parsing: `cargo test -p ocs_ocsm --lib dump_row_template_widths -- --ignored --nocapture`
  (reads `bom/OCSM_BOMROW.dwg`, prints the column widths + compression results for common content).

## 3.6 External `.xlsx` Collaboration (`BOMXLSX` / `BOMXLSXI`)

```
BOMXLSX                    → export to "<drawing name>-明细表.xlsx" (same directory), and link it from the header (Ctrl+click to open)
BOMXLSX D:/表.xlsx          → explicit path (a missing extension is filled in as .xlsx; the path keeps its original case)
BOMXLSX D:/表.csv          → with `.csv` it exports CSV (UTF-8 with BOM; opens in Excel/WPS/text editors)
BOMXLSXI                   → import from the file linked in the header (no link → default path)
BOMXLSXI D:/表.xlsx         → explicit file (`.csv` is also accepted)
```

**Save first, then draw item numbers** (important): by default the xlsx goes into the **drawing's own directory**, so a new drawing that has never been saved falls back to the default directory
(the **system temporary directory** `/tmp/OCSM/明细表.xlsx`; `%TEMP%\OCSM\` on Windows — via `std::env::temp_dir()`, one cross-platform code path). The plugin warns in three places:

* **before** drawing item numbers: an orange notice line on the "序号" (item number) panel of the GDIM window;
* **when they are placed**: the command line says "本图还没存过盘" (this drawing has never been saved) (once per tab, does not block the main flow);
* when `BOMXLSX` exports: it states where the file goes and how to change that.

**Column order** (the first 8 columns = the on-drawing BOM itself; column 9 exists only in this file and **does not go into the drawing table**):

```
序号 | 图号 | 名称 | 数量 | 材料 | 单重 | 总重 | 备注 | 锁定数量
```

(item no. | drawing no. | name | qty | material | unit weight | total weight | notes | locked qty)

*"锁定数量" (locked qty): empty = not locked; `Y`/`是`/`1`/`√` = lock to the **quantity on the same row**; a number N = lock to N.

**Import rules**:

| Case | Behaviour |
| --- | --- |
| file cell **non-empty** | overwrite the drawing (the file is the explicit editing channel) |
| file cell **blank** | keep the drawing's current value (existing content is not erased) |
| an item number **absent** from the file | keep that row in the drawing (import only changes, never deletes) |
| **Total weight** column | input ignored (= unit weight × quantity, computed) |
| quantity ≠ the value at the last export (baseline) | treated as **manually edited** → **locked automatically**, not overwritten by later syncs |

- On export, each row's current quantity is recorded in the row-block XDATA (`OCSM_BOMXEXP`) — that is the comparison baseline for "edited ⇒ locked";
  after a successful import the baseline refreshes automatically, so importing the same file repeatedly does not lock again.
- **Existing rows are never deleted automatically by sync** (not even when a balloon is removed or a part is deleted): to make a row disappear, **delete it by hand**.
- The xlsx written out is a standard file (zip + SpreadsheetML, inlineStr); Excel / WPS / LibreOffice can all open it;
  reading also understands their rewritten sharedStrings / numeric cells / skipped cells.
- The plugin **does not depend** on LibreOffice/Excel/Python: reading and writing are both done in the plugin (`flate2` + `crc32fast` + `quick-xml`).

## 4. AI-Side Recipe

```jsonc
// 1) first see which standard parts exist (is there anything to count)
{ "op": "read", "op": "query", "parameters": { "type": "Insert" } }
// 2) build/refresh the table (lay out 20 rows first if needed)
{ "op": "execute", "request": { "op": "run", "request_id": "b1", "document_id": 1, "revision": 60, "cmd": "BOM 20" } }
// 3) configure (header/columns)
{ "op": "execute", "request": { "op": "run", "request_id": "b2", "document_id": 1, "revision": 61, "cmd": "BOMCFG 20" } }
// 4) verify: the table's row-block INSERTs + each row's ATTDEF values (do quantities/weights add up)
{ "op": "read", "op": "query", "parameters": { "type": "Insert", "layer": "6文字层" } }
```

## 5. Web Editor (2026-09-16, iteration 4)

Two ways to open it (added 2026-09-16):

- **`OCSMBOMEDIT` / `BOMEDIT`** (command, recommended) → opens a **standalone chromium `--app` window** directly (as clean as the annotation configuration window);
  the window URL carries `tab=<drawing id>` → multiple drawings can each keep their own editor window without mixing tables;
- **Ctrl+click the header/row block** → the host only opens it with `xdg-open`, landing in a **browser tab with an address bar**; when the plugin sees such an
  "external open" it **adds an app window on top**, and replaces that tab with an "already opened in a new window" notice page (which can just be closed).
  Without an app window it adds exactly one; if one is already open (heartbeat) it does not open duplicates.

The BOM editor page (the drawing = the single source of truth):

- **read**: `GET /api/bom_get` — scans the row blocks (ascending item no.) + lock/baseline XDATA + unsaved state + missing-ledger count.
- **write**: edit freely in the page (add row = add part by hand, delete row, edit cell, lock), then click "应用到图纸" (apply to drawing) to rebuild the whole table in one go (`fill_bom` replace semantics,
  one Ctrl+Z); the submit carries a **fingerprint** (current rows + drawing path), so if the drawing was changed by balloon sync etc. after the page was opened → **409 rejected** and the latest state is returned, preventing a stale view from covering a new drawing.
- **renumbering (reorder style)**: tick "改序号撞号时后续自动顺延" (when renumbering, later colliding rows shift automatically); a colliding row is +1 automatically (keeping prefix/width, A09→A10);
  on apply the backend maps identity by "drawing no. + name + material" and **renumbers the balloons in sync** (reusing the balloon GUI rebuild mechanism; unmatched rows are left alone with a warning).
- **export**: `POST /api/bom_export?fmt=xlsx|csv&name=…` → bytes over JSON → the browser's "save as" (default "<drawing name>-明细表.xlsx");
  the **export baseline** is recorded at the same time (row-block `OCSM_BOMXEXP`).
- **import**: the page's file picker reads xlsx/csv bytes → `POST /api/bom_import` → reuses `BOMXLSXI`'s merge rules
  (non-empty overwrites / blanks kept / missing item nos. kept / **edited ⇒ locked**).
- The truth about locks always lives in the drawing's row-block `OCSM_BOMLOCK`; the "锁定数量" column in xlsx is only the import/export representation.
- PE_URLs in old drawings that point at an xlsx file path (the host's `web_hyperlink` only allows http/https, so they are dead links that can never open)
  are rewritten automatically into bom.html links during port refresh / table build; the last export path is stored in `OCSM_BOMXPATH` (the default path for `BOMXLSXI`).

## 6. Pitfalls

1. **No standard parts, no content**: hand-drawn bolts do not enter the table (no `OCSM_PART` record).
2. **The width is not 200**: this frame's title block is **180** wide — laying out at 200 would exceed the inner frame (the numbers in `bom/README.md` are authoritative; this page is synced with the code: table width 180, left x=210, right x=390, bottom y=45).
3. **Not enough rows**: `BOM 30` only means "lay out 30 rows in the first column this time"; the actual row count is decided by the number of part kinds in the drawing; re-running refreshes it.
4. **Weight**: standard-part weight is estimated as volume × steel 7.85 g/cm³ (see `03-standard-parts.md`); check non-steel materials yourself.
5. **Changing the frame**: the placement datums change (inner-frame right / title-block width / title-block top edge) and the table must be repositioned.
6. **Opens empty and needs a refresh click?** That was the page's opening script throwing (`add` in `addEventListener('click', add)`
   was undefined → the later `reload()` never ran). Fixed on 2026-09-16, with a static-check test to catch this class of bug.

## 7. Verification Checklist

- [ ] Table placement: right edge flush with the inner frame's right edge, bottom edge flush with the title block's top edge, width = title-block width.
- [ ] Each row's item no./code/name/quantity matches the part on the assembly drawing.
- [ ] The header is at the very top of the table and item 1 is at the bottom.
