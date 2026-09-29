# 04 Bolt Joint Assembly (`OCSMJOINT`)

> English translation of `handbook/04-螺栓副装配.md` (the Chinese original is the source of truth).

> From "two plates + one hole" to "a whole bolt joint": compute the length, place along the part chain, trim occlusions, one undo.
> **The command only does deterministic execution**; "whether a washer/locking part is needed" is a service-condition decision (see `21-knowledge-fastener-selection.md`).

> **Classify before starting** (`13-task-kickoff-protocol.md`): the hole is **already on the drawing** and you only add the bolt → back-solve from hole diameter/plate thickness (reverse mode; the flow on this page suffices);
> if "the hole is not decided yet / the joint must be newly designed" → **ask the user for the design inputs first**, and do the selection per `21-knowledge-fastener-selection.md`.

## 1. Two Ways to Use It

| Usage | Command | Good for |
| --- | --- | --- |
| **Human**: configure in a window | `OCSMJOINT` (**no parameters**) | editing the part chain while watching the live preview |
| **AI/script**: one-line direct install | `OCSMJOINT at x,y rot deg [protrude <turns>] bolt=<family>:<d>[:<l>] [plate=<th>\|gap=<th>\|nut=<family>:<d>\|washer=<family>:<d>] …` | known service conditions, batch work, regression-testable |

Human flow:

```
OCSMJOINT                       → opens the "OCSM 螺栓副装配" (OCSM Bolt Joint Assembly) window + enters placement mode
In the window: ① pick the bolt family/diameter/length (automatic or manual) / exposed thread turns
        ② edit the part chain (plate/gap/washer/nut; add, delete, move up/down)
        ③ common part-chain templates (no locking / plain washer / spring washer / double nut…)
        Right side: live preview (real geometry, including occlusion trimming) + numbers + that one inference line
        Click "装配到图纸" (assemble to drawing)
Back in the drawing: click once for the base point → move the cursor to rotate → click again to place (repeatable; Esc to finish)
```

## 2. Syntax (AI Direct Install)

```text
OCSMJOINT at x,y rot deg [protrude <turns>] bolt=<family>:<diameter>[:<length>]
          [plate=<plate thickness> | gap=<gap thickness> | nut=<family>:<diameter> | washer=<family>:<diameter>] …

Examples (all runnable):
# two 10 mm plates + grade C nut (no locking)
OCSMJOINT at 50,10 rot -90 bolt=hex_bolt_b_full:8 plate=10 plate=10 nut=nut_c41:8
# plain washer + spring washer + nut (spring-washer locking)
OCSMJOINT at 50,10 rot -90 bolt=hex_bolt_b_full:8 plate=10 plate=10 washer=washer_971:8 washer=washer_93:8 nut=nut_c41:8
# double-nut locking: thin nut inside + thick nut outside
OCSMJOINT at 50,10 rot -90 bolt=hex_bolt_b_full:8 plate=10 plate=10 washer=washer_971:8 nut=nut_61721:8 nut=nut_c41:8
# double-nut locking: two thick nuts
OCSMJOINT at 50,10 rot -90 bolt=hex_bolt_b_full:8 plate=10 plate=10 washer=washer_971:8 nut=nut_c41:8 nut=nut_c41:8
```

Key points:

- **The part chain must start with `bolt=`** (the bolt is the length datum and the axis datum).
- Each part can be written in two forms: `<kind>=<family>:<d>` (e.g. `nut=nut_c41:8`) or the **direct family-name** form `<family>=<d>` (e.g. `hex_bolt_c=8`).
- `protrude <turns>`: the desired number of exposed thread turns (1 turn = 1 pitch), **default 2.5**.
- `view <view>`: overrides the default `main` (usually no need to change).
- `trim off` / `no-trim`: **turns occlusion trimming off** (use when you want "every part drawn in full").
- `at x,y`, `rot deg`: base point (= the bolt head's bearing face) × axis direction; `rot -90` = shank pointing down.
- HTTP equivalent: `POST /api/joint`, body = `{"at":[x,y],"rot":-90,"protrude":2.5,"trim":true,"view":"main","items":[{"kind":"bolt","family":"hex_bolt_b_full","d":8},{"kind":"plate","t":10},…]}`.
- Compute only, write nothing (preview): `POST /api/joint_plan` (returns SVG + report + numbers, **does not touch the document**).

## 3. The Three Deterministic Things the Command Does

1. **Σ part height**: sum the heights of the non-bolt parts in the chain (plate/gap = the given thickness; washer/nut = the thickness in the **GB tables**,
   e.g. plain washer 97.1 M8 = 1.6, spring washer 93 M8 = 2.1, thin nut 6172.1 M8 `m` = 4.0, grade C 41 M8 `m` = 7.9).
2. **Length target**: `required l ≥ Σ + exposed turns × pitch`, and from **that family's table of supplied length steps** take the **smallest** value ≥ the requirement
   ("slightly long is fine"). If nothing fits, report the family's maximum length and suggest a different family (e.g. grade C M8 does not fit → suggest 5783 fully threaded).
   - Supplied series = the steps listed in the table (what the manufacturer actually sells). GB/T is a recommended standard and manufacturers often make steps close to it,
     so **the table is authoritative** — do not force-fit the nominal series.
3. **Placement along the axis**: each part's base point lands at its cumulative `Σ` position per its own family convention; the whole chain is **one transaction = one undo entry**.

It also emits **one auditable inference line** (command-line log + report), for example:

```
OCSMJOINT：六角头全螺纹螺栓 全螺纹 B级 GB/T 5783-2016 M8×35｜件链 板厚 10 + 板厚 10 +
平垫圈 A级 GB/T 97.1-2002 M8(厚1.6) + 标准型弹簧垫圈 GB/T 93-2025 M8(厚2.1) +
六角螺母 C级 GB/T 41-2016 M8(m7.9) → Σ31.6｜需 l ≥ 34.725(= Σ31.6 + 露出2.5扣×1.25)
→ 取供货长度 35（实际外露 3.4mm≈2.7扣）｜基点 (50, 10) rot -90°｜共 4 件
｜遮挡裁剪 六角头全螺纹螺栓 全螺纹 B级 M8×35 被遮 20~31.6
```

(in words: `OCSMJOINT：` hex-head fully threaded bolt, fully threaded class B, GB/T 5783-2016, M8×35 | part chain: 10 mm plate + 10 mm plate +
plain washer class A GB/T 97.1-2002 M8 (thickness 1.6) + standard spring washer GB/T 93-2025 M8 (thickness 2.1) +
hex nut grade C GB/T 41-2016 M8 (m 7.9) → Σ 31.6 | required l ≥ 34.725 (= Σ 31.6 + 2.5 exposed turns × 1.25)
→ supplied length 35 taken (actual exposure 3.4 mm ≈ 2.7 turns) | base point (50, 10) rot -90° | 4 parts in total;
occlusion trim: hex-head fully threaded bolt, fully threaded class B, M8×35 occluded 20~31.6)

## 4. Occlusion Trimming (outline views do not draw hidden lines)

- The stretch of **bolt shank** enclosed by a **nut/washer** (including the thread's thin solid lines) is **not drawn**: straight lines are reduced over the axial interval
  (delete the whole segment / shorten one end / split into two across the interval), arcs/circles are deleted only when occluded as a whole, and **the centerline layer is not trimmed** (the axis runs the full length).
- The criterion is "compare thickness in place": an annular part's radial profile dimension counts as covering only if it is ≥ the shank's radial dimension **within that window**
  (M8 shank Ø8/2=4; plain washer 8, spring washer 6.2, nut 7.5 all cover it).
- Trimming is done **after geometry generation and before writing the block**: a trimmed part gets a block name with a trim marker, e.g.
  `OCSM_HEX_BOLT_B_FULL_M8_35_CUT20x31.6`; the preview block is `OCSMJOINT_PREV_…`.
- Turn it off: add `trim off` to the command, or `"trim": false` over HTTP.
- **Boundary**: `plate=`/`gap=` in the chain are only a "thickness ledger"; the command does not take the plates' outlines → the shank is still drawn inside the plate hole (a section view should draw it anyway).
  Trimming is **frozen at insert time**: dragging the nut afterwards does not re-trim the bolt (run the assembly again to redo it).

## 5. Length Quick Reference (steel, coarse M thread, default exposure 2.5 turns)

| Service condition | Σ | Required l | Supplied length taken | Actual exposure |
| --- | --- | --- | --- | --- |
| Two plates 10+10 + grade C nut M8 | 27.9 | 31.03 | **35** (5783 fully threaded) | 7.1 mm ≈ 5.7 turns |
| Two plates 10+10 + plain washer + spring washer + grade C nut M8 | 31.6 | 34.73 | **35** | 3.4 mm ≈ 2.7 turns ✓ |
| Two plates 10+10 + plain washer + thin nut 6172.1 + grade C nut M8 (double nut) | 33.5 | 36.63 | **40** | 6.5 mm ≈ 5.2 turns |

> Rule of thumb: **to land at 2–3 turns of exposure, make the part chain just "fill up" the supplied step**; if it does not fill up, it jumps to the next-larger step (more exposure, still legal).
> To make the exposure tighter, adjust `protrude` (e.g. `protrude 2` or `protrude 3`) and let the command re-pick the step.

## 6. AI-Side Recipe (one full round)

```jsonc
// 1) inspect current state: document, selection (the two plate outlines the user selected), geometry
{ "op": "read", "op": "state" }
{ "op": "read", "op": "query", "parameters": { "type": "Line", "bounds": [40,-15,60,15] } }
// 2) preview first (writes nothing): get the inference line + numbers + SVG
POST /api/joint_plan  { "at":[50,10],"rot":-90,"items":[{"kind":"bolt","family":"hex_bolt_b_full","d":8},
                        {"kind":"plate","t":10},{"kind":"plate","t":10},{"kind":"nut","family":"nut_c41","d":8}] }
// 3) place into the drawing (one transaction)
{ "op": "execute", "request": { "op": "run", "request_id": "j1", "document_id": 1, "revision": 12,
  "cmd": "OCSMJOINT at 50,10 rot -90 bolt=hex_bolt_b_full:8 plate=10 plate=10 nut=nut_c41:8" } }
// 4) verify: INSERT position/block name + report lines in the log + screenshot
{ "op": "read", "op": "query", "parameters": { "type": "Insert" } }
{ "op": "read", "op": "history" }
{ "op": "capture", "scope": "viewport" }
```

## 7. Pitfalls

1. **Part-chain order = from the connected parts outward** (`plate plate washer nut`). A wrong order → wrong length and position (the command does not guess).
2. **Do not use `OCSMJOINT` as a substitute for service-condition judgement**: whether to add a plain washer/spring washer/double nuts, the bolt grade — you (or the skill layer) decide from
   material crushing, vibration and assembly frequency, then write it into the part chain. **How crushing is computed** is in `21-knowledge-fastener-selection.md` §4,
   with the companion script `manual/tools/crush_check.py` (e.g. `python3 crush_check.py M8 8.8 hex_ab mat=6061 dh=10 → 超限，加平垫后通过` — over the limit; passes after adding a plain washer).
3. **Giving a fixed length in `bolt=`** (`bolt=hex_bolt_b_full:8:40`) skips the length rule; do that only when "existing stock must be used".
4. **The two plates must be a "thickness ledger"**: the command does not measure your plates, it only accepts `plate=<th>`; a wrong thickness → the whole length is wrong.
5. **After occlusion trimming the block name carries `_CUT…`**: the same joint produces different blocks under different part chains (intentional — the geometry differs);
   to reuse one block, keep the part chain identical.
6. **The preview block `OCSMJOINT_PREV_*` is only for cursor following**: after placement nothing references it and it can be purged.
7. **The assembly window is also "pinned to a drawing"** (the URL carries `tab=`): assembly/placement happens only in the drawing that opened it; after that drawing is closed,
   the old window errors out immediately (no more 5 s of spinning, and it will not write into another drawing). The rules are in `06-leader-annotation.md` §5.7.

## 8. Verification Checklist

- [ ] The log contains an `OCSMJOINT：…` report line, and Σ / required / taken / exposure match hand calculation.
- [ ] `query Insert` part count matches the part chain (plates produce no parts); position = base point + Σ offset (along the `rot` direction).
- [ ] **No** bolt-shank lines inside the nut/washer (unless `trim off`).
- [ ] One `undo` removes the whole part chain.
