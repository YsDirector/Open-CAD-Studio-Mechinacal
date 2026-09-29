# 05 Smart Dimension `D` (`OCSMPOWERDIM`)

> English translation of `handbook/05-智能标注D.md` (the Chinese original is the source of truth).

> The fastest dimension: pick two points and the dimension appears. Good for everyday linear/aligned/radius/diameter dimensioning.

## 1. Usage

The command prints a prompt telling you it is in point-pick mode (endpoint/centre/intersection); `Enter` switches to segment picking, `Esc` cancels.

```
Command: D        (= OCSMPOWERDIM)
Output: OCSMPOWERDIM: 拾取点模式（选端点/圆心/交点），Enter 切换线段点选；Esc 取消。
```

Interaction:

1. **Point-pick mode** (default): the cursor snaps near **endpoints / centres / intersections**; click twice → the dimension appears.
2. **`Enter`**: toggles between "point-pick mode" and "**segment picking**" (segment picking = click a whole line; its two endpoints are taken automatically).
3. `Esc` **cancels** (MCP side: `op:"cancel"`).

What the command does first: make sure `7标注层` exists and that text style `OCSM_GB` and dimension style `OCSM_GB` are ready (adding missing style-table entries also changes the document, so the command declares the undo point `OCSM 智能标注` first); the dimension is then placed on `7标注层`.

## 2. What It Can Dimension

| Picked object | Output |
| --- | --- |
| Two endpoints/intersections | Linear or aligned dimension (vertical/horizontal/aligned decided automatically from the two points) |
| Circle/arc + a point on it | Radius `R…` / diameter `⌀…` (decided by the pick position) |
| Both endpoints of a segment | Same as above (segment picking mode) |

> More complex annotations (section symbols, auxiliary views, detail views, welds, leaders, geometric tolerances, datums, angles, arc lengths, roughness)
> **do not go through `D`**; use leaders: see `06-leader-annotation.md`.

## 3. AI-Side Recipe

```jsonc
// Trigger (enters interactive mode; on the AI side this is usually only used when a human would click with the mouse)
{ "op": "run", "request_id": "d1", "document_id": 1, "revision": 5, "cmd": "D" }
// After that, send each "click" as a run carrying coordinates (coordinates = world x,y)
{ "op": "run", "request_id": "d2", "document_id": 1, "revision": 5, "cmd": "0,0" }
{ "op": "run", "request_id": "d3", "document_id": 1, "revision": 5, "cmd": "45,0" }
// Finish
{ "op": "cancel", "request_id": "d4" }
```

> Whether "point coordinates fed to an interactive command" works depends on the host version; **for batch annotation from AI, leaders + `GDIM` are recommended** (see `06`),
> because leaders are entities (draw them first, then generate the annotations one by one — deterministic all the way).

## 4. Pitfalls

1. **`OCSM` initialization was not run** → styles are missing; the dimension text height/colour is wrong. Run `OCSM` first.
2. **Picking on another layer is fine**: the dimension output always goes to `7标注层`.
3. **Dimensions too small/crowded**: change the overall scale or text height of the dimension style (`OCSM_GB`) instead of editing dimensions one by one.
4. **`D` is in interactive mode**: numbers you type are taken as coordinates; press `Esc` before switching layers.

## 5. Verification Checklist

- [ ] `query` shows `Dimension` entities on layer `7标注层`.
- [ ] The dimension style name is `OCSM_GB` (or the matching scale variant `OCSM_GB_xN`).
- [ ] `undo` removes the dimension just created (whether it is pushed depends on how the host handles dimension commit; the command declares the table changes itself).
