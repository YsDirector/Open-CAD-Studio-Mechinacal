# 15 Centerlines (`OCSMCENTERLINE` / `ZX`)

> English translation of `handbook/15-中心线.md` (the Chinese original is the source of truth).

> Long axes, symmetry centerlines and the line halfway between two lines are all drawn with this command — the output is an **ordinary `LINE`** (not a block, not a polyline),
> landing on `3中心线层` (linetype `CENTER2`), and it can be trimmed / extended / lengthened / grip-dragged directly.
> No dimension annotation (centerlines are not dimensioned per GB).

## 1. Two Ways to Use It

| What you pick | What comes out | Line length |
| --- | --- | --- |
| **Circle / arc** | **A cross** of two lines (horizontal + vertical), both centred on the circle's centre | `diameter + n×6` |
| **A line**, then a **second line** | One line lying on the **angle bisector of the two lines** | `projected length of the first line + n×6` |

* `n` = the **frame scale** (the `比例` attribute of the frame block in that drawing; with no frame it is 1). The frame containing the pick point decides n,
  the same "scale awareness" as `D` (smart dimension) / `GDIM`.
* The overshoot is always **6 mm × n** (a 1:1 drawing → 3 mm overshoot at each end).
* The cross is always the **world-coordinate horizontal + vertical** (this plugin draws in world coordinates and does not follow the UCS).

## 2. How to Operate It

### 1. Circle / Arc → Cross Centerlines

```
ZX            ← or click "中心线" (centerline) on the ribbon
pick a circle/arc      ← one edge is enough, no need to pick the centre
```
→ the cross appears immediately (**two lines in one operation, one Ctrl+Z to undo**).

### 2. Two Lines → Angle-Bisector Centerline

```
ZX
pick the first line
pick the second line     ← what comes out is "the line in the middle" of the two
```

**Which of the two bisectors?** Two lines actually have **two mutually perpendicular** bisectors; this command takes
the bisector of **"the angle the segments themselves enclose"** (= the bisector of the angle between the two rays from their intersection point to the **far ends** of the two lines):

| Relation between the two lines | Result |
| --- | --- |
| Nearly parallel (the two edges of a slot, a keyway, a gap) | **the line halfway between them** (the right one — it will never cut across) |
| Forming an angle (polyline, V shape, chamfer) | the **interior-angle bisector** of that angle (an obtuse angle also takes the interior angle; it is not limited to acute angles) |
| Strictly parallel | takes the separate "midline" branch: direction = the line direction, position exactly halfway between the two lines |

The **position** of the midline/bisector is decided by the first line: **midpoint = the projection of the first line's midpoint onto that bisector**;
**length** = the **projected length of the first line along the bisector direction + n×6**.

> ⚠️ The two lines are "the first line sets length/position", so **whichever line you pick first is the datum**. Picked them the wrong way round → undo (`U`) and pick again.

## 3. Running with a Selection Set (for scripts / AI)

The command can take the **current highlighted selection set** directly (the same as `XL`/`ME`; in MCP, `op:"select"` names the handles):

| Selection | Result |
| --- | --- |
| 1 circle / arc | cross centerlines (two lines placed at once) |
| 2 lines | angle-bisector centerline (**the first one in the selection = the datum line**) |
| anything else (including 1 line, mixtures) | does not touch the drawing, only reports, then enters pick mode |

```bash
# MCP / HTTP example: select a circle, then run ZX
mcporter call ocs.ocs_execute --args '{"request":{"op":"select","handles":["1F"]}}'
mcporter call ocs.ocs_execute --args '{"request":{"op":"run","cmd":"ZX"}}'
```

## 4. Common Problems

| Symptom | Cause / what to do |
| --- | --- |
| Picking a circle does nothing and it reports "只能点圆/圆弧或直线" (only a circle/arc or a line can be picked) | You hit an entity inside a block / a spline / a polyline. **A polyline (LwPolyline) is not a line**: explode it with `X` first, or run `ZX` and then pick an edge that really is a `LINE` |
| The cross overshoot is wrong | No frame inserted / the frame has no `比例` attribute → `n` is taken as 1. Insert a frame again with `TF` + `OCSMFRAMEINSERT` |
| The two lines give a direction that "cuts across" | The two picked lines are **not parallel** and form an angle close to 90° — that is the interior-angle bisector, not a bug; if you wanted the "midline", do not use two lines: use 1 circle or a parallel line aligned with the first line's midpoint |
| Line too long/too short | Length = diameter (or projected length) + n×6; for an exact length grip-drag it directly (it is an ordinary `LINE`) |
| Undo | The cross is **one** undo entry (both lines go back together) |

## 5. Layer and Line Weight (do not touch)

| Item | Value |
| --- | --- |
| Layer | `3中心线层` (layer 3, `CENTER2` linetype, purple) |
| Colour / line weight / linetype | **always ByLayer** (change the layer and everything follows) |
| Printing | Centerlines **are printed** (unlike `10引导线层`, which is a non-printing annotation-intent layer) |

> `ZX` also works on a drawing where `OCSM` has never run: the command idempotently fills in the layers/linetypes first (`1轮廓实线层` … `10引导线层`).
