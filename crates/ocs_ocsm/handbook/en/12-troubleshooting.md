# 12 Troubleshooting (Quick Reference)

> English translation of `handbook/12-故障与坑.md` (the Chinese original is the source of truth).

> A command does not respond, a window does not open, a revision error — look here first.

## 1. Command Layer

| Symptom | Cause | Fix |
| --- | --- | --- |
| Plugin commands missing from `op:"commands"` | Plugin not loaded (or an old build is loaded) | Make sure the `.so` under `~/.config/OpenCADStudio/plugins/opencad.ocsm/` is a fresh build; restart OCS |
| Command sent but reports "未知命令" (unknown command) | Command name misspelled / the command has no registered prefix | Check against the `op:"commands"` list; command names are case-insensitive, but **aliases** must be exact (`XL`/`D`/`TF`/`GDIM`/`D2G`/`ME`/`CC`/`BOM`) |
| Command "does nothing" | You are inside an **interactive command** and your input was consumed as a point/argument | Send `op:"cancel"` (=Esc) first, then the command |
| Error `Request payload changed` | The same `request_id` was used twice (with different bodies) | Use a unique `request_id` per request (e.g. append a counter) |
| Error about revision/document mismatch | `document_id`/`revision` is stale | `read state` again, then send |
| Drawing disappears after the plugin restarts | Restarting OCS loses the current document | `save` beforehand; or open the file with the host `open` |

## 2. Window Layer (GUI)

| Symptom | Cause | Fix |
| --- | --- | --- |
| Running `OCSM`/`XL`/`OCSMJOINT`/`GDIM` opens no window | That page's window **heartbeat is still alive** (the page pings every 5 s; with a heartbeat within 15 s no new window is opened) | Close the old window (or wait 15 s); locate it with `kdotool search --name "<title>"` and close it with `kdotool windowclose <id>` |
| Window opens but is blank / reports "无法装配" (cannot assemble) | The parameter combination is not in the library (e.g. the family has no such diameter) | Read the error text on the status line / in the report; switch to a size that exists in the table |
| Clicking "OK/Assemble" in the window does not place anything | The window only **registers** the part (builds a preview block + a pending place item); placing happens in the drawing | Switch back to the drawing: click the base point → move → click again to place (Esc ends) |
| Command name is right but the browser opens a tab instead of an app window | A browser session already exists, so `--app` degrades to a tab | Normal; switch back to the drawing as prompted |
| In development, "the page changed but I still see old content" | Pages are `include_str!`-embedded in the plugin, so you must **rebuild the plugin**; and the old window is still alive (heartbeat) | Rebuild → close all old windows → open again |
| A window reports an error but you "cannot see it" | The backend `{error}` is indeed written to `#status`, but `#status { color:#555 }` (ID selector) beats `.bad` (class selector) → shown as **grey text**, i.e. not shown | Route all errors through the shared helper `ocsmStatus(..., "bad")` in `ocsm_gui_common.js`: **red box, red text** (same `#b3261e`/`#fdecea`/`#f3b7b3` as hole's `.hint.bad`) + scroll into view; **logging only to the console equals not reporting at all** |
| Window reports `SyntaxError: Unexpected token` instead of the backend text | `.json()` is called directly on non-2xx responses (404 is plain text `not found`, so `.json()` always throws) | Use the shared helpers `fetchApi`/`fetchText`: `text()` first, then parse according to `ok`, so the backend `error` text comes through **verbatim** |

## 3. Geometry / Annotation Layer

| Symptom | Cause | Fix |
| --- | --- | --- |
| Dimension text takes the layer colour and has the wrong height | `OCSM` initialization was not run; styles are missing | Run `OCSM`; for old dimensions use `D2G` or regenerate them |
| `GDIM` reports "请先选中一条引导线" (select a leader first) | The leader is not on `10引导线层`, or nothing is selected | Move the line to `10引导线层`; select it first, then run the command |
| Annotation parameters changed but nothing happened | Only "Apply" (writes the hyperlink) was clicked, not "Apply and refresh" | Click "Apply and refresh" again |
| `ME` reports "请先选中一个 OCSM 生成的标注" (select an OCSM-generated annotation first) | That annotation has no `OCSM_EDIT` record (native dimension / standard part) | Convert it with `D2G` first, or regenerate the dimension through a leader |
| A shank line shows inside the nut in a bolt joint | `trim off` disabled the occlusion clipping, or the part chain order is wrong (the nut is not on the shank) | Remove `trim off`; check the part chain order (from the clamped parts outwards) |
| Bolt too long/short | Wrong plate thickness / missing part in the chain / a fixed length was given via `bolt=…:8:40` | Read the report line: `Σ`, `需 l≥`, `取供货长度`, `实际外露`; fix the chain or `protrude` |
| BOM is empty | The drawing has no standard parts carrying `OCSM_PART` (drawn by hand) | Use the `XL`/`OCSMJOINT` plugin; do not explode blocks |
| **Hatch lines missing in a standard-part section view** | ① The new drawing did not run `OCSM`, so `5剖面线层` does not exist (entities on a non-existent layer are simply not drawn); ② The hatch **pattern definition is wrong**: `pattern.lines`'s `offset` must be a **world-coordinate vector** (the host un-rotates it via `dx=off·(cos a,sin a)`, `dy=(-off.x·sin a+off.y·cos a)`; `dy=0` → the spacing collapses → renders as a **solid fill**; the correct reference value is `(-2.245,+2.245)` @45°/scale 1); ③ The direction must be **baked into the line angles** (`line.angle=45°/135°+dir`, rotate `offset` in step, `pattern_angle` is only a record) — on the host's prebaked path `angle_offset=0`, so changing only `pattern_angle` makes all four pieces run the same way; ④ Degenerate boundary polygon (duplicate points/backtracking/long diagonal closure) → the even-odd rule cannot find the interior → nothing is drawn | Run `OCSM` first; use `partgen_kit::hatch_ansi31_edges/scaled` (direction already baked, scale already applied) instead of hand-writing a HatchPatternLine; deduplicate boundaries with `dedup_ring`, build them **edge-by-edge in ring order** from the template and keep them **simple polygons** (`partgen_kit::tests::hatch_patterns_are_readable_by_host` + `bearing_*_hatch_ring_order_and_simplicity` are the guard rails) |

## 4. Headless / Scripting (AI Side)

| Symptom | Cause | Fix |
| --- | --- | --- |
| `--export` cannot produce png/pdf | Only dwg/dxf is supported | Use another render path (host render API / custom drawing), or take a GUI screenshot |
| Plugin commands hang in headless mode | A headless session **does not service plugin worker requests** (no GUI event loop) | In headless mode use only `open`/`run <command+inline coordinates>`/`save`; run plugin commands in a GUI session |
| Screenshot is black/empty | The viewport is wrong (objects are elsewhere) | `ZOOM EXTENTS` first (or `ZOOM WINDOW`); note that sending ZOOM while an interactive command is running will be swallowed → cancel first |
| Chinese text garbled | `$DWGCODEPAGE` / encoding (common when converting to DXF) | See the encoding pitfalls in the `attdef-conversion` skill; read/write DWG through the host, do not assemble DXF by hand |

## 5. One Lesson

**Read the `history` log first, then take a screenshot.** All plugin reports (`OCSMJOINT：…`, `OCSM 初始化完成：…`,
`撤销：…`) are written to the command-line log; the raw error text is there too. Looking only at "the command returned ok" misses half the problems.
