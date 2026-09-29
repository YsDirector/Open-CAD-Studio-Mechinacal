# 16 Gears (`OCSMGEAR`) — External Gear + Internal Gear (Ring)

> English translation of `handbook/16-齿轮.md` (the Chinese original is the source of truth).

> **Prerequisite**: run `OCSM` initialization first (creates the 10 layers + linetypes + text/dimension styles).
> Running `OCSMGEAR` on an uninitialized drawing is **stopped outright and reports what is missing** (decided by the user on 2026-09-17);
> otherwise the drawing you cut comes out with "the centerline is a solid white line and the hatching layer has the wrong colour".
>
> Parametrically draw a **cylindrical gear** (spur / helical, optionally with profile shift), one view per run, any of four views:
> **section view / side view / simplified front view / regular front view**.
> Placement is **exactly the same** as `XL` (standard parts): click "生成到图纸" (generate to drawing) in the window → back in the drawing pick the base point →
> move the cursor to rotate → click again to settle (repeatable, Esc to finish).
>
> **Stage 1 draws external gears only**; internal gears are stage 2 (after the external gear is accepted).
> The sole authority for the drawing method = the user's template `~/桌面/OCSM/齿轮/齿轮画法.dxf` (m=2 z=40 h=20);
> every construction rule of this command was reverse-engineered from that drawing and checked point by point.

## 1. Two Ways to Use It

| Usage | Command | Result |
| --- | --- | --- |
| Human (recommended) | `OCSMGEAR` | Opens the **gear window**: preview top-left + 4 view buttons + parameter form; after clicking "生成到图纸" (generate to drawing) the gear follows the mouse and a click places it |
| AI / script | `OCSMGEAR [内齿轮\|int] <m> <z> [h] [key=value…] [view <view>] [at x,y] [rot deg]` | Generates and inserts directly (`at` given → lands on that point, otherwise on the pending placement point / origin) |

Parameter keys (the command line and the window share one set):

| Key | Meaning | Default |
| --- | --- | --- |
| `int` / `内齿轮` (positional, or `kind=internal`) | **switch to an internal gear (ring)** — see §4b | external gear |
| `m` (1st positional) | normal module Mn (for a helical gear this is Mn) | — |
| `z` (2nd) | number of teeth | — |
| `h` (3rd / `h=`) | thickness (face width) | external gear `10×m`; internal gear `15×m` |
| `ha=` | addendum coefficient ha* | `1` |
| `c=` | clearance coefficient c* | `0.25` |
| `alpha=` (the compact form `α25` is also accepted) | **basic profile pressure angle α** (degrees; default 20°, sensible range **10°<α<50°**, common 14.5/15/17.5/20/22.5/25/30/37.5/45) | `20` |
| `beta=` | helix angle β (**degrees**, right-hand positive, left-hand negative; 0 = spur) | `0` |
| `x=` | profile shift coefficient Xn | `0` |
| `view` | `剖视图`/`section`, `侧视图`/`side`, `简化正视图`/`simplified`, `常规正视图`/`front` (an internal gear also accepts `端视图`) | `剖视图` |
| `at x,y` | insertion point | pending placement point or origin |
| `rot deg` | rotation angle (counter-clockwise, degrees) | `0` |

Examples:

```
OCSMGEAR 2 40 20                                  ← section view (default), lands on the pending placement point
OCSMGEAR 2 40 20 view 常规正视图 at 300,200        ← regular front view, lands on (300,200)
OCSMGEAR m=3 z=25 h=30 beta=-8 x=0.2 view 侧视图
OCSMGEAR 2 40 20 alpha 25 view 剖视图              ← pressure angle 25° (α25 also works)
OCSMGEAR int 2 40 30 alpha 25 view 剖视图        ← internal gear (ring), thickness 30, pressure angle 25°
OCSMGEAR int 2 40 30 view 端视图 at 300,200
OCSMGEAR m=2 z=40 h=20 alpha 45 ha=0.5 view 端视图  ← 45° must go with a short addendum (ha*<1, otherwise the tip becomes pointed)
```

### Interoperability With the Shaft Generator (Expressions)

At the bottom of the parameter area of the gear window there is a read-only "轴生成器表达式" (shaft-generator expression) box + a "复制表达式" (copy expression) button (same source as the "command-line equivalent" hint,
refreshed live with mode/parameters/view, but without `view`/`at`):

* Both gear mode and spline mode emit the **same unified tooth-profile segment expression** (decided by the user on 2026-09-23):
  `MARK KIND M… Z… ALPHA… X… DA… DF… BETA… H…`:
  * `MARK` = `GEAR` (gear) / `SPLINE` (involute spline) — a **drawing switch (not a classification label)**: for the shaft generator's
    **regular side view** the inner diameter (external teeth = root circle; internal teeth = the inner tip circle) thin solid line on `2细线层` (cyan ACI 4)
    is drawn for `SPLINE` (the spline minor-diameter thin line) and not drawn for `GEAR` (a gear has "no root line"; direction corrected 2026-09-23, consistent with the existing convention);
    in section views both markers draw it (`1轮廓实线层`).
  * `KIND` = `EX`/`IN` (external/internal; default EX); `DA`/`DF` = major/minor diameter (for a spline computed by the standard engine
    GB/DIN/NF/ANSI; for a gear computed locally per the `da/df` convention); after pasting, the shaft generator gets the same geometry.
  * Examples: `GEAR EX M3 Z20 ALPHA20 X0 DA66 DF52.5 BETA0 H30`;
    `SPLINE EX M3 Z20 ALPHA30 X0 DA63 DF54.6 BETA0 H30`;
    internal spline `SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30`.
* The old form `GEAR M3 Z20 H30 ALPHA20` is still parsed on the shaft side (equivalent to `GEAR EX … X0`, with DA/DF derived from
  `ha*=1, c*=0.25`); the spline expression is provided by `expr` from `/api/gear_info` (engine DA/DF) and refreshes immediately after a parameter change.

After copying, paste it into the expression box above the segment table of the shaft generator (`OCSMSHAFT` window) or into a row's GEAR input box;
it is parsed into segments by the segment syntax of `shaft.rs` (tolerating an `OCSMGEAR`/`OCSMSHAFT` prefix, Chinese spaces and multiple spaces;
a failed parse reports the reason as "segment N"); the shaft generator also has an "打开齿轮生成器" (open gear generator) button that opens the gear window in one click.

Note: **internal gears / internal splines are drawn by the minimal convention in the shaft generator** (outer outline = major diameter `DA`, bore line = minor diameter `DF`;
the hatching approximates them as solid segments and does not cut the bore out; the true internal tooth profile is still produced with `OCSMGEAR`);
tooth-profile segments on a shaft are spur only (`BETA` ≠ 0 is a clear error); DP diametral pitch is converted with `m = 25.4/DP` into the unified expression's `M`.

## 2. What the Four Views Are

| View | Content | When to use |
| --- | --- | --- |
| **Section view** | Tip cylinder (with axial chamfers) + **root line** + pitch line (dash-dot) + axis + **hatching** (ANSI31, two rings above and below the axis: root circle ↔ axis) | Main view of an assembly/part drawing, most common |
| **Side view** | Same as above but **not sectioned**: no root line, no hatching; a helical gear gets three extra **thin solid lines** showing the tooth helix direction | Paired with the section view; a must for helical gears |
| **Simplified front view** | Tip circle (thick solid) + pitch circle (dash-dot) + root circle (thin solid) + cross centerlines | Quick representation, assembly drawings |
| **Regular front view** | True **involute** tooth profile (8 entities per tooth) + pitch circle + cross centerlines | When you need the real tooth form (wire EDM, inspection) |

## 3. Drawing Essentials (all reverse-engineered from the template, not invented in a handbook)

* **Tooth profile**: involute (**default α=20°, adjustable with `alpha=`**; 14.5°/25°/30°/37.5°/45° are all supported), represented by a **clamped cubic B-spline**, **7 control points per tooth profile**
  — the fit points are taken at `r = effective start + (ra−effective start)·{1/8, ¼, ½, ¾, 1}` (5 points), knots parametrized by **chord length**.
  The effective start is normally = base circle radius `rb`; when the base circle falls inside the root circle (`rb < rf`, e.g. α>20.36° at z=40),
  the involute has no material meaning below the root circle, so the fit band starts from the **root circle** instead (at the default 20° `rb=37.588 > rf=37.5`, so the behaviour is point-for-point identical to before the α parameter was added).
  Template measurement: all 80 splines deviate from the theoretical involute by at most 0.012° (≈9 µm); this implementation ≈5 µm.
* **Precision is a hard constraint**: this scheme of "8 entities per tooth / 7 control points per profile" is the best balance of machinability and performance that the user has validated,
  **do not raise the precision** (higher precision was tried; the computer cannot keep up).
* **Root fillet**: ρ = **0.38m** (GB/T 1356). **Large tooth count (the template convention)**: the centre lies on `r = rf+ρ` and the **distance to the profile start point = ρ**
  (tangent only to the root circle, "leaning against" the profile).
* **Small-tooth-count degradation** (decided by the user on 2026-09-17 after z=17 rendered abnormally): the construction above is equivalent to the triangle
  `r_c = rf+ρ`, `r_s = profile start radius`, `ρ` — the **solvability condition = |r_c − r_s| ≤ ρ**. With few teeth `r_s` is more than ρ above `r_c`, so there is no solution.
  When there is no solution it does **not** produce a messy drawing; it gives a warning + two degradation levels (both written into the command prompt and the GUI prompt):
  1. **Degradation ① (FreeCAD `fcgear/involute.py` convention)**: the involute runs only to the **base circle**, and below the base circle a **line pointing at the centre**
     connects to the top of the fillet arc (the fillet is kept). On the same radius line as the arc top, the centre angle is `δ = 2·asin(ρ/(2·Rci))`.
  2. **Degradation ②**: even the fillet does not fit → **fillet = 0** (the line falls straight onto the root circle).

  > Boundary (m=10): z≥20 uses the template convention; z≈17↔19 is the critical zone; z=17/14/7 uses degradation ①.
  > The degraded band misses the concave shape of the true undercut curve (the true root = the offset curve of the trochoid traced by the cutter-tip fillet centre, which needs cutter phase calibration) —
  > the rendering is fine, but it differs slightly from the actual hobbed shape; for the true undercut curve use a profile shift (`x=`) or supply another drawing template.
* **Axial chamfer**: `C = round(0.6m)` (decided by the user), 45°, on **both ends of the tip cylinder**;
  tip face length = `h − 2C`, and the end-face lines are drawn to `±(da/2 − C)`.
  * The side view has two **chamfer step lines** (`x = ±(h/2 − C)`);
  * **the section view does not** (that is how the template is drawn; the edge is omitted in a full section).
* **Pitch circle / pitch line**: always a **dash-dot line** (`3中心线层`).
* **Centerline length**: `annotated length + 6n` (n = sheet scale), the same convention as `OCSMCENTERLINE`.
  * Front-view cross = `da + 6n`; the pitch line and axis of the section/side view = `h + 6n`.
  * (In the template the side-view pitch line is `h+4` and the simplified-view cross is `da+4`, which are hand-drawing deviations; this command unifies them to 6n.)
  * **The shaft generator's tooth-profile segments (GEAR/SPLINE) use the same convention** (fixed 2026-09-23): the two `3中心线层` pitch-circle lines have a
    total length = segment length + 6n, extending 3n at each end, symmetric about the segment centre (same function `gear::centerline_len`);
    the axis is still full length + 6n (3n at each end).
* **Section-view hatching**: one HATCH, **two boundary rings** (one above and one below the axis, extent = root circle ↔ axis)
  — the teeth are treated as not sectioned (GB/T 4459.2). Pattern ANSI31, scale 1.0, angle 0.
* **Tooth centre phase**: the tooth centreline is at `pitch/2 + k·pitch` (i.e. the tooth-space centre is at 0°, at the pitch angle), consistent with the template.

## 4. Helical Gears (β≠0)

* The front-view tooth profile is drawn with **transverse parameters**: `mt = Mn/cosβ`, `αt = atan(tanαn/cosβ)`, `d = mt·z`.
* The **side view** gets **three parallel thin solid lines** (`2细线层`) showing the tooth helix direction:
  * angle with the centerline (axis) = **90° − |β|**;
  * spacing = **5 mm × k** (k = view scale, i.e. the sheet `比例`; a 1:2 drawing → spacing 10);
  * length covers the whole view;
  * **right-hand `/ / /`** (β positive), **left-hand `\ \ \`** (β negative);
* The section view does not get these three lines (user convention: draw them in the side view only).

## 4b. Internal Gears (Rings) — Stage 2

Template: `~/桌面/OCSM/齿轮/内齿轮.dxf` (m=2 **z=40 h=30**, i.e. ring width 15m) +
usage example `内齿轮使用示例.dxf`. Command: `OCSMGEAR int 2 40 30 view 剖视图`.

### Only Two Views (the template has only these two)

| View | Content |
| --- | --- |
| **Section view** | Ring cut axially: **root line (outer circle ±df/2) + tip line (bore ±da/2) + 45° hole-mouth chamfer + pitch line (dash-dot)**, **23 lines in total** (10 contours × 2 sides + pitch lines × 2 + axis × 1). **No hatching** |
| **End view** (`front`/`端视图`/`常规正视图`) | True involute **tooth-space** profile (teeth grow towards the centre), 8 entities per tooth + **cross centerlines** (length = df + 6n). **No pitch circle drawn** (the internal-gear template does not have one; the external-gear template does) |

`侧视图` / `简化正视图` are **rejected with a clear error** on an internal gear: the template provides no drawing method for these two views,
and per the project rule "do not guess a drawing method the template does not have" — if you want them, supply a template first.

### Drawing Essentials (reverse-engineered + checked point by point, deviation from the template ≤ 0.0036 mm)

* **Core conclusion: the *tooth-space* shape of an internal gear = the *tooth form* of an external gear with the same parameters** — the same involute, the same
  `ψ(R) = st/(2r) + (inv αt − inv αR)`, only this angle is measured from the **tooth-space centreline**.
  (Template check: for m2 z40 at R=42.076, ψ=0.9869° vs. theoretical 0.987°)
* The tip circle is **inside**: `da = d − 2ha` (76); the root circle is **outside**: `df = d + 2hf` (85); ρ = 0.38m (0.76).
* The root-fillet centre lies at **`rf − ρ`** (for an external gear it is `rf + ρ`) — measured 41.74 in the template. The tangent point is **solved for**
  (solving `|P(R) + ρ·n(R)| = rf − ρ`), unlike an external gear where the profile start point is fixed first and the centre solved afterwards.
  * Out of the four roots (two normal branches × two sides) pick the one whose **centre falls inside the tooth-space angular range at its own radius**
    (`|Δθ| < ψ_space(|F|)`) — picking only by "closest to the centreline" or "same side as the profile" chooses wrongly at m=10.
* Phase: **the tooth centre is at 0° and the tooth-space centre at half a pitch** (4.5° in the template), structurally the same phase convention as the external-gear template.
* Tip-arc endpoints = **tooth centre ± tip half-angle** (`pitch/2 − ψ_space`); the two root-arc halves are picked by **offset ordering** of the fillet tangent points on the two sides,
  and must not be hard-wired as "the − side first" (the two sides swap when the centre crosses the tooth-space centreline).

### Internal Gears With z ≤ 33: Tip Circle Below the Base Circle (common!)

For an internal gear `da = d − 2m` and base circle `db = d·cos20°` — `da < db` ⇔ **z < 2/(1−cos20°) ≈ 33.2**.
That is, the common tooth counts of an internal gear (z=24…40) **mostly fall into "tip circle below base circle"**: the involute cannot run down that far,
so this command draws the simplified form "involute down to the base circle → **radial line** to the tip circle" and says so in the prompt
(the true tip is formed by the envelope of the shaper-cutter tip, not an involute; for an exact drawing supply a template).

### The Ring's Outer Wall in the Section View: Left to the User/AI (decided by the user on 2026-09-17)

The template draws only to the **root circle** and **does not hatch**, because the ring's outer-wall structure (rim / web / keyway / lightening holes) varies with the application
and one tooth-form drawing must serve different rings. **How to extend it** (following the sample in `内齿轮使用示例.dxf`):

1. In the section view, thicken the ring outwards from the **root line**, keeping the axial width `h` (example: add another 10 mm outside);
2. The new outline goes on **`1轮廓实线层`**, end faces/fillets as usual;
3. Hatching: **`5剖面线层` + ANSI31 / scale 1.0**, **one HATCH with two rings** (one above and one below the axis,
   bounded by the outer-wall rectangle − the cavity inside the root line) — exactly the same hatching convention as this command's external-gear section view.

The generation prompt quotes this extension recipe verbatim, so the user/AI can keep drawing without coming back to the handbook.

## 4c. Pressure Angle α (`alpha=` / `α25`)

Default **20°** (GB/T 1356 basic rack profile); any value **10°<α<50°** is accepted; common steps:
14.5 / 15 / 17.5 / 20 / 22.5 / 25 / 30 (involute splines) / 37.5 / 45 (DIN/ANSI short teeth).

Where α enters the geometry (consistent with the existing formulas, nothing new invented):

* base circle `db = d·cos αt` (spur `αt = αn`; helical `αt = atan(tanαn/cosβ)`);
* `inv αt` in the profile ψ(R) (`αR = acos(rb/R)` changes with db);
* transverse pitch-circle tooth thickness `st = πMn/(2cosβ) + 2Xn·Mn·tanαn` (the shift term follows α).

**Quantities that do not follow α automatically (the user gives them manually per the standard in use)**: addendum coefficient `ha*`, clearance coefficient `c*`,
root fillet coefficient `ρ = 0.38m`. For the old 14.5° system, 45° splines etc. specify them explicitly with `ha=` / `c=`.

**Pointed tip (the limit for large α)**: for an external gear with a full tooth depth `ha*=1`, when `2·ha*·tanα ≳ π/2`
(about α>38.15°) the two involutes meet before the tip circle, `ψ(da/2) ≤ 0`.
This command does not force a drawing (that would produce an unmachinable mess with crossed tip lines), instead:

* the **regular front view** reports a clear error `齿顶变尖/渐开线交叉…` (tip pointed / involutes crossing…);
* the **section view / side view / simplified front view** are generated as usual (they only use d/da/df, unaffected by the profile);
* `notes` give repair guidance (reduce ha*; 45° splines usually use ha*=0.5, or reduce α/increase z).

Example: `OCSMGEAR m=2 z=40 h=20 alpha 45 ha=0.5 view 常规正视图`.

## 5. Profile-Shifted Gears (Xn≠0)

* `da = d + 2Mn(ha* + Xn)`, `df = d − 2Mn(ha* + c* − Xn)`;
* tooth thickness (transverse) `st = πMn/(2cosβ) + 2Xn·Mn·tanαn`, and the whole profile is rotated by the corresponding half-angle about the centre;
* the addendum **does not subtract the addendum modification coefficient Δy** (a single-part drawing has no mating centre distance information; if needed, compute it from the mating centre distance).
* When `z < 17` and `Xn = 0` the prompt reminds you that "real hobbing would undercut; this view does not draw the undercut".

## 6. What Is Generated

* **Block**: named like `OCSM_GEAR_M2_Z40_H20_SECTION` (**a non-default pressure angle adds `_A25`/`_A37_5`**, so a 20° and a 25° part never hit a block of the same name; a helical gear adds `_B8R`/`_B8_5L`, a shifted gear adds `_X0_3`/`_XN0_25`),
  in the same style as standard-part blocks; a block of the same name is **reused idempotently** (the same parameters and view build it only once).
* **Placement**: the block insert (`INSERT`) lands on `1轮廓实线层`, the insertion point = the point you click/give, the rotation = the angle you mark;
  one generation = **one Ctrl+Z undo**.
* **Records**: the insert entity carries XDATA `OCSM_PART` (same app name as standard parts), recording `family=gear`, m/z/α/ha*/c*/β/h/Xn,
  the view, the spec text, and the **blank mass** (solid, without a bore; correct it per the actual hole pattern if you need it in a BOM).
* **No dimensions** (same as standard parts: the entities go only onto OCSM layers, no annotations are generated).

## 7. Notes / Frequently Asked Questions

| Symptom | Cause / handling |
| --- | --- |
| Reports "这张图还没跑过 OCSM 初始化" (this drawing has not run OCSM initialization) | **Run `OCSM` first** (or the initialization in the "图幅" group of the ribbon) before drawing a gear; this is a deliberate block, not a bug |
| Prompts "齿根圆角无解…已按…降级出图" (root fillet unsolvable … degraded drawing produced) (**external gear**) | Too few teeth, the 0.38m fillet does not fit (`\|rf+ρ − profile start\| > ρ`). It degrades automatically (FreeCAD convention: line + fillet below the base circle) and the drawing is not a mess; use a profile shift if you want the real shape |
| Prompts "内齿轮齿根圆角无解" (internal-gear root fillet unsolvable) (**internal gear**) | The internal fillet is "solved" (centre at rf−ρ); the prompt appears only when there is no solution: it is drawn without a fillet (the profile ends in a radial line falling on the root circle). This is a different logic from the external-gear degradation, do not mix them up |
| With few teeth the root looks like a "line + small fillet" instead of a concave undercut curve | Same as above, the degradation convention (that is how FreeCAD `fcgear` draws it). The true undercut curve = the offset curve of the trochoid traced by the cutter-tip fillet centre; it needs cutter phase calibration and can be done in stage 2 |
| An internal gear reports "不提供「侧视图」视图" (the 「侧视图」 view is not provided) | The internal-gear template has only the section view + end view, so it deliberately refuses (do not guess a drawing method the template does not have). Supply a template if you want it added |
| No hatching found in an internal-gear section view | **Deliberately not drawn**: the ring's outer-wall structure is left for the user/AI to extend and then hatch; see the extension recipe in §4b |
| An internal gear prompts "齿顶圆…低于基圆" (tip circle … below the base circle) | A normal prompt for z≤33; it is drawn in the simplified form (radial line up to the tip circle), not a bug |
| A helical gear's tooth profile looks "a bit fatter" than a spur one | Normal: the pitch circle `d = Mn·z/cosβ` is larger than for a spur gear |
| Reports "齿顶变尖/渐开线交叉" (tip pointed / involutes crossing) | α too large for a full tooth depth (e.g. 45° with ha*=1): reduce `ha=` (45° splines usually use 0.5) or increase z; the section/side/simplified front views are unaffected |
| The three thin lines are not visible in the side view | β=0 (spur) does not draw them; filling in only degrees/minutes without degrees also counts as 0 |
| The centerline is a bit longer than in old drawings | Old drawings were hand-drawn with `+4`; this command uses the GB convention `+6n` throughout |
| The section view has no chamfer step line, the side view has | Per the template: a full section omits that edge |
| The generated drawing is too big / too small | Gears are drawn at **true size**; the sheet scale is controlled by the frame (`TF`), and only the centerline/thin-line spacing follows the scale |

## 8. How to Change the Drawing Method

1. First change the table at the top of **`crates/ocs_ocsm/src/gear.rs`** (rule → checked value);
2. The unit tests contain the template measurements (`template_basic_dimensions` / `flank_fit_points_match_template` /
   `fillet_is_tangent_to_root_and_touches_flank` / `section_view_has_two_hatch_loops_and_root_lines` …);
   after changing, run `cargo test -p ocs_ocsm --lib gear::`;
3. Drawing comparison: `cargo test -p ocs_ocsm --lib gear::tests::dump_views_csv -- --ignored --nocapture`
   exports the four external-gear views + internal gears (`int_m2_z40_front/section`, `int_m10_z24_*`, `int_m10_z17_*`)
   to **`~/桌面/OCSM/review/*.csv`** (a persistent directory; it used to write to `/tmp` and was lost on reboot),
   where they can be overlaid directly on `齿轮画法.dxf` / `内齿轮.dxf` for checking.
4. Template check script for internal gears: `~/桌面/OCSM/review/compare_int.py` (bidirectional nearest neighbour + overlay image `int_overlay.png`).

## 9. Involute Splines (generated only in `OCSMGEAR`'s spline mode)

> An involute spline shares the involute tooth profile with gears but is not a gear: it is drawn by the spline standards
> (GB/T 3478.1-2008 / DIN 5480-1:2015) basic profiles and fit conventions. The core geometry (conversion / table lookup / inspection / transverse profile) lives in the
> **shared calculation engine `src/invol_spline.rs`**; the **only generation entry point is the "花键模式" (spline mode) of the gear generator (`OCSMGEAR`)**
> (views/block names/GUI/command line are all organised the gear way).
> **Involute splines are generated only by the gear generator (decided by the user, 2026-09-22)**: XL's `detail_invol_spline`
> and the shaft generator's `INVOLSPLINE` shaft segment **have both been removed**; there is no involute-spline segment on a shaft any more — for a spline on a shaft use the
> rectangular spline `SPLINE`.

### Entry Syntax

- **Gear generator (the main entry)**: `OCSMGEAR 花键 [内花键] [std=GB|DIN] [profile=GB30R] [db=40] <m> <z> [x=..] [h=..] [view 端视图|侧视图|剖视图]`;
  **an internal spline follows the internal-gear convention exactly** (user ruling: "the internal-spline section view is the same as an internal gear, there is no side view"):
  only `view 剖视图|端视图`, no side view; in the section view the ring's internal teeth are not sectioned and there is no hatching.
  The GUI (the window `OCSMGEAR` opens without arguments) has a "花键模式" (spline mode) checkbox at the top: when ticked, the gear parameters are extended with
  standard number / profile / d_B / hf* / ρf* / cF*, and the same "齿轮种类" (gear kind) switch toggles **external spline / internal spline**;
  gear mode (unticked) accepts only the regular items module/teeth/pressure angle/profile shift, and giving a standard number or d_B is a **clear error**.
  **The GB/T 3478 basic profile has no profile shift (user ruling 2026-09-23)**: in spline mode the GB x input box is **locked at 0**
  (prompt "GB/T 3478 不变位：x 恒为 0，不可编辑"; a non-zero x on the CLI/query string → clear error);
  the x of DIN/NF is still a read-only value derived from `d_B`/`A`, and the x behaviour of gears/ANSI is unchanged.
- **Shaft generator**: **shares the unified tooth-profile segment expression** with the gear generator (2026-09-23): the existing `GEAR` segment is extended to accept
  `EX/IN`, `X`, `DA`, `DF`; `SPLINE` + tooth keywords (M/Z/EX/IN/DA/DF…) = an involute-spline tooth-profile segment
  (`MARK` drives the minor-diameter thin solid line of the regular side view: drawn for `SPLINE`, not drawn for `GEAR`; direction corrected 2026-09-23);
  the pitch-circle `3中心线层` overhang of a tooth-profile segment matches the gear generator: total length = segment length + 6n (3n at each end,
  the same `gear::centerline_len`; the same convention for external/internal teeth and external/internal splines). The rectangular spline `SPLINE 6x23x26x6 L30` (GB/T 1144) is **unchanged**.
  The old syntax is still accepted; for copying the expression from the gear window see "Interoperability With the Shaft Generator (Expressions)" at the end of §1.
  The shaft-generator window **has only the regular / section views** (the dual view was removed 2026-09-23; the old `VIEW 双` is a clear error and does not silently degrade to regular);
  **it opens with an empty segment table** (no pre-filled example shaft features): click "+ 加行" (add row) to add the first segment, or paste an expression into the "行文本" (row text) / expression box.
- ~~Old XL structural element~~: `XL detail_invol_spline …` has been removed (2026-09-22) — to produce a spline drawing use the gear generator above.

### Presets and Conventions

| Standard | Profile (code) | α | ha* | hf* | ρf* | cF* |
| --- | --- | --- | --- | --- | --- | --- |
| GB/T 3478.1 | 30° flat (`GB30P`) / 30° round (`GB30R`, default) / 37.5° round (`GB375R`) / 45° round (`GB45R`) | 30/37.5/45 | 0.5/0.45/0.4 | 0.75/0.9/0.7/0.6 | 0.2/0.4/0.3/0.25 | 0.1 |
| DIN 5480-1 | 30° round (`DIN30`) | 30 | 0.45 | **0.55** | 0.16 | 0.1 |

- **Basis for DIN `hf*=0.55`**: DIN 5480-1:2015 clause 5.1 — for flank centering,
  `h_fP = 0.55·m` is the reference; hobbing −0.1m and shaping ±0.2m are tool-related corrections, to be overridden with
  `InvolParams::with_coeffs` when needed. With 0.55, `d_f1 = mz+2xm−1.1m` matches the DIN 5480-2
  nominal table row by row (m=0.5, x·m=0.225: z=34 → 16.90, z=10 → 4.90); the old value 0.60 is off by 0.1m.
- All diameters follow the standard formulas: `d=mz`, `db=d·cosα`, external spline `da=mz+2xm+2ha*m`, `df=mz+2xm−2hf*m`,
  tooth thickness `s=mπ/2+2xm·tanα`; DIN shift range `x∈[−0.05, 0.45]` (`validate` rejects out-of-range).
- **DIN `d_B` (reference diameter) is the main parameter**: data source DIN 5480-2:2015-03 nominal table, already in the repository as
  `crates/ocs_ocsm/assets/din5480_2_nominal.csv` (721 rows = old 618 + m=1.5 screenshot 56 + m=5 screenshot 47,
  see `din5480_2_notes.md` in the same directory). How to give it: `d_B+m` looks up z in the table (multiple z variants are listed as candidates), `d_B+z` looks up m,
  `m+z` computes d_B from the formula and displays it; when all three are given and inconsistent, **d_B wins and z is recomputed, with an explicit note** — no "inconsistent combination" error.
  Off-table values are derived from the formula (marked "derived value, table miss" + a feasible interval); m=1.5/m=5 have both been added, with no missing steps.
- Conversion between `d_B` and `m/z/x` (**reverse-engineered from the OCR table and verified row by row over the whole table**, not the standard's own formula):
  `x = (d_B − m(z + 1.1)) / (2m)`, equivalent to `d_B = d + 1.1m + 2x·m`.
  x is always determined by `d_B` and its source is displayed (table page / formula solution); the old `d_B_estimate = m(z+2x)` is kept only for compatibility.
- **GB has no concept of a reference diameter**: under GB/T 3478, giving `d_B` always reports
  "基准直径 d_B 是 DIN 5480 的概念，GB/T 3478 体系请给 m 与 z（本体系不用 d_B）" (the reference diameter d_B is a DIN 5480 concept; in the GB/T 3478 system give m and z — that system does not use d_B);
  in the gear GUI's spline mode the GB d_B input box is not shown; **the module `m` input box is shown for GB/DIN/NF**
  (GB = m/z driven; DIN = d_B+m; NF = A+m, d_B/A may be empty and derived from m/z), while under ANSI the same input box switches to diametral pitch `P` (A/B dropdown).
- **The GB basic profile has no profile shift**: `x` is always 0 for a GB spline (locked in the GUI; a non-zero x on the CLI/query string reports
  `GB_X_MSG`, the same convention as intercepting d_B / diametral pitch at the entry); only for DIN/NF is x an independent variable / derived quantity.
- **Internal spline** (material outside, teeth pointing inwards; shares one involute with an external spline of the same parameters, tooth space = external-spline tooth form):
  GB Table 3 convention `D_ei = m(z+1.5)/(z+1.8)/(z+1.4)/(z+1.2)`, `D_ii = D_Fe max + 2C_F`;
  DIN convention `d_f2 = d_B`, `d_a2 = d − 0.9m + 2xm` (identities of the 721-row nominal table).
  **The view convention is exactly the same as for internal gears**: only **section view + end view**, **no side view** (user ruling "the internal-spline section view
  is the same as an internal gear, there is no side view"); requesting a side view is a clear error and no longer produces the old draft without template evidence.

### Views and Layers

- `front` (called "端视图" in spline mode), transverse face: a **true involute** per tooth (`INVOLUTE_SEGMENTS=12` polyline) +
  tip/root arcs + cross centerlines (phase: tooth-space centreline at 0°+k·360°/z, same as a rectangular spline); no root fillet arcs are drawn
  (ρf is exported numerically only). A degenerate profile (external spline tip pointed / internal spline tooth space too wide) is a **clear error**, no messy forced drawing.
- `side` / `section`, axial: an external spline is chamfered **at both ends** (convention from the gear side view: `C=round(0.6m)`, end-face height
  `ra−C`, tip face length `L−2C`, chamfer diagonals + step verticals; the side view includes the step verticals, the section view omits them as for gears) + minor-diameter lines
  (regular `2细线层`; in a section view they move to `1轮廓实线层` + two rings of hatching, hatching = axis ↔ root circle) + **two pitch-circle lines**
  (`3中心线层` dash-dot, `y=±d/2`, length = `L + 6n`, extending `3n` at each end — the same algorithm as the gear side/section view's
  `cl = h + CENTER_OVERHANG·n`; the axis and the pitch lines have the same length). The **only drawing difference** from the gear side/section view
  = **a spline has those two `2细线层` minor-diameter (root circle) lines, a gear does not** (everything else is shared). **An internal spline has no side view**;
  its section view uses `gear.rs`'s shared ring-section template `internal_bore_section` (end faces / tip line / root line /
  bore wall / hole-mouth chamfer + pitch line / axis, 23 lines, **no hatching**, structurally the same as an internal gear; the differences are only the tooth parameters
  `D_ei/D_ii` ↔ `da/df` and the chamfer `round(0.6m)`, and the ring's outer wall is left for the user to extend; the pitch line/axis length =
  `centerline_len(L, n)` = `L + 6n` (3n at each end) — user ruling 2026-09-21 "both internal and external extend 3n at each end,
  gears too, everything stays uniform", **the internal-gear template's original `L+3n`/`L+4n` convention is void**).
- ~~Shaft segment `INVOLSPLINE`~~: removed on 2026-09-22 (the only entry = the gear generator's spline mode).

### How to Change the Drawing Method / Look Up Data

- Geometry, preset tables, code parsing, DIN 5480-2 table lookup and inspection dimensions all live in **`src/invol_spline.rs`** (the top comment holds the
  authoritative formulas, the reverse-engineering conventions and their sources); the view organisation/block names/meta of spline mode are in **`src/gear.rs`**;
  shaft segments are in `src/shaft.rs` (the XL structural-element entry and the shaft segment `INVOLSPLINE` have both been removed; `src/detail.rs` no longer has involute splines).
  Run `cargo test -p ocs_ocsm --lib invol` / `gear::` (including the whole d_B table/formula suite and the spline regressions).
- Repository copy of the DIN 5480-2 nominal table: `crates/ocs_ocsm/assets/din5480_2_nominal.csv` (721 rows,
  columns include `flags/source` for traceability); notes in `crates/ocs_ocsm/assets/din5480_2_notes.md`.
  The source OCR snapshots are in `~/桌面/OCSM/review/花键标准资料/` (including `DIN5480-2_名义表_续2_merged.csv`,
  `合并对账_续2.md` and the per-page PNGs p11–p41); the GB/T 3478 originals are in the same directory.
