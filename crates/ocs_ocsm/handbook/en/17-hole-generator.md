# 17 Hole Generator (`OCSMHOLE` / short command `DK`)

> English translation of `handbook/17-孔生成器.md` (the Chinese original is the source of truth).

> **Prerequisite**: run `OCSM` init first (10 layers + linetypes + text/dimension styles). On a drawing that has not been initialized,
> running `OCSMHOLE` is **blocked outright with a report of what is missing** (same policy as `OCSMGEAR`/`OCSMSHAFT`).
>
> Placement works **exactly the same way** as `OCSMSHAFT`/`OCSMGEAR`: click "确定" (OK) in the window → back in the drawing click the base point →
> move the cursor to rotate → click again to place (repeatable; Esc to finish).
>
> The output is only the **hole itself** (pilot-hole wall + bottom cone + thread lines + counterbore/countersink + centerline), **no material-plate outline**:
> base point = hole-mouth center × material surface, hole direction is local −Y; the coarse/fine material plate and green hatching appear only in the window preview.

## 1. Two Ways to Use It

| Usage | Command | Result |
| --- | --- | --- |
| Human (recommended) | `OCSMHOLE` or `DK` | opens the **hole generator window** (hole type 2×2 + subtype/size/fit + blind/through + automatic switches + live section preview); after clicking "确定" (OK) the cursor carries the drawing, click to place |
| AI / script | `OCSMHOLE [type] [带螺纹\|无螺纹] [公制\|UN/UNC/UNF/UNEF\|G/BSPP\|R/BSPT\|NPT\|ACME\|矮牙\|Tr] [钻孔\|自定义\|间隙] [规格名 M10\|M10×1.25\|G1/8\|1/4-20\|NPT1/2\|Tr8×1.5\|ACME 1/4-16\|公称6.35 P1.058] [P1.5] [孔径8.5] [H18] [L15] [盲孔\|贯通] [全长] [6H\|6G\|精装配\|中等装配\|粗装配] [view 侧视图\|俯视图\|双视图] [at x,y] [rot deg]` | one-line direct insert (with `at` it lands at that point) |

The bracketed Chinese tokens are the **literal CLI keywords** (threaded/unthreaded, thread system, drill/custom/clearance, spec name, pilot diameter, blind/through, full thread, tolerance class, view); each is explained in §2 below.

Examples:

```
OCSMHOLE 螺纹孔 M10 H18 L15 at 100,50          # M10×1.5 threaded blind hole, hole depth 18 / thread effective length 15
OCSMHOLE 简单孔 M10 钻孔 H20 贯通               # pilot hole Ø8.5 for tapping M10, through
OCSMHOLE 沉头孔 无螺纹 M10 间隙 中等装配 H22     # counterbore Ø18×11 + through hole Ø11
OCSMHOLE 埋头孔 带螺纹 M12 细牙 全长 H30         # 90° countersink Ø24.4 + M12 fine thread (pilot defaults to coarse)
OCSMHOLE 螺纹孔 带螺纹 G G1/8 H20 L12           # spec name looked up in the table: G1/8 (= `G 公称9.728 P0.907143`)
OCSMHOLE 螺纹孔 UNC 1/4-20 H18 L15              # `1/4-20` = `1/4-20 UNC` (omitting the profile suffix also works)
OCSMHOLE 螺纹孔 Tr Tr8x1.5 H20                  # ASCII multiplication sign x = ×
OCSMHOLE 螺纹孔 NPT NPT1/2 H25 贯通             # `NPT 1/2` is accepted as well
OCSMHOLE 螺纹孔 ACME 1/4-16 H20                 # general ACME; `矮牙 1/4-16` = Stub ACME
```

## 2. Hole Types and Parameters (one set for GUI and command line)

**Hole type** (2×2 buttons): `简单孔` (simple hole) / `螺纹孔` (threaded hole) / `沉头孔` (counterbore) / `埋头孔` (countersink).
`螺纹` (thread) is not the opposite of a hole type: **counterbore/countersink can be "带螺纹" (threaded) or not** (the "带螺纹" switch).

| Hole type | Thread | Pilot hole (the drilled hole) | Additional |
| --- | --- | --- | --- |
| `简单孔` (simple) | never | determined by the subtype (see below) | — |
| `螺纹孔` (threaded) | always | minor diameter `D = d − 1.0825P` (GB/T 197) | thread major-diameter line + runout line |
| `沉头孔` (counterbore) | optional | threaded = drill from the tap-drill tooth-depth table; unthreaded = determined by the subtype | cylindrical counterbore `d2 × t` (**GB/T 152.3-1988**; recommended values below) |
| `埋头孔` (countersink) | optional | threaded = drill from the tap-drill tooth-depth table; unthreaded = determined by the subtype | 90° countersink `d2` (**GB/T 152.2-2014**, for GB/T 70.3 and similar countersunk screws; M1.6–M10) |

**Counterbore "recommended value" (automatic)**:

* `GB/T 70.1 内六角圆柱头` (hex socket head cap screw) → GB/T 152.3-1988 **Table 1** (official 3.1: applies to GB 70); default.
* `GB/T 70.2 内六角平圆头` (hex socket button head) → **marked missing**: GB/T 152.3-1988 has only Table 1/Table 2 (official 3.2: Table 2 applies to
  GB 6190, GB 6191 and GB 65) and **has no counterbore table for 70.2**; under the "no interpolation" rule, selecting it is a clear error.
* `GB 6190/6191/65 低圆柱头` (low head) → GB/T 152.3-1988 **Table 2** (an extra optional item; the data is sourced).

**Countersink "recommended value" (automatic)**: `GB/T 70.3` → **GB/T 152.2-2014** (countersinks for countersunk screws, replacing the 1988 edition);
`d2` takes the standard `dc_max` (cross-checked identical to the 1988 d2), the through hole takes `dh_min` (= GB/T 5277 normal fit), 90° geometry.

**Spec names (one table for CLI and GUI)**: in the CLI's "size" slot you can write the **spec name from the GUI dropdown** directly (the table's `name`
is the single source of truth): `M10`, `M10×1.25`, `G1/8`, `R1/8`, `NPT1/2`, `1/4-20` (= `1/4-20 UNC`),
`ACME 1/4-16`, `Tr8×1.5`…. Normalization tolerances: letter case, spaces (`G 1/8`), the ASCII multiplication sign (`Tr8x1.5`),
an omitted thread-profile suffix (`1/4-20`), an omitted system token (when the system is already written, `G 1/8`). Parsing order = **look the name up in the table first → if it misses, fall back to
the numeric `公称 d P` path** (all legacy forms are kept); the table lookup **does not interpolate**: a miss reports `未知规格名 X` (unknown spec name X) + examples for that system;
if the form belongs to another system (e.g. `G 1/4-20`) it says so explicitly: "system and spec do not match: this is a UN spec".

**Subtypes**:

* With thread: `标准螺纹` (standard thread, coarse, ISO 724) / `细牙螺纹` (fine thread, from the fine-pitch list, e.g. `M10×1.25`).
* Without thread (simple holes, and the unthreaded versions of counterbore/countersink):
  * `钻头大小` (drill size) — size = **standard twist-drill diameter series** (GB/T 6135.3-1996 straight-shank twist drills,
    0.20–20.00, e.g. Ø5.1/5.2/6.7/6.8/8.5…); for counterbore/countersink also pick the "公称" (nominal) that looks up the counterbore table.
  * `自定义` (custom) — size still picks a nominal (used for the counterbore-table lookup with counterbore/countersink), plus a **孔径** (hole diameter) in mm.
  * `螺栓间隙` (bolt clearance) — size = the bolt nominal M; pilot hole = the **GB/T 5277** through hole (only then is `配合` (fit) selectable:
    `精装配` / `中等装配` / `粗装配` (close / normal / loose fit)).

**Fit**: with thread = internal thread tolerance class (M = `6H`/`6G`; UN = `2B`/`3B`; pipe threads/trapezoidal have no class → greyed out; this batch only records it and does not change the basic profile geometry);
`螺栓间隙` = close/normal/loose fit; under the other subtypes it is **greyed out**.

## 3. Rules (settled by the user)

> **Two quantities kept separate** (conceptual model):
> **① thread effective length** (= engagement length, the **same quantity** for internal and external threads, "how deep it must screw in");
> **② hole depth / actual machined length** = effective length + **process allowance** (tool runout + incomplete threads).
> The two rows in the UI each display and edit their own value; automatic values are given separately.

| Rule | Implementation |
| --- | --- |
| **Automatic thread effective length**: M (iso724) = `1.5 × nominal diameter` (GB/T 3098.1-2010; M12 → 18); UN/ACME/Tr temporarily the same 1.5d; **pipe threads G/R/NPT = `eff_len` from the table** (R = ISO 7-1 table column 16 (no relief groove = max gauge length + assembly allowance); NPT = basic gauge length + assembly allowance + gauge-plane deviation (deviation taken at the largest +1P step, same family as NPTF LW3-55); G = same spec as R, ISO 228-1 has no such basis, **to be confirmed**) | `hole.rs::AUTO_THREAD_FACTOR`; `src/thread.rs::ThreadSpec.eff_len`; when `螺纹有效长度` (thread effective length) has "自动" (auto) ticked and "全长" (full thread) unticked |
| **Pilot (drilled) diameter** of a threaded hole = the actual drill from the tap-drill tooth-depth table (M10×1.5 → Ø8.5; off-table falls back to the GB/T 197 theoretical minor diameter d−1.0825P) | The "螺纹小径 D" (thread minor diameter) readout is still d−1.0825P (M10 → 8.376), displayed separately from the drill |
| **Automatic hole depth** = effective length + `2 × pitch` (process allowance: tool runout; M10×1.5, effective 15 → **18**). **For tapered pipe R/NPT the 2P takes the upper bound of the tapered-pipe process wording "1~2 extra threads", to cover the external taper's incomplete-thread zone** | `AUTO_RUNOUT_FACTOR = 2`; automatic hole depth = effective length + 2P; imperial P = 25.4/TPI |
| Unthreaded (simple holes, unthreaded counterbore/countersink): **neither "自动" (auto) is selectable** | GUI greys them out + unticks; the model layer errors out if given automatic values |
| Through holes: **the hole depth cannot be automatic** (plate thickness / through length must be typed) | through holes have no tool-runout issue; `孔深` (hole depth) is typed manually |
| **Through = no 118° bottom cone** | with `range=through`, `cone_height=0`; the side view draws no cone slant lines |
| **Blind-hole bottom cone 118°** (half angle 59°, cone height = R minor diameter / tan 59°) | `CONE_HALF_ANGLE_DEG = 59` |
| Thread major diameter `D1 = d`, minor diameter `D = d − 1.0825P` (GB/T 197; M10×1.5 → **8.376**) | geometry and readout share one source |
| "全长" (full thread): `L = H`, in which case the hole depth must be typed (circular dependency) | `full_thread`; no thread runout line is drawn |

With counterbore/countersink, `孔深 / 螺纹有效长度` **start from the counterbore bottom / countersink cone bottom** (both the thread and the pilot hole lie below the counterbore).

> ⚠️ For pipe threads the **depth datum = the hole mouth / end face** (material surface); the R page notes "the thread-start chamfer is ≤1 thread axially and the chamfer is included in the effective length"
> — the chamfer zone counts inside the effective length and does not add extra length.

## 4. Drawing Conventions and Layers (GB/T 4459.1 simplified representation)

* **Side view (section)**:
  * pilot-hole wall (minor diameter = pilot diameter) → `1轮廓实线层` thick solid line; the blind-hole 118° bottom cone on the same layer;
  * thread major-diameter line (thin solid) → `2细线层`, drawn from the counterbore bottom / hole mouth to the thread runout;
    the **thread runout line** → `1轮廓实线层` (connecting the two major-diameter thin lines; not drawn for full-thread / bottomed holes);
  * counterbore wall + annular bottom face, countersink 90° cone lines → `1轮廓实线层`;
  * axis → `3中心线层`, extending 3 mm at both ends.
* **Top view (end view)**:
  * one full thick solid circle at the pilot minor diameter; threaded holes add a major-diameter thin solid circle for **3/4 of a turn**
    (following the `partgen_more.rs` nut end view: `270° → 180°`, `2细线层`);
  * counterbore/countersink add one more deeper/shallower solid circle;
  * cross centerlines → `3中心线层`.
* **No dimensions are generated** (`D1`/`D` are only drawn in the window preview sketch).
* The top view (if also ticked) is placed to the right of the side view with a 20 mm gap.

## 5. Data Sources (every table's `source`/`note` is in the JSON)

| Table | File | Source | Cross-check |
| --- | --- | --- | --- |
| Thread basic dimensions (coarse 40 / fine 171) | `src/tables/threadIso724.json` | ISO 724, reference site `mecalculator.tw` (`metric-thread.min.js` dataC/dataF) | `d1 = d − 1.0825P` (GB/T 197): of the 40 coarse rows, 38 agree; two digit-transposition typos in the source table, M18 (15.394→15.294) and M48 (42.857→42.587), are corrected here per the standard. Coarse pitches agree row by row with the repo's `partsNut6170.json` / `partsHexBolt5782.json` |
| Unified inch UN (356 rows: UNC/UNF/UNEF + 4UN…32UN) | `src/tables/threadUn.json` | ASME B1.1 / GB/T 20666~20670, reference site `unified-thread.min.js` | D2=D−0.649519P, D1=D−1.082532P; three typos in the reference site corrected (1 3/8 major diameter, 8UN 2 3/4 pitch diameter); pilot hole = 75% tapping drill table or D−P |
| Parallel pipe G (24 rows) | `src/tables/threadG.json` | ISO 228-1 / GB/T 7307, reference site `pipe-parallel-thread.min.js` | D2=D−0.640327P; `eff_len` = same spec as R column 16 (no standard basis, **to be confirmed**) |
| Tapered pipe R (15 rows) | `src/tables/threadR.json` | ISO 7-1 / GB/T 7306, JLC (嘉立创) LW3-3-03 column by column | `gauge_len`/`makeup`/`eff_ext`/`eff_len` = gauge length / assembly allowance / external-thread effective / column 16 (no relief groove) |
| NPT tapered pipe (24 rows) | `src/tables/threadNpt.json` | ASME B1.20.1 / GB/T 12716, JLC (嘉立创) LW3-38 | `eff_len` = gauge length + assembly allowance + deviation (+1P; ASME L1/L2/L3 naming cross-checked) |
| ACME trapezoidal (23+23) | `src/tables/threadAcme.json` | ASME B1.5, mechtool general/stub tables | general D2=D−0.5P/D1=D−P; stub D2=D−0.3P/D1=D−0.6P (the reference site is actually stub) |
| Metric trapezoidal Tr (236 rows) | `src/tables/threadTr.json` | ISO 2901 / GB/T 5796, mechtool/164580 | D2=d−0.5P, D1=d−P |
| Counterbore Table 1/Table 2 (16+9 steps) | `src/tables/holeCounterbore.json` | **GB/T 152.3-1988** (YiJinTong 易紧通 1543 official structure: Table 1 for GB 70; Table 2 for GB 6190, 6191, 65) | cross-checked value by value against JLC (嘉立创) 5-1-43; Table 1 M4–M36 and Table 2 all agree; the JLC M30 d3=6 is a typo, corrected to the official 36 |
| Countersink (M1.6–M10, 10 steps) | `src/tables/holeCountersink.json` | **GB/T 152.2-2014** (YiJinTong 易紧通 4257; replaces the 1988 edition) | `dc_max` cross-checked value by value against the 1988 `d2`, all agree; `dh` = GB/T 5277 normal fit |
| Bolt clearance through holes (50 steps M1…M150) | `src/tables/holeClearance.json` | **GB/T 5277-1985 through holes for bolts and screws** (new05053) | the source page gives only nominal hole diameters (no deviation column); no tolerance zones are invented |
| Drills (standard twist drills, 198 steps, 0.20–20.00) | `src/tables/holeDrill.json` | **GB/T 6135.3-1996 straight-shank twist drills** (JLC 嘉立创 2.1.2.3) | cross-checked with the d1 columns of 2.1.2.2 short twist drills and 2.1.2.4 long twist drills; off-table **reports a clear error, no interpolation** |
| Thread tap drills (83 rows) | `src/tables/holeTapDrill.json` | `~/桌面/GB/公制螺纹底孔牙深明细表.png` (left coarse / right fine) | used for **threaded holes'** drill diameter; off-table falls back to the GB/T 197 theoretical minor diameter |

## 6. Pitfalls / Conventions

* **Counterbore/countersink pilot hole vs thread**: a threaded counterbore/countersink = counterbore + internal thread; the pilot (drilled) hole takes the twist drill from
  the tap-drill tooth-depth table (off-table falls back to `d−1.0825P`), not the `d1` from the GB/T 152.3/152.2 tables
  (that is the clearance hole matching a **plain-hole counterbore**).
* **Countersink geometry is a 90° cone**: cone depth = (d2 − pilot diameter)/2; the `t≈` in the table is the nominal depth (used for display in the UI).
* **Off-table specs report an error** (no interpolation, no extrapolation): e.g. no M7 for counterbore, no M24 for countersink; bolt clearance runs from M1 to M150.
* **Counterbore standard basis**: the recommended value defaults to GB/T 152.3 Table 1 (GB 70 → 70.1); Table 2 (GB 6190/6191/65)
  is switchable; **GB/T 70.2 has no official counterbore table (marked missing, no interpolation)**; GB/T 152.4 hex-head counterbores are not included.
* **Countersink**: GB/T 152.2-2014 only goes to M10; M12 and above report a clear error (no interpolation).
* Command prefix is kept in sync in three places: `command_prefixes` in `plugin.toml`, `MANIFEST.command_prefixes` in `lib.rs`
  plus the short-command dispatch, and `guide_server::COMMAND_CATALOG`; the `ribbon_registers_*` tests assert the two prefix lists are **multiset-equal**.
