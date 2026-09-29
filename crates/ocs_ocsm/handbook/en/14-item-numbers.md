# 14 Item Number Balloons (`BALLOON`, pick "序号" (item number) in GDIM)

> English translation of `handbook/14-序号标注.md` (the Chinese original is the source of truth).

> An item-number balloon is a **variant of leader annotation**: it leads a part's item number onto the assembly drawing and hooks into the BOM's "序号" (item number) column.
> It is isomorphic to a leader (a two-segment PLINE leader), but it **can extend horizontally or vertically**: one "group" holds several item numbers.

## 1. What a Group Looks Like (geometry contract)

```
one group = 1 leader line + 1 dot + N horizontal lines + N item numbers (each written centred above its own horizontal line)
                    │
                    └── the horizontal lines are connected per the extension direction:
                        horizontal = collinear in a row, joined to the next one by a V-shaped polyline
                        vertical   = stacked, joined at the leader-line end by a vertical line
```

Leader geometry (a **two-segment PLINE with 3 vertices** on `10引导线层`, exactly the same as a leader line):

| Vertex | Meaning | What is generated |
| --- | --- | --- |
| Vertex 1 (start) | the **pointer point** | draws a **dot** (Ø0.33h, filled) — not an arrow |
| Vertex 2 (bend) | the **bend point** | block origin; the **leader-line-side end** of the horizontal line |
| Vertex 3 (end) | end of the shoulder line | **sets the direction only** (the horizontal line runs from the bend point towards it) and "which way it grows" |

> **The horizontal line must be horizontal** (draw the second segment horizontally). Draw it vertically → error:
> `序号标注：横线必须水平（第二段水平画）——不支持竖肩线（序号不旋转）`
> (item numbers: the horizontal line must be horizontal (draw the second segment horizontally) — a vertical shoulder line is not supported).
> The item number is **always written horizontally** (decided by the user: no "vertical shoulder line + item number rotated 90°" style).

### Proportions (measured from the reference drawing; datum = text height h, default 3.5 mm, scaled with the sheet-format factor)

| Quantity | Value | Notes |
| --- | --- | --- |
| Horizontal-line length | text width + **1.0h** (0.5h on each side) | one character ≈1.4h, two characters ≈1.8h |
| Number position | **0.3h above the midpoint** of the horizontal line | centred (anchored `BottomCenter`) |
| Horizontal gap | **0.5h** | between two horizontal lines |
| V-shaped polyline | width 0.5h, **depth 0.38h** (bent towards below the text) | means "continuation of the same leader line" |
| Vertical pitch | **1.6h** | distance between the upper and the lower horizontal line |
| Dot | **Ø0.33h** (filled octagon) | two `SOLID` entities (the host does not draw filled circles) |

## 2. The Two Extension Directions

| `dir` | Layout | Connector symbol | Typical use |
| --- | --- | --- | --- |
| `H` horizontal (default) | horizontal lines **collinear** in a row, laid out from the leader-line end towards the end of the shoulder line | V-shaped polyline | item numbers 2 and 3 on one horizontal line |
| `V` vertical | horizontal lines **stacked** (pitch 1.6h), all aligned at the leader-line end | vertical line (at the leader-line end) | two rows such as 6/5, 14/13 |

- **The order of the entries = outwards/upwards from the leader line**: `items=5,6` → 5 hugs the leader line and 6 sits above it (consistent with 6 on top, 5 below in the drawing).
- Each group has only **one leader line + one dot** (a shared pointer): the typical case is a bolt joint (bolt/washer/nut) written as one group with a shared leader.

## 3. Item Numbers and Numbering Rules (fixed by the user on 2026-09-15)

1. **The ending must be a digit**: `7`, `A1`, `M10`; `A` and `A1B` do not count as item numbers (they error / are not accepted).
2. **The default number = the drawing's highest existing item number + 1**, keeping the prefix and the width: `A1`→`A2`, `A09`→`A10`, `9`→`10`.
   - "existing" = the item balloons already generated (①) (`OCSM_BALLOON` ledger) + the BOM row blocks' "序号" attribute (②).
3. **The BOM is sorted by item number ascending** (`A2` < `A10`, pure numbers first); every new item number adds a cell to the BOM.
4. **When a manually entered item number duplicates an existing one**, look at the "**插入序号**" (insert item number) switch in the window:
   - **checked** → every item number **after** the inserted one gets +1 (insert and shift the rest);
   - **unchecked** → treated as the same part, and that item-number **row's quantity in the BOM gets +1** (merged counting).

## 4. How to Use It (human side)

```
1) on 10引导线层 draw a two-segment PLINE: pointer point → bend point → end of the shoulder line (draw the second segment horizontally)
2) select it → GDIM → pick the type "序号"
3) fill in the entries in the panel: + add / auto number (previous +1) / ▲▼ reorder / ✕ delete
   pick the extension direction (horizontal/vertical); check "插入序号" when you need to insert in the middle
4) look at the preview (the same set of proportions as the server) → "应用并刷新" → generated on 8符号标注层 (anonymous block *XH + INSERT)
   the leader PLINE is kept (does not print); later select the annotation → ME to change it again
```

- Pick-point snapping: **when the pointer point falls inside a part block** → link that part (by reading `XDATA OCSM_PART`) and record it in the ledger;
  when it is not inside any part block, the point as drawn is used. With nested blocks the one with the **smallest bounding box** is taken (it will not wrongly attach to a big assembly block).
- Generated ledger `OCSM_BALLOON` (JSON): item-number list / direction / insert mode / linked part — the BOM sync and re-sorting rely on it.

## 5. AI-Side Recipe

```jsonc
// 1) draw the leader line (on 10引导线层, a two-segment PLINE; the second segment must be horizontal)
{ "op": "run", "cmd": "10" }
{ "op": "run", "cmd": "PLINE 20,10 45,35 70,35" }
// 2) generate straight from parameters (equivalent to a human clicking "应用并刷新" in the window)
//    POST /api/apply_refresh {"handle":"6F","url":"http://127.0.0.1:23751/DIM/BALLOON/0?items=2,3&dir=H"}
// 3) verify: the INSERT on 8符号标注层 (block name *XH{n}) + the ledger
```

- URL keys: `items=2,3` (comma-separated, outwards/upwards from the leader line), `dir=H|V`, `ins=1` (insert and shift the rest on conflict).
- The reply to `GET /api/guide?handle=…` carries `balloon{items,dir,ins}` and `balloon_next{existing,next}` (used for auto numbering).

## 6. Pitfalls

1. **The horizontal line must be horizontal**: a vertical shoulder line errors out immediately (that is a decision, not a missing feature).
2. **The pointer point is a dot, not an arrow**: do not look for an arrow the way you would with a leader line.
3. **The order of the entries**: `items` is "outwards/upwards from the leader line", not top to bottom.
4. **One group has only one dot**: if you want to lead to different parts individually, draw two groups (or use two leader lines).
5. Numbering depends on the balloons/BOM rows that "already exist" in the drawing — on a brand-new drawing the first number taken is `1`.
6. The output is an **anonymous block** (`*XH{n}`); change parameters with `ME` (delete the old and rebuild), do not hand-edit the text inside the block.

## 7. BOM Sync (implemented)

After a balloon is generated/renumbered the **BOM is synced automatically** (you can also re-sort by hand at any time with the command `BOMSYNC`):

| Rule | Implementation |
| --- | --- |
| the table is **sorted by item number ascending**, bottom-up | row order = item number ascending (`A2` < `A10`; pure numbers first) |
| every **new item number** adds **one cell** to the table | each balloon entry is one row; an entry that cannot be linked to a part creates an **empty row** (item number only, to be filled in by hand) |
| duplicate item number → "插入序号" checked: everything after it gets **+1** | the affected existing balloons are **renumbered as a whole** (blocks rebuilt, geometry/ledger updated together) + the table is re-sorted |
| duplicate item number → unchecked: that item number's **quantity gets +1** | the same item number is referenced by N balloons → that row's quantity = max(aggregated parts, references), total weight = unit weight × quantity |
| parts not referenced by any balloon | keep the item number of the same **(drawing no., name)** in the old table; if it is not in the old table → take the next number automatically (highest item number + 1) |
| **manual-edit protection** | the item number is never locked; the **quantity** can be locked (`BOMLOCK 5 3` / `BOMLOCK 5 off`); all other columns are **kept as long as they are non-empty** (only blanks get a new value). Deleting a row = reset |

- The row contents come from the part ledger `OCSM_PART`: drawing no. = code, name = name + specification, material, unit weight; the total weight is computed automatically.
- Precondition: the only part that can be linked is **the one the pointer point sits in** (the first entry of the group) — the user decided: snapping only.
- When the table template (`bom/OCSM_BOMROW.dwg`) is missing: the balloons are generated as usual, and only a warning is given in the report (it does not block the main flow).

> A drawing that has not been saved: the xlsx lands in a **temp directory** (`/tmp/OCSM/`, on Windows `%TEMP%\OCSM\`) — it will be cleaned up by the system, so save the drawing first for long-term keeping.

## 8. Checklist

- [ ] The horizontal line is horizontal, the item number is centred above it, and the dot is at the pointer point;
- [ ] The V-shaped polyline of a horizontal extension and the vertical line of a vertical extension are both on the correct side ("away from the pointer / hugging the leader line");
- [ ] The item numbers and the BOM are **consistent** (after `BOMSYNC` you can check row by row: item number/drawing no./name/quantity/total weight);
- [ ] The pointer point is inside a part block (so the ledger can link it), otherwise the BOM will only show empty rows.
