# 01 Initialization and Layers

> English translation of `handbook/01-初始化与图层.md` (the Chinese original is the source of truth).

> The first thing to do on a blank drawing, and the fix when layers/linetypes/styles are wrong.

## 1. `OCSM` — Initialization

```
Command: OCSM
```

It does four things (adds whatever is missing, **idempotent**, safe to re-run):

1. **Linetypes**: `CENTER2`, `DASHED2`, `DIVIDE2` (linetypes must come before layers, otherwise writing a DWG loses the linetype names).
2. **Layers**: `1轮廓实线层` … `10引导线层` (see the layer table in the 00 overview); colour/linetype/lineweight follow the GB division of labour.
3. **Text style**: `OCSM_GB` (GB font, dimension text height 5). **Old-drawing font rescue**: when a same-named `OCSM_GB` already exists with an empty TrueType font, the font is filled in from the plugin definition (an existing non-empty font is **never** overwritten, even if it is not ours).
4. **Dimension style**: `OCSM_GB` (parameters taken from Mechanical in the dimension sample dwg, font switched to `OCSM_GB`), set as the **current style**.

Example of a successful output, as emitted (still Chinese):
`OCSM 初始化完成：新增图层 N 个、线型 N 个、文字样式 N 个（OCSM_GB）、标注样式 N 个（OCSM_GB，当前样式）。数字键 1-10、TF、D 已就绪。`
(OCSM initialization complete: added N layers, N linetypes, N text styles (OCSM_GB), N dimension styles (OCSM_GB, current style). Number keys 1-10, TF, D are ready.)

> Initialization itself is a **document change** (the host does not automatically push table changes onto the undo stack), so the plugin declares its own undo point "OCSM 初始化" (OCSM initialization).

### Auto-initialization of New Blank Drawings (old drawings are left alone)

After creating a drawing you **no longer have to type `OCSM`**: before dispatching every command the plugin applies a strict heuristic, and only when all three conditions hold does it run the initialization above automatically:

1. **Never saved** (`document_path` is empty — an old drawing that was opened or saved-as is never touched, and **a blank file also counts as an old drawing**);
2. **No entities at all** (any content at all means it is not touched);
3. **No OCSM traces at all** (not one of the 10 layers exists, and there is no `OCSM_GB` text/dimension style).

When it hits, it runs **exactly** the same initialization as a manual `OCSM` (the same undo point `OCSM 初始化`, the same layers/linetypes/styles, undoable with Ctrl+Z),
echoes `新建图纸：自动执行 OCSM 初始化（旧图纸不会自动初始化）。` (new drawing: OCSM initialization ran automatically; old drawings are not auto-initialized), and then runs your command as usual
(e.g. `OCSMSHAFT`/`OCSMGEAR` is no longer blocked by "run OCSM first").

> **Old drawings are left alone**: even a single point, or a blank drawing that was saved once, is not auto-initialized;
> when initialization is needed, type `OCSM` once by hand (the only entry point for old drawings).
> If the auto-initialization is undone with Ctrl+Z, the next command adds it again by the same three criteria.

> Implementation basis: the host has no "document new/open" event (`HostNotification` only has change/selection/tab-close),
> hence the heuristic above (safety first); the trigger point = before command dispatch, reusing the body of `cmd_init`.

## 2. Number Keys 1–10 — Switch Layer / Move to Layer

| Case | Behaviour |
| --- | --- |
| **Nothing selected** | Switch the **current layer** to that layer (output: `当前图层已切换到「N…」`, current layer switched to "N…") |
| **Objects selected** | Move the **selected objects to that layer** (the current layer does not change, recorded for undo, **the selection set is kept**) |

So "move these lines to the centerline layer" = select the lines → press `3`; "draw dashed lines next" = select nothing + press `4`.

## 3. Colour / Linetype / Lineweight Are Always ByLayer

- Entity colour = ByLayer, linetype = ByLayer, lineweight = ByLayer.
- When you need to distinguish thickness/linetype, **change the layer**, do not override it entity by entity.
- Criterion: `ocs_read op:"query"` to look at the entity's color/linetype/lineweight — a concrete value (e.g. 3=green) means somebody overrode it by hand;
  change it back to ByLayer and move the entity to the correct layer.

## 4. AI-Side Recipe (initializing a new drawing)

```jsonc
// 1) create/confirm the document and read the state
{ "op": "read", "ocs_session_id": "...", "op": "state" }          // gives document_id / revision
// 2) run the initialization (pass the plugin command name directly inside run)
{ "op": "execute", "request": { "op": "run", "request_id": "init-1",
  "document_id": 1, "revision": 7, "cmd": "OCSM" } }
// 3) switch layer and draw (number keys, or a command with inline coordinates)
{ "cmd": "1" }                                                    // current layer = 1轮廓实线层
{ "cmd": "LINE 0,0 100,0" }                                       // a line accepts inline coordinates
// 4) verify
{ "op": "read", "op": "query", "parameters": { "type": "Line", "layer": "1轮廓实线层" } }
```

## 5. Pitfalls

1. **Dimensioning without running `OCSM`**: text style/dimension style missing → dimensions fall back to defaults (text takes the layer colour, the height is wrong).
   Fix: run `OCSM` once, then use `D2G` / regenerate the dimensions.
2. **Number keys do not work "while a command is running"**: during an interactive command (such as `D`, or placement mode) the input is taken as a point/parameter;
   press `Esc` first (on the MCP side `op:"cancel"`), then switch layer.
3. **Always use the 10 layer names the plugin provides**: self-made layers are not recognized by the "dimension/symbol" commands (e.g. a dimension outside `7标注层` will not be edited by `ME`).
4. If the layer template is missing it reports `OCSM: 图层模板缺失。请先运行 OCSM 初始化。` (OCSM: layer template missing. Run OCSM initialization first.) — that simply means it was not initialized.

## 6. Verification Checklist

- [ ] `ocs_read op:"commands"` shows the `OCSM` command (the plugin is loaded).
- [ ] The initialization output contains "标注样式 OCSM_GB（当前样式）" (dimension style OCSM_GB (current style)).
- [ ] After drawing a line, `query` shows it on `1轮廓实线层` with color/linetype = ByLayer.
