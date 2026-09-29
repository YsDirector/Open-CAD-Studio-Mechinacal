# 08 Tolerance, Fits, Roughness and GD&T Symbols

> English translation of `handbook/08-公差粗糙度与GDT.md` (the Chinese original is the source of truth).

> Dimensional tolerances, fit designations, surface roughness, geometric tolerances / datums / engineering symbols — all of them are done in the **annotation settings window** (opened with `GDIM`).

## 1. Dimensional Tolerance (`TOLERANCE`)

Two modes (switched in the settings window):

| Mode | How it is written | Use |
| --- | --- | --- |
| **Code mode** | `⌀30H7`, `⌀30k6` | fit dimensions that mate with something |
| **Limit-deviation mode** | `⌀30 +0.021/0` (upper/lower deviations) | one-off dimensions, given per the process |

- **Fit table**: the plugin has ISO 286's 4 tables built in (hole/shaft × common tolerance bands); the deviations are looked up automatically when you pick a code;
  you do not have to leaf through a handbook for the limit deviations of common fits.
- Diameter/radius tolerance: drawn as **stacked MTEXT inside an anonymous block** (upper deviation above, lower deviation below), per the GB convention.
- When only a "text override" is given, the text is emitted as you wrote it (the hand-made part is preserved).

## 2. Geometric Tolerance (`GD&T`) and Datum (`DATUM`)

- **Feature-control frame**: the leader = 1 PLINE locating the frame's position; the content = symbol (straightness/roundness/parallelism/position…) + tolerance value +
  datum letter, picked in the window (there is a **GDT symbol font**, no typing characters to fake them).
- **Datum symbol**: two GB drawing conventions (1996 / 2008); pick per the project's requirement.
- Symbol-type output lands on `8符号标注层` (anonymous block + INSERT).

## 3. Surface Roughness (`CC` / `OCSMRGH`)

```
Command: CC        (= OCSMRGH)
Flow: interactively pick the symbol insertion point → open the "粗糙度配置" (roughness settings) window → pick the form + fill in the Ra value → "确定" (OK)
Output: anonymous block (with ATTDEF text) + INSERT (onto 8符号标注层)
```

- Forms: basic symbol / material removal / no material removal / with annotation (e.g. `Ra 3.2`, `Ra 1.6`, `Rz`…).
- If you want to manage it uniformly through **leader lines**, a `RGH`-type leader + `GDIM` was an option too.
  → **Correction (2026-09-17)**: there is **no** `Rgh` in `GuideType` (and no roughness tab in the GDIM window either), so roughness always goes through the standalone `OCSMRGH`/`CC` command (pick the insertion point + settings window); the old statement on this line is void.

## 4. AI-Side Recipe

```jsonc
// roughness: pick the insertion point first (enters interactive mode) → the window parameters are submitted by a human or over HTTP
{ "op": "run", "request_id": "r1", "document_id": 1, "revision": 40, "cmd": "CC" }
POST /api/rough_apply   { "x": 30, "y": 12, "shape": "remove", "value": "3.2", … }   // creates an anonymous block + INSERT
// tolerance/geometric tolerance: leader line + the GDIM settings window (or POST /api/apply_refresh with parameters)
{ "op": "run", "request_id": "t1", "document_id": 1, "revision": 41, "cmd": "LINE 0,0 30,0" }  // dimension leader line
{ "op": "run", "request_id": "t2", "document_id": 1, "revision": 41, "cmd": "GDIM" }           // pick the tolerance in the window
```

## 5. Pitfalls

1. **A tolerance attaches to a dimension**: `TOLERANCE` adds deviations to a **dimension annotation**, it is not a standalone symbol → there must be a dimension first (`D` or a leader dimension).
2. **Pick the right basis system for a fit code**: `H7/k6` is hole basis (common), `K7/h6` is shaft basis; pick the wrong one and the deviations come out pointing the other way.
3. **Do not type GDT symbols by hand**: the font carries dedicated symbols; only picking them in the settings window guarantees the right shape and avoids rare/unsupported characters.
4. **A roughness value omits "Ra"**: the default is `Ra`; for `Rz` you must change the symbol type in the window.
5. **After editing, check the position on the drawing** (the symbol must not cover the contour lines): take a screenshot and look at it.
6. **The roughness window is also "pinned to a drawing"** (its URL carries `tab=`): the insertion only lands in the drawing that opened it; once that drawing is closed,
   the old window errors out immediately. See `06-leader-annotation.md` §5.7 for the rule.

## 6. Verification Checklist

- [ ] Tolerance text is stacked per GB (upper deviation on top, lower deviation below, decimal points aligned).
- [ ] Symbol layer = `8符号标注层`; the block is an anonymous block `*X{n}`.
- [ ] The tip of the roughness symbol's triangle points at the material surface.
