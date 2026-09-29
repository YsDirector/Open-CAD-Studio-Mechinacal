# 03 Standard Parts Library (`OCSMPART` / `XL`)

> English translation of `handbook/03-标准件库.md` (the Chinese original is the source of truth).

> Insert bolts/screws/nuts/washers/pins and look up GB specs. **Never hand-draw standard parts.**

## 1. Two Ways to Use It

| Usage | Command | Good for |
| --- | --- | --- |
| **Human**: open a window and pick | `XL` (= `OCSMPART`, **without arguments**) | Picking a spec while watching the preview; placing as you go |
| **AI/script**: one line, direct insert | `OCSMPART <family> <d> <l> [view <view>] [at x,y] [rot deg]` | Known spec, batch work, reproducible |

The no-argument flow (human):

```
OCSMPART            → opens the "OCSM 标准件库" window (OCSM Standard Parts) + enters placement mode
In the window: pick a part in the left tree → choose nominal diameter / length / view on the right → check the preview → click "零件出库" (Produce part)
Back in the drawing: click once for the base point → move the cursor to rotate → click again to place (repeatable; Esc to finish)
```

## 2. Syntax (AI Direct Insert)

```text
OCSMPART <family> <diameter d> <length l> [view <view>] [at x,y] [rot deg]

Example:
OCSMPART hex_bolt_c 10 95 at 150,30 rot 0
OCSMPART nut_c41 8 0 at 150,-30            ← nut/washer only takes d (write 0 in the l slot)
OCSMPART washer_971 8 0 view main at 150,-40
```

- **Family names** are the lowercase ids in the table below (the `family=diameter` form is also accepted: see `04`).
- `view`: `main` front view / `top` top view / `end` left view / `section` section (the views each family supports differ and are
  listed in the window; an unsupported view is an error).
- `at x,y`: where the base point lands (if omitted, the "point pending placement" picked in the drawing is used, otherwise the origin).
- `rot deg`: rotation about the base point (`Insert.rotation` is in radians internally; the argument is given in **degrees**).
- `l`: bolts/screws need a length; nuts/washers/pins (some) ignore it or use it per family.

### Structural Details (the second tree, next to standard parts)

The left side of the `XL` window is the "结构要素" (Structural elements) tree: **`d` is free input** (not a fixed spec dropdown). Currently on the shelf:

| Family id | Name | Command line |
| --- | --- | --- |
| `detail_grind_od` | Grinding-wheel relief groove, external grinding GB/T 6403.5-2008 | `OCSMPART detail_grind_od <d> [b1 value] [at x,y] [rot deg]` (b1 omitted = the default row for that d) |
| `detail_thread_relief` | External thread relief groove GB/T 3-1997 | `OCSMPART detail_thread_relief <d> P <pitch> [g1 v g2 v dg v r v alpha v] [at x,y] [rot deg]` (P required) |
| `detail_spline_rect` | Rectangular spline GB/T 1144-2001 | `OCSMPART detail_spline_rect <spec code N×d×D×B> L <full-tooth length> [de value] [view front/side/section] [at x,y] [rot deg]` (de is looked up for specs in the table; required for off-table specs) |
| `detail_hub_keyway` | Parallel-key hub keyway GB/T 1095-2003 | `OCSMPART detail_hub_keyway <d> [len hub length] [view main/side] [at x,y] [rot deg]` (b/t₂/r looked up from d in the 1095 table; len defaults to 30) |

> **Involute splines are generated only by the gear generator `OCSMGEAR`** (the old XL structural-element entry was removed on 2026-09-22 to prevent two entry points):
> open the window without arguments, tick "花键模式" (spline mode) at the top →
> standard number / tooth profile / reference diameter d_B + m/z/α/ha*/hf*/ρf*/cF*; the same "齿轮种类" (gear kind) switch toggles external/internal spline.
> In gear mode, giving a standard number or d_B is a clear error; under GB, d_B is not shown (the system has no such concept). Internal splines share the internal-gear convention:
> **section view + end view only, no side view** (decided by the user). See `16-gears.md` §9.

Rectangular-spline spec codes are a **dropdown** in both the `XL` window and the shaft-generator segment table (33 rows in the GB/T 1144 table,
with "自定义…" (custom…) at the end for typing an off-table spec; selecting one fills in the derived de/h/l values, and the de box can be edited = override).

The shaft generator **no longer offers** an involute-spline segment (`INVOLSPLINE` was removed on 2026-09-22; the user decided "generate only in the gear generator");
use the rectangular spline `SPLINE` for shaft splines. The involute spline is gone from the XL window too; the engine data reaches the catalog at the top level as `spline_engine`
and is used only by the gear generator's spline mode (presets / DIN 5480-2 nominal table / inspection table / modules per system).

- Thread-relief geometry follows GB/T 3-1997 figure 2: shoulder-face vertical line (to `d/2+r`) → R=r fillet → groove bottom `y=dg/2` (`x=r…g1`)
  → 30° flank to `(g2, d/2)` → 5 mm schematic thread outside diameter; `dg = d − Table 2 reduction`; base point = intersection of the shoulder face and the axis.
- Relief-groove data: Table 2 single value = pitch P (0.25…6, **no 0.2**); `g1/g2/dg/r` can be overridden; `alpha` defaults to 30° (must be ≥30°; an error is reported if the actual flank is shallower).
- Preview (structural details have no `l`; parameters ride in the query):
  `GET /api/part_svg?family=detail_thread_relief&d=20&P=1.5`, `…&b1=8` (grinding relief).
- **Hub keyway** (`detail_hub_keyway`, the hub side; the user decided "this is not a shaft-generator thing"):
  `d` (bore) → `b` (1979 GB/T 1095 d column) → `t₂`/`r` (Table 1; data reuses `assets/keyway_gb1095.csv`);
  two views: `main` = bore end face (bore arcs with the keyway opening + two walls + groove bottom + two fillets r),
  `side` = local longitudinal section (hub length × (d/2+t₂) rectangle + `√(R²−(b/2)²)` sagitta line);
  keyway opening faces +Y, centrelines protrude 3; **layers unified to the OCSM five layers** (bore circle and keyway outline/side-view outline all on `1轮廓实线层`,
  centrelines on `3中心线层`; unlike the template's `0` layer this is a **deliberate deviation** — the template's 0 layer is treated as casually unassigned);
  **no hatching, no dimensions**; the fillet defaults to that step's `r_max` (template specimen b=8→0.25; a single switch at `HUB_KEYWAY_R_PICK`).

### Parallel Keys (GB/T 1096 types A/B/C, GB/T 1097 types A/B)

Parallel-key families use the **two-parameter model**'s `d` slot to carry the **key width b** (`h` is derived from the table), and `l` for the length
(`l=0`/omitted = that step's default L). The type (A/B/C) is encoded in the family id; in the `XL` window it is a **type dropdown**
(rendered from the catalog's `type_group`; changing the type = changing the family, and the tree highlight / view / spec dropdown all follow).

| Family id | Name | Standard | Specs | Views |
| --- | --- | --- | --- | --- |
| `key_1096_a` / `key_1096_b` / `key_1096_c` | Round-end parallel key, types A/B/C | GB/T 1096-2003 | b=2…50 (20 steps) | main / top / section |
| `key_1097_a` / `key_1097_b` | Gib-head key, types A/B | GB/T 1097-2003 | b=8…45 (14 steps) | main / top (the template has no section view) |

Command line (`d` slot = b):

```text
OCSMPART key_1096_a 4 8          ← b=4, L=8 (h from the table = 4)
OCSMPART key_1096_b 22 0         ← b=22, L omitted (0 = the table default 63)
OCSMPART key_1097_a 8 25         ← b=8, L=25 (short key; L1/L2/L3 from the table = 13/12.5/6)
```

- **L convention**: the 1096 series is `6,8,10,…,400` (6/8 added from the user's material; the online L row starts at 10) plus the constraint
  `L < 10·b` (GB/T 1096-2003 note ③); the 1097 series is `25,28,…,450` (26 steps in the length table of 嘉立创's《导向平键的尺寸与公差
  (GB/T 1097-2003)》`new05232.htm`, checked cell by cell) plus the same `L < 10·b`. When the table default L is not legal,
  the nearest legal value is used (1097 b=8/10: 100 → 70/90); the dropdown lists only the legal L values for that b.
- **⚠️ `L1/L2/L3` of 1097 are derived from `L` via the GB/T 1097-2003 length-series table** (26 steps, JSON
  `partsKey1097{A,B}.json`'s `length_rows`): hole positions `±L1/2`, end distance `L3=(L−L1)/2`, break line
  `xb=−(L1/2−D/2−1)` all follow `L`, and the specimen's fixed values `L1=60/L2=50/L3=20` are **no longer used**.
  **No interpolation, no extrapolation**: an `L` outside the table (e.g. 26/27/500) is a clear error listing the available series;
  `key_1097_length_for()/key_1097_length_checked()`; the table's `L2=L/2` and `L1+2L3=L` have data guard rails.
  On the same page, `L0` (**thread depth of the fixing screw in the shaft, measured downwards from the keyway bottom** — in an assembly section that
  dimension is placed inside the shaft; corrected by the user on 2026-09-25) and the fixing screw `d×L4` (`screw` field, GB/T 822/65)
  are already in the library; `d0` is the fixing screw's nominal thread, `d1/D/h1/C1` are the holes in the key.
- **1096 chamfers are stepped by b, defaulting to the lower end of that step's range** (new decision by the user 2026-09; same convention as 1097's C taking the lower limit):
  steps = GB/T 1096-2003 Table 1 `s` row — `2~4: 0.16~0.25` | `5~8: 0.25~0.40` | `10~18: 0.40~0.60` |
  `20~32: 0.60~0.80` | `36~50: 1.00~1.20` | `56~70: 1.60~2.00` | `80~100: 2.50~3.00`;
  each row stores `c_min/c_max` in JSON, and the drawing switch point is `partgen_keys::CHAMFER_1096_PICK` (Min default / Max / Mid, one place to change).
  ⚠️ Specimen b=2 now draws **0.16** under the new convention, while the source template DXF drew 0.2 (a mid value inside 0.16~0.25)
  → a **known, deliberate 0.04 mm deviation** (deviation ④ on the evidence page); do not change it back to 0.2 to match the template.
- **⚠️ The last 4 rows' h in the 1096 type A parameter-table PNG are misprinted** (32/20/22/25; pixel-level proof that the source image misprinted them):
  the library takes the standard values **20/22/25/28** (consistent with types B/C and the online table); the printed values are kept only in JSON as `h_printed` for reference.
- **1097's fixing method = the key is fixed to the shaft with screws** (corrected by the user on 2026-09-25): the shaft has 2 fixing-screw
  threaded holes (`d0×L0`, downwards from the keyway bottom, 118° drill point); the key has matching `d1` through-holes + `D×h1` counterbores,
  with the mounting-hole spacing `L1` and the distance from the end face `L3` (both derived from `L` via the length-series table).
  **⚠️ Do not mix up the concepts**: "起键螺孔" (key-lifting screw hole) = a GB/T 1096 Annex A concept (parallel key, used to pry the key out);
  "固定螺钉孔" (fixing-screw hole) = GB/T 1097 (Gib-head key, fixing the key to the shaft). Older comments that treated the `d0/L0` columns
  as 1097 key-lifting screw holes are obsolete (`L0`'s real meaning is above).
  **The central d0 hole in the key part's front view = a through hole** (ruling by the user on 2026-09-25: the screw passes through the key body; the old
  implementation wrongly used the shaft's `L0` as the hole bottom and drew outside the key for several steps where `L0>h`); `L0` stays in the data but
  **does not take part in the key part's geometry**.
  (GB/T 1096 Annex A also gives key-lifting holes M3…M20 for parallel keys; this library does not draw them yet.)
- **User's drawing convention**: the 1097 front view = right half in section + left half in outline; **the dividing line = the `2细线层` standard 45° break line**
  (corrected by the user 2026-09: no longer copying the source drawing's `1轮廓实线层` "vertical line + small step at the end" hand artefact; amplitude 0.5 mm,
  4 segments strictly at 45°, general rule `xb = L1/2 − D/2 − 1`, see `break_1097()`); the thread minor diameter
  3/4 arc is drawn on `2细线层` (the source drawing had it on `4虚线层`); hatching is a **global 3.0 mm** (not the template's 0.794/3.175);
  **no dimensions are generated**.
- Entity-by-entity evidence against the template and the deviation list: `~/桌面/OCSM/review/平键_对模板举证.md`.

### Shaft Keyways (the KEY column of the shaft generator's segment table)

The shaft generator treats the shaft keyway as a **shaft segment type** (the same level as the thread `M` and the rectangular spline `SPLINE`, not a view-level thing): tick it in the
`OCSMSHAFT` window's **KEY column of the segment table**, then give the **key type** (table-driven single dropdown: `普通平键A型` / `普通平键B型` /
`普通平键C型` / `导向平键A型` / `导向平键B型`; the separate "导向" checkbox is **gone** — wrapped up by the user on 2026-09-25),
the **key length L** (**the key length of the selected key type**; parallel-key L series
∩ `L<10b`, dropdown candidates), and the **position centred/end**; **the key size `b×h` is determined automatically from that segment's diameter d via the first d column of GB/T 1095**
(`assets/key1096_shaft_ranges.csv`, 26 steps, **the selection basis**; h follows b) and is **shown read-only in the KEY column**
(e.g. `d25 → b8×h7 t1=4 · 槽长 18`). `t1` is looked up by b in `assets/keyway_gb1095.csv` (GB/T 1095-2003 Table 1).
An explicit override `b8h7` is only validated: it must be the standard pairing for that shaft-diameter step, otherwise a clear error is reported.

```text
OCSMSHAFT S25 E25 L40 CH2@L KEY A 18 | S30 E30 L30     ← centred A (slot length = L; d25 automatically b8×h7)
OCSMSHAFT S25 E25 L40 CH2@L KEY B 10 | S30 E30 L30     ← centred B: shows A, slot length = L_B + b = 18
OCSMSHAFT S25 E25 L40 CH2@L KEY C 14 @端 | S30 E30 L30 ← end-position C (shows C; slot length = L + t1 = 18)
```

- **Display rule (user convention 2026-09-23)**: **a regular side view (centred) always shows the type A dumbbell shape** (whatever A/B/C is selected it draws
  two round ends; the slot ends are arcs, no straight edge); **end-position B/C shows type C** (flat end facing the opening + round inner end + 40° lead-in line),
  and end-position A still shows two round ends. The display length is the converted length, so the displayed straight run can hold the selected key.
- **Automatic L conversion (the slot-end arc radius `b/2` eats into the straight run)**: the user only enters **the key length of the selected key type**; the program converts the actual slot length:

| Selected key type | Straight-run requirement | Centred (shows A) L_eff | End (B/C shows C) L_eff | Actual slot length |
|---|---|---|---|---|
| A, two round ends | `L − b` | `L` | `L` | centred = L_eff; end = L_eff + t1 |
| B, two flat ends | `L` | `L + b` | `L + b/2` | as above |
| C, one round end | `L − b/2` | `L + b/2` | `L` | as above |

  Example (template shaft d25: b8, t1=4): to use a type B key of length 10 → enter `KEY B 10`; centred gives an actual slot length of 18 (shows A with a straight run of 10),
  end position `KEY B 10 @端` gives an actual slot length of 18 (shows C with a straight run of 10). The equivalent hand-written form (old convention): centred with A enter `L_B + b`,
  end with C enter `L_B + b/2` — both forms give the same geometry.
  Regression tests: `shaft::tests::keyway_display_rules_abc_and_bc_length_conversion` +
  `shaft::tests::b_key_maps_to_ac_keyway_length` (arc-centre positions and straight runs asserted for every step of A/B/C × centred/end).
- **Key-type dropdown + table-driven (wrapped up by the user on 2026-09-25)**: key type and "导向" merged into one dropdown (backend
  `partgen_keys::KEY_STYLES` table: `id/label/key_type/guided/standard/places/allow_double`,
  handed down via `/api/parts`' top-level `key_styles`; the GUI's dropdown options, position constraints, double-slot availability and info line are all driven by it).
  **To add a new key type such as "楔形平键" (taper key): add one table row + draw branches in `shaft.rs` keyed on `key_type`/`guided`.**
  **DSL**: the new recommended form is `KEY 导向A 25` (plain keys still `KEY A 18`); **the old form `KEY A 25 导向` is obsolete but still works**
  (equivalent mapping, do not delete); the ASCII aliases `GUIDED_A`/`GUIDED_B` also work.
  **JSON**: the old schema `{"type":"A","guided":true}` is unchanged (still readable); `{"type":"导向A"}` is also accepted as an alias.
- **Optional "双槽" (double slot, v7)**: a second keyway **180° opposed** about the shaft axis (mirrored about the shaft centreline; centred and end positions both supported) —
  **shown in the section view only** (the regular side view is unchanged; the far-side slot is not drawn); section hatching is partitioned symmetrically as "upper and lower rings each with one gap",
  and the middle band between the two slots is hatched as well. Purpose convention (user 2026-09-23): **carries about 1.5× the torque of a single-key joint**.
  DSL `KEY A 18 双槽` (`DOUBLE` alias) / JSON `"double":true` / tick the KEY column in the segment table;
  Gib-head key types are disabled automatically by `allow_double=false` (table-driven).
- **Gib-head keyway (GB/T 1097, 2026-09-25)**: choose "导向平键A/B型" in the key-type dropdown (DSL recommended `KEY 导向A 25`,
  old `KEY A 25 导向` equivalent; JSON `"guided":true` / `"type":"导向A"`):
  key types are **A/B only** (1097 has no C), L comes from the 1097 length series (25…450 ∩ `L<10b`), **slot length = key length L** (a fixed key is not converted);
  **centred only** (ruling by the user on 2026-09-25: `@端` / `"place":"end"` is a clear error — the end position is the plain-key KEY convention;
  the GUI's end option is disabled);
  the slot automatically gets **2 fixing-screw threaded holes** (`d0×L0` downwards from the slot bottom, 118° drill point; hole centres `L3` from the slot ends,
  `L1/L2/L3` looked up from L in the 26-step length-series table); mutually exclusive with `双槽`. Regular side view = key body (A round end / B flat end, length L)
  + hole circles (minor diameter solid full circle + major diameter 3/4 thin arc, the library-wide convention = template conversion 265°→185°);
  section = slot bottom dips along the hole profile + upper ring of hatching along the hole / lower ring flat-topped;
  no dimensions. Template `~/桌面/OCSM/轴生成器-导向平键槽.dxf`; reverse-engineering `review/导向平键槽_几何反解.md`.
  **⚠️ Key-lifting holes are not part of 1097** (that is GB/T 1096 Annex A; only fixing-screw holes are on the slot).
- **Segment-type exclusivity (following the existing implementation style)**: KEY cannot share a segment with **GEAR / SPLINE / M / OV / RL** —
  backend DSL parsing `shaft.rs:1908`, `:2004`, JSON `shaft.rs:2575/2655/2750`, `validate` `shaft.rs:2941`
  (KEY×GEAR/SPLINE), `:2946` (KEY×M/OV/RL); GUI `shaft_gui.html:510 enforceExclusive` + `rowHtml` linked disabling
  (the same style as the thread/spline/gear segments).
- **Geometry**: in the section-view gap the chord drop = `R−√(R²−(b/2)²)` (sagitta, independent of t1), slot bottom = `R−t1`;
  a centred slot is centred on the clean cylinder "chamfer root ↔ segment end"; an end slot opens at the free end of the first/last shaft segment, and both the flat-end offset and the 40° lead-in
  x span use `t1`; the key centreline protrudes 3× the frame scale (the same convention as the axis); no dimensions are generated.
  The end face of an end-position section view closes with 3 segments = lower half `−R→slot bottom` + upper half `slot bottom→sagitta` + slot bottom/closed end wall; invariants in
  `keyway_section_end_face_closes_without_chamfer` / `keyway_section_entities_stay_inside_shaft_envelope`.
- **Data**: shaft diameter → b×h ← `assets/key1096_shaft_ranges.csv` (the 1979 old d column; the 2003 edition dropped it from its table,
  hence `keyway_gb1095.csv` has no d column and the two tables do not conflict); key type/b/h ← `partgen_keys.rs` +
  `tables/partsKey1096{A,B,C}.json`; t1 (t2 reserved) ← `assets/keyway_gb1095.csv`.
- Template (`轴生成器-普通平键.dxf`) four clusters reverse-engineered entity by entity: centred A ↔ upper-left/lower-left, end C ↔ upper-right/lower-right
  (`crates/ocs_ocsm/tests/fixtures/keyway_template_golden.csv`).

## 3. Family List (grouped by kind)

| kind | Family id | Name | Standard | Specs |
| --- | --- | --- | --- | --- |
| **bolt** | `hex_bolt_c` | Hex head bolt, grade C | GB/T 5780-2016 | 23 |
| bolt | `hex_bolt_ab` | Hex head bolt, grades A/B | GB/T 5782-2016 | 29 |
| bolt | `hex_bolt_b_full` | Hex head bolt, fully threaded (grade B) | GB/T 5783-2016 | 20 |
| bolt | `hex_bolt_hole_a` | Hex head bolt with hole in head, grade A | GB/T 32.1-2020 | 15 |
| **screw** | `socket_head` | Hexagon socket head cap screw | GB/T 70.1-2008 | 20 |
| screw | `socket_button_702` | Hexagon socket button head screw | GB/T 70.2-2015 | 8 |
| screw | `socket_torx_2671` | Hexalobular socket low head cap screw | GB/T 2671.1-2017 | 9 |
| screw | `set_screw_77` | Hexagon socket set screw with flat point | GB/T 77-2007 | 13 |
| screw | `eye_bolt_825` | Lifting eye bolt, type A | GB/T 825-1988 | 15 |
| nut | `nut_6170` | Hex nut, style 1 | GB/T 6170-2015 | 29 |
| nut | `nut_61721` | Hex thin nut | GB/T 6172.1-2016 | 29 |
| nut | `nut_c41` | Hex nut, grade C | GB/T 41-2016 | 23 |
| nut | `round_nut_812` | Round nut | GB/T 812-1988 | 48 |
| washer | `washer_971` | Plain washer, grade A | GB/T 97.1-2002 | 28 |
| washer | `washer_93` | Standard spring washer | GB/T 93-2025 | 24 |
| washer | `lock_washer_858` | Tab washer for round nut | GB/T 858-1988 | 48 (two drawing segments d≤35 / d≥13) |
| ring | `ring_893` | Circlip for holes, type A | GB/T 893-2017 | 88 |
| ring | `ring_894` | Circlip for shafts, type A | GB/T 894-2017 | 86 |
| pin | `pin_1191` | Parallel pin (unhardened steel / austenitic stainless steel) | GB/T 119.1 | 20 |
| pin | `pin_1201` | Parallel pin with internal thread | GB/T 120.1 | 10 |
| **key** | `key_1096_a` / `key_1096_b` / `key_1096_c` | Round-end parallel key, types A/B/C | GB/T 1096-2003 | 20 |
| key | `key_1097_a` / `key_1097_b` | Gib-head key, types A/B | GB/T 1097-2003 | 14 |
| bearing | `bearing_276` | Deep groove ball bearing, 60000 series | GB/T 276-2013 | 335 designations (78 bore steps) |
| bearing | `bearing_297` | Tapered roller bearing, 30000 series, size series 02 | GB/T 297-1994 | 20 (no trustworthy online source for the E column; not extended) |
| bearing | `bearing_288` | Spherical roller bearing, 20000C series | GB/T 288-1994 | 71 |
| seal | `seal_fb` | Rotary shaft lip seal, metal case with dust lip, type FB | GB/T 13871.1-2007 | 53 (uniquified by d1×D) |

> **Bolt ≠ screw** (made explicit by the user on 2026-09-16):
> * **Bolt** (kind `bolt`) = passed through a clearance hole and tightened with a nut → the "Bolts" class of the parts library, and the `OCSMJOINT` part chain recognises only this class;
> * **Screw** (kind `screw`) = has its own head, **screws directly into a threaded hole**, takes no nut → the "Screws" class of the parts library
>   (hexagon socket head cap screw GB/T 70.1, hexagon socket button head screw GB/T 70.2, hexalobular socket low head cap screw GB/T 2671.1,
>   set screw GB/T 77, lifting eye bolt GB/T 825, etc.).
> The two are **two top-level categories** in the part tree; writing a screw family into `OCSMJOINT` is rejected with the reason.

### Screw-Family Drawing / Data Notes (two families added 2026-09)

- `socket_button_702` (hexagon socket button head screw, **standard GB/T 70.2-2015**; the user's material folder is wrongly named `GB-T702`, but the code registers it as 70.2):
  views `main`/`end`; base point = head bearing face × axis; front view = flat top ring + spherical-cap arc (through `(0,±dk/2)` and `(−k,±rt)`,
  radius `r3`) + hex socket (the same formula as 70.1) + thread root 0.85d, run-out `a=2P`, end chamfer 0.075d.
  `rt/r3`: M3 = measured from the template (1.38 / 4.41827658463912); other specs `rt=e/2+w`, `r3` = the 2025 edition's r3 upper limit
  (the 2015 table has no r3; leftover see `review/内六角螺钉_校验.md`). The length series takes the 2008 handbook's commercial range ∩ the handbook's l series
  (M3 6~12 … M16 20~50).
- `socket_torx_2671` (hexalobular socket low head cap screw, GB/T 2671.1-2017): views `main`/`end`;
  head = **5° cone** (standard drawing `≤5°`) + top fillet `r`, top-face radius `rt = dk/2 − tan5°(k−r) − r/cos5°`;
  the lobular socket end view is defined by `A` (circumscribed circle `A`, valley circle `0.8·A/2`, 6 valley arcs `rs=0.1424A`, centre radius `0.5424A`);
  thread `b`=25 (d≤3)/38 (d≥3.5); when `l>b` the shank end has a length `b` of thread and a 2P run-out; the shank end in the front view is flat (no chamfer).
  The length upper bounds come from the user's table `l/2提示` (4~8 / … / 16~70); no standard length range is available outside the table (leftover see the verification report).
- Both families **have no dimensions** (generation produces only the OCSM four layers + centrelines); the entity-by-entity template regression is in `partgen_b5.rs`:
  702's two views and 2671's front view within **1e-5** (measured 2.7e-7 / 2.3e-7 / 1.5e-6), 2671's end view 5e-5
  (the template's 6 valley arcs are a source-CAD approximation; the analytic construction still uses 0.1424A/0.5424A, plus a 1e-9 analytic assertion).

> The family list and spec series are authoritative in `/api/parts` (the plugin's HTTP) or the window's left tree; the table above may grow with the implementation.
> `catalog_json` gives every family a `kind` field (bolt/screw/nut/washer/pin/ring/bearing/seal/key); grouping scripts by it is more reliable than guessing family names.

## 4. Base-Point Convention (the cause of 90% of misplacements)

| Family | Base point | Axial |
| --- | --- | --- |
| Hex head bolt / hexagon socket head cap screw / hexagon socket button head screw / hexalobular socket low head cap screw | **head bearing face × axis** | shank towards `+x` (with `rot 0` the shank points right) |
| Hexagon socket set screw with flat point | **socket-end face × axis** | the screwed-in end is at `x=0`, flat end towards `+x` |
| Lifting eye bolt, type A | **bearing face × axis** | ring centre on the `+y` side, shank along `−y` (vertical) |
| Nuts (hex/round/thin) | **end face × axis** | the nut occupies `x ∈ [0, m]` |
| Washers (plain/spring/tab) | **end-face centre** | occupies `x ∈ [0, thickness]` |
| Retaining rings (for holes/shafts) | **end-face centre (axis)** | ring plane ⊥ `x`, occupies `x ∈ [0, s]` |
| Bearings (deep groove/tapered/spherical) | **end-face centre × axis** | section: axis `y=0`, end face `x=0` |
| Lip seal type FB | **end-face centre × axis** | section: axis `x=0`, end face `y=0` |
| Pins | end face/centre (see `base_hint` in the window) | along `+x` |

> The two-parameter GUI model: bearings use "bore d + width B/T", lip seals use "shaft diameter d1 + outside diameter D" (variants with several outside diameters for one bore are selected in the length dropdown);
> see each family's `len_label`.

So `rot -90` = shank downwards (head on top), `rot 0` = shank to the right.

## 5. Block Names and Records

- Inserted block-name rule (ASCII, recognisable in the block table): `OCSM_<family>_<spec>`, e.g.
  `OCSM_HEX_BOLT_B_FULL_M8_35`, `OCSM_NUT_C41_M8`, `OCSM_WASHER_971_8`.
  `×`/`d`/`Ø` in a spec are converted to separators (`M8×35 → M8_35`).
- Every INSERT carries **XDATA `OCSM_PART`** (the family's family/spec/code/spec etc.) — the BOM and the "is this a standard part" test both rely on it.
- Inserting is **one undo entry** (`begin:零件插入` (part insert) → `commit`).

## 6. AI-Side Recipe

```jsonc
// Read the family list and specs (with kind / sizes / lengths / views)
GET http://127.0.0.1:23751/api/parts
// Preview one spec as SVG (for human checking)
GET /api/part_svg?family=hex_bolt_b_full&d=8&l=35&view=main
// Parameterised insert (recommended for AI: deterministic, reproducible)
{ "op": "execute", "request": { "op": "run", "request_id": "p1", "document_id": 1, "revision": 9,
  "cmd": "OCSMPART hex_bolt_b_full 8 35 at 50,10 rot -90" } }
// Verify: read back the INSERT and XDATA
{ "op": "read", "op": "query", "parameters": { "type": "Insert", "detail": "full" } }
```

HTTP form (usable from scripts): `POST /api/part_pick`, body = `{"family":"hex_bolt_b_full","d":8,"l":35,"view":"main","x":50,"y":10,"rotation":-90}`.

## 7. Pitfalls

1. **`l` must be in that family's table series**: out of range is an error (e.g. the `l` series of 5783 M8 is a table of steps, not arbitrary values).
   To choose a length by requirement → use `OCSMJOINT` from `04` (it picks from the supply series itself).
2. **A nut/washer `d` is the thread spec** (M8 → `d=8`), not the bore/outside diameter; their thickness comes from the GB table.
3. **Wrong view**: a washer's "front view" is the **side profile** (plain washer = rectangle, spring washer = crescent); in an assembly drawing that is the `main` view you want;
   `end` is the face-on ring. A bolt's front view = the side view with a horizontal axis.
4. **Does re-inserting create duplicate block definitions?** No: a block with the same name is reused (the name is decided by family + spec).
5. **Do not explode standard parts**: exploding loses XDATA and the block name → the BOM cannot count them and `OCSMEDIT`-style features cannot recognise them.
6. **The parts-library window is "pinned to the drawing"**: the window URL carries `tab=<drawing id>`, and "produce/insert" lands only in **the drawing that opened it**
   (several drawings each open their own window and do not cross; once a drawing is closed, the old window immediately reports "窗口对应的图纸已关闭" (the drawing for this window has been closed)). The rule see
   `06-leader-annotation.md` §5.7.

## 8. Verification Checklist

- [ ] In `query Insert` the block names look like `OCSM_*`; the base point matches expectation.
- [ ] The INSERT's records from `query` contain `OCSM_PART`.
- [ ] A single `undo` removes the part just inserted.

## 9. Involute Spline Parameter Table (GB/T 3478 data + formula layer, stage 1)

> Status: **data + formula layer landed** (`src/spline_tol.rs` + `assets/spline_gb3478_*.csv`);
> the GUI/XL command is stage 2 (not done). XL's keyway/spline entries see `00-overview.md`'s `OCSMPART XL`.

- **Target shape**: XL gains a "花键参数表" (spline parameter table) (choose internal/external → compute automatically → fill 21 attributes → insert the drawing + a report);
  the ATTDEFs of the templates `~/桌面/GB/参数表/内花键参数表GB.dxf` / `外花键参数表GB.dxf` are exactly those 21 items.
- **Only three things are hand-entered** (ruling by the user on 2026-09-25): ① tolerance grade and fit class (`6H` / `5f`-style: grades 4/5/6/7 +
  hole basis H/d/e/f/h/js/k); ② pin diameter `Dp` (optional; blank = chosen automatically by the standard R40 rule); ③ `m/z/αD` inherited from the GEAR/spline segment.
  Everything else is computed; **no interpolation or extrapolation outside the table** (a clear error is reported).
- **Tolerances (GB/T 3478.1 §8.1~8.6, p17/p18)**: `(T+λ)`, `λ=0.6√(Fp²+Fα²+Fβ²)`, `T=(T+λ)−λ`;
  `Fp=a√L+b` (`L=πmz/2`), `Fα=aφ1+b` (`φ1=m+0.0125mz`), `Fβ=a√g+b` (g = fit length, `D/2` if omitted);
  coefficients for grades 4/5/6/7: `(T+λ)=10/16/25/40·i_d + 40/64/100/160·i_E(internal) or i_S(external)`;
  `i_d=0.45∛D+0.001D` (D≤500)/`0.004D+2.1` (D>500), `i_E=0.45∛E+0.001E`, `i_S=0.45∛S−0.001S`;
  rounded to whole μm. The formula layer is in `spline_tol.rs`; Table 7 spot check `m=0.25,z=10 → (T+λ)=19/31/48/77, λ=10/14/21/32`.
- **Tables 23/24/25 (`assets/spline_gb3478_ev_sv.csv` / `_ext_dia_dev.csv` / `_limits.csv`)**:
  source = 嘉立创 FA handbook `5-3-52/50/51` (consistent with standard p18 §8.7.2~8.7.4's references); deviations are all in μm.
  Table 23: `Ev` lower deviation H=0, the d/e/f table values of `esv`, h=0, js=+(T+λ)/2, k=+(T+λ).
  Table 24: `esv/tanαD` (d/e/f × 30°/37.5°/45°), js=+(T+λ)/(2tanαD), k=+(T+λ)/tanαD;
  **footnote ①: the upper deviation of the major diameter Dee is taken as 0** (the table value applies to the minor diameter Die). Table 25: Dii uses H10/H11/H12,
  Dee uses IT10/11/12, module steps `0.25~0.75 → 10`, `1~1.75 → 11`, `2~10 → 12` (cell-for-cell consistent with the GB/T 1800 IT table).
- **Geometry (GB/T 3478.1 Table 3, p09/p10)**: `D=mz`, `Db=mzcosαD`, `E=S=0.5πm`; internal-spline major diameter
  `m(z+1.5/1.8)` (30° flat/round), `m(z+1.4)` (37.5°), `m(z+1.2)` (45°); external-spline major diameter `m(z+1)/(z+0.9)/(z+0.8)`;
  external-spline minor diameter `m(z−1.5/1.8)/(z−1.4)/(z−1.2)`; `D_Fimin`, `D_Femax` (Table 3 note 3 formula, h_s see figure 2),
  `Dii = D_Femax(H/h) + 2CF` (Table 3 note 2), `CF=0.1m` (note 4: H/h only; other fit classes change — **no column values anywhere in the 70 pages**).
- **Source of R_imin (user decision on 2026-09-27: always follow the standard)**: **the card face uses the Table 26 values** (GB/T 3478.1 §8.7.5,
  doc88 p51 = book page 50; ledger `assets/spline_gb3478_table26_rimin.csv`, lookup `rimin_table26()`);
  values **not listed in Table 26** (printed "—" in the original table, e.g. m=0.25's 30° flat/30° round/37.5°, m≥3's 45°) → **the card shows "—"**, no extrapolation.
  Figure 2's (p07 a) 30° flat / p08 b–d) round root) coefficient forms `0.2m / 0.4m / 0.3m / 0.25m` **are kept as the internal computation/report convention**
  (`rimin_coef()`/`rimin()` unchanged); when Table 26 has no value, the report's "R_imin convention" section states
  "not listed in Table 26; per figure 2's coefficient form = … (note: for internal/report comparison only, not placed on the card face)".
  A cell with a value = coef·m half-even rounded to 2 decimals (cell-for-cell consistent with Table 26; second source 嘉立创 5-3-44).
- **Pins and M/W (GB/T 3478.6/.7/.8)**: the full standard text comes from the **National Standard Full-Text Public System openstd online preview**
  (hcnos/URL see `review/花键公差_资料/花键章节_导航_嘉立创.md`, page images `3478.{6,7,8}_在线预览_p*.png`).
  `D'_Ri = Db[tanα_ci − tan(α_ci − E_max/D + invα_ci − invαD)]`, `D'_Re = Db[tan(α_ce + invα_ce + π/z − S_min/D − invαD) − tanα_ce]`,
  `α_ci/ce = acos(Db/D_ci/ce)`, `D_ci/ce=(Dee max + Dii min)/2`; **pins follow GB/T 321 R40 taking the nearest larger value**;
  `M_Ri/M_Re` even/odd tooth formulas (2)~(5)/(7)~(10), `W_min/W_max` formulas (11)(12), `K=z/6+0.5` rounded;
  inv constants `30°=0.0537515, 37.5°=0.1128285, 45°=0.2146018`. `Md` is recomputed with the entered `Dp` (user ruling ④).
- **"3 candidate Dp"**: the standard text defines only one (the larger R40 value); `dp_candidates_3()` additionally gives the 3 nearest alternatives in the series.
- **Material gaps (stated as they are)**: ① Tables 23/24/25 themselves have been captured (doc88 p49–p51, 2026-09-26) and match 嘉立创's tables;
  ② how `CF` changes for fit classes other than H/h: **no such column values anywhere in the standard's 70 pages** (p07 §5.6 and the Annex C example are both 0.1m) → this library uses 0.1m,
  and the card/report explicitly marks "standard lists no value"; ③ Table 25's Dii column was an image in the original page (checked = +IT/0).

### 9.2 Using the Smart Card `OCSMCARD` (spline parameter table; stage 4, landed)

```text
OCSMCARD 花键参数表 [std GB] 内 6H <nine-field expression> [dp 4.5] [root 平|圆] [at x,y] [rot deg]
OCSMCARD 花键参数表 外 5f SPLINE EX ...   ← external spline; fit classes d/e/f/h/js/k; expression KIND is EX
OCSMCARD                                   ← no arguments: opens the smart-card window + enters placement mode
```

- **Three elements of the command**: card type (this card = "花键参数表" (spline parameter table); UI name `GB 花键参数表`); plus the gear card, ANSI Chinese/English cards, NF internal/external cards and DIN card of 9.4/9.5/9.6/9.6.2/9.7) + system (optional `std GB`, table-driven)
  + nine-field tooth-profile expression (`MARK KIND M Z ALPHA X DA DF BETA H`, e.g.
  `SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30`; the shaft/gear generator GUI can copy it directly).
- **Expression reverse-resolve**: reuses `shaft::parse_program` to reverse-resolve `m/z/αD/x/Da/Df`; when `root` is omitted it is
  reverse-resolved from the expression's `DA/DF` (GB/T 3478.1 Table 3 only differs for the 30° flat/round formulas), then falls back to the αD default.
- **Hand-entered/source items**: ① tolerance grade and fit class (`6H`/`5f`, grades 4/5/6/7); ② pin diameter `dp` (blank = per
  GB/T 3478.6 §3.1.1 formula (1) compute `D'` then take the nearest larger R40 value; the **3 engineering alternatives** are listed in the echo,
  and `dp` can be entered by hand to recompute `Md`); ③ all parameters come from the expression (no longer reading a block in the drawing).
- **The old short command `XLT` has been removed** (name convergence): calling it reports a clear error and points to `OCSMCARD`; the `XL 花键参数表`
  subcommand is no longer offered either.
- **Drawing / layout**: built-in table blocks `OCSM_SPTABLE_GB_INT` / `..._EXT` (22 lines + 35 TEXT + 1 MTEXT + 21 ATTDEF,
  entity by entity per `~/桌面/GB/参数表/内(外)花键参数表GB.dxf`, **no external DXF dependency**); each insert fills the 21 values via
  `INSERT.attributes` (`6文字层` / `OCSM_GB`). **Main values show at most 3 decimals**; the value/tolerance column
  ATTRIB **entity-level** width factor 0.7 (without touching the global `OCSM_GB` style); the main value's right edge does not enter the tolerance column (there is a geometric regression assertion).
- **Report**: `OCSMSHAFT … REPORT` automatically adds a "## 2. 花键参数表（GB/T 3478，默认 7 级 / H·h）" section for involute-spline segments —
  **the same `spline_tol::compute()`** as `OCSMCARD 花键参数表` (not allowed to compute twice in two places).
- Units: diameters/over-pin measurements/base tangent/arc radii = mm; Ff/Fp/λ = μm; tolerance-band signs (e.g. `+0.12`/`0`).

### 9.3 Using the GUI (`OCSMCARD` smart-card window, landed)

```text
OCSMCARD        ← no arguments: opens the smart-card window + enters placement mode
```

- In the window: card type (table-driven, **11 cards switchable**; the dropdown shows common card names with the standard number, not truncated when closed; `title` falls back to the full name and convention; deep link `?card=<card-type id>`) + the GB panel's tolerance grade/fit/pressure angle/root (**the panel no longer offers "direction/internal-external" or "system" choices** — the direction is fixed by the card type, see 9.9 of this section; the CLI `std GB` is still accepted)
  → tolerance grade (4/5/6/7) + fit class (internal H; external d/e/f/h/js/k; **at 45° the preferred items H/k, H/h, H/f
  are moved to the front automatically and marked "（45° 优先）" (45° preferred)**) + pressure angle αD + root form (auto/flat/round).
- **Tooth-profile expression**: paste the nine-field expression (the hint says it can be copied from the shaft/gear generator GUI) → the window shows the reverse-resolved
  m/z/αD/x/Da/Df and the root source (hand-picked / reverse-resolved from the expression / αD default). The old "read selection / previous block"
  buttons and the `/api/spline_meta` endpoint are deleted together (no longer a mid-flow object-picking step, so the placement state is not lost).
- **Pin Dp**: the standard solution + 3 engineering alternatives are all listed as clickable chips, and a series value can also be typed (a non-series value is a backend error);
  **once Dp is picked/entered, Md is recomputed immediately with the new Dp** (the same `spline_tol::compute()`).
- **Result**: 21 items listed live; formulas/conventions/sources are in each row's `title=` hover, the always-visible text holds only operational guidance, and anomalies are reported dynamically.
- **Insert**: with `at x,y` (+ `rot`) = inserted directly; without = back in the drawing click the base point → cursor rotates → click again to place (repeatable).
  The table block geometry and 21 attributes go through **the same path** as the command line (`spline_table::build_insert`).
- **Report**: the "计算书" (report) button at the bottom of the window = a Markdown report of the same computation as the card (formulas/substitutions/results/sources;
  can be copied or have the "summary table inserted into the drawing") — usage and convention see **9.10** of this section.
- **Table-driven unified appearance**: card type/system/21-item formula conventions are handed down by `GET /api/spline_options` from the backend
  `card::CARD_TYPES` + `spline_gui::SPLINE_SYSTEMS`, and the page only renders. The spline card is a **dedicated panel**
  (the unified-appearance baseline for the other cards); the gear/ANSI/NF internal/NF external/DIN six cards **no longer each have their own HTML** — the page has only one generic skeleton,
  and the field list/control types/defaults/hints come from `CARD_TYPES[].form` (top parameter area + middle main input and readings +
  result card + bottom buttons are all in the same positions). Adding a card later = one table row + one renderer + one field list.
  The spline card uses `/api/spline_*`, the other cards use the generic `/api/card_preview|export` (dispatched from the same card-type table).
- **Layout (user screenshot review on 2026-09-26)**: on a generic card the "form → result readings" stack in one left column,
  while the "result preview" sits in the right-hand column beside the form (each column sized to its content, no big empty area left under the left form);
  the card-type dropdown takes a whole row with adaptive width, and shows the full card name with standard number when closed.
- **Expression reverse-resolve (common to five cards, 2026-09-26)**: the panels of the gear/ANSI CN/ANSI EN/NF/DIN five cards all have a
  **nine-field expression input** (textarea; the gear card already had it, the other four got it new). After pasting the
  `MARK KIND M Z ALPHA X DA DF BETA H` copied from the generator, the generic preview returns `fields` and the window **fills this card's fields automatically**
  (for the gear card the input is the expression itself, with no fields to fill back); parse failures/system·direction conflicts are computed in the backend → shown in red,
  without overwriting newer input (stale-response guard). Mapping and conflict conventions see **9.8** of this section.

### 9.4 Gear Parameter Table (`OCSMCARD` gear card, landed)

```text
OCSMCARD 齿轮参数表 <nine-field expression> [mate z₂] [dwg drawing no.] [grade accuracy grade] [center a] [at x,y] [rot deg]
OCSMCARD 齿轮参数表 GEAR EX M3 Z20 ALPHA20 X0 DA66 DF52.5 BETA0 H30 mate 20 dwg OCS-002 at 0,0
```

- **System self-consistency**: the expression's `MARK` must be `GEAR`; writing `SPLINE` is a clear error
  「表达式 MARK 写的是 SPLINE（花键），与卡片体系「GEAR（齿轮）」不一致」 (the expression's MARK says SPLINE (spline), which does not match the card system "GEAR (gear)").

- **Template**: `~/桌面/GB/参数表/齿轮参数表.dxf` (27 LINE + 46 TEXT + 18 ATTDEF, reverse-engineered entity by entity);
  block `OCSM_GEARTABLE_GB`, promoting the template's **sample TEXT** on the accuracy-grade (精度等级) row (`887FHGB10095-88`) to
  **the 19th attribute** (no hard-coded sample value; `grade` supplies the value, defaulting to "—").
- **Value sources**: reverse-resolve the nine-field expression → reverse-resolve `ha*/c*` from `DA/DF` → feed `gear::GearParams`
  (the same `d/da/df/mt/alpha_t` as the gear generator); whole depth = |da−df|/2; centre distance = `mt(z₁+z₂)/2`
  (`center` can override); base tangent `W/k` goes through `GearParams::span_measurement()` (**spur external gear**).
- **State gaps honestly** (constant `MISSING = —`): GB/T 10095-88's Fr/FW/ff/fpt/Fβ, centre-distance limit deviations,
  internal-gear/helical-gear base tangent — this repo has not collected those standard data, **no invention**.
- **Layout**: the value column is right-anchored in the template → rendered right-anchored; entity-level width factor 0.7; the header `公差(或极限偏差)值`
  width factor 0.6 (the template's 0.819 would clash with the middle column header; the value was fixed after OCS interference checking).
  The header `公差(或极限偏差)值` is the catalog's `card.gear.label.tol_value` ("Tol. (or limit)").

### 9.5 ANSI Spline Parameter Table (internal/external × Chinese/English = four cards, landed)

```text
OCSMCARD ANSI花键参数表_中文 <nine-field expression> [profile tooth profile] [at x,y] [rot deg]
OCSMCARD ANSI花键参数表_外_中文 <nine-field expression> [profile tooth profile] [at x,y]
OCSMCARD ANSI花键参数表_英文 / _外_英文  <nine-field expression> [profile tooth profile] [at x,y]
OCSMCARD ANSI花键参数表_中文 内 SPLINE IN M1.5875 Z20 ALPHA30 X0 BETA0 H30   ← old id + old "internal/external" form still accepted
OCSMCARD ANSI花键参数表_外_中文 P16 Z20                 ← old P/Z form kept
```

- **Expression reverse-resolve**: `P = 25.4/m`, `N = z`; the tooth profile takes Table 2's
  same-angle column per the expression's `α` (if the current selection has the same α it is kept, otherwise that α's default column is used: 30°→column A, 37.5°/45°→round-root flank).
  This card system requires `MARK=SPLINE`, `α∈{30/37.5/45}`, spur (β=0), **no profile shift** (`X≠0` is an error),
  and `KIND` must match the card's direction (same wording as GB: 「表达式 KIND 写的是 EX（外），与卡片方向「内」不一致」 — the expression's KIND says EX (external), which does not match the card direction "internal").
  The `花键类型` row is the catalog's `card.ansi.label.spline_type` ("SPLINE TYPE").

- **Template**: `~/桌面/GB/参数表/内(外)花键参数表ANSI.dxf` (26 LINE + 20 MTEXT + 17 ATTDEF each).
  The user's original template is **bilingual Chinese+English** (each MTEXT = Chinese + English); this card splits it by language into two versions: the Chinese-only version emits only the Chinese segment,
  the English-only version only the English segment, with positions/heights/layers/alignment per the template.
- **Two exceptions in the split** (already written into the module comments): ① the English segment of the 花键类型 row in the template is actually the sample value `FLAT ROOT SIDE FIT`,
  so the English version names the row `SPLINE TYPE` (the value comes from the attribute); ② the title `INTERANL INOLUTE`→`INTERNAL INVOLUTE` and
  `MIN MINEFFECTIVE`→`MIN EFFECTIVE` are two obvious spelling corrections; `REF`/`MIN` come out as `参考`/`最小` in the Chinese version.
- **Blocks** (4): `OCSM_ANSI_INT_CN` / `OCSM_ANSI_INT_EN` / `OCSM_ANSI_EXT_CN` / `OCSM_ANSI_EXT_EN`.
- **Value sources**: the ANSI B92.1 Table 2 branch of `invol_spline.rs` (`InvolParams::ansi(profile,P,z)`):
  diametral pitch P/Ps, tooth count N, pressure angle, Db/D, Dri/Do, Di/Dre, DFi/DFe, tooth form type (Table 2's five columns).
  The tooth profile can be written as a token (`ANSI30P`/`ANSI30PM`/`ANSI30R`/`ANSI375R`/`ANSI45R`) or a full name; default column A.
- **State gaps honestly**: ANSI B92.1's fits/tolerances (Table 4/5), pin inspection (p30–p32), base tangent/span
  are not collected in this repo → the relevant cells show "—". The `圆周齿厚` (circular tooth thickness) row has no ATTDEF in the template itself (follow the template, do not add one).
- **Layout**: value attributes entity-level width factor 0.5; the `花键类型` value squeezed to 0.32 (the English full name `FLAT ROOT MAJOR DIA FIT`
  stays inside the narrowest middle column — the value was fixed after OCS plotting + text-box geometry checks).

### 9.6 NF Internal Spline Parameter Table (one card per direction; NF internal/external two cards, landed)

```text
OCSMCARD NF内花键参数表 <nine-field expression> [中心 外径|齿面] [根 平|圆] [配合 松动|滑动|固定|压] [at x,y]
OCSMCARD NF内花键参数表 SPLINE IN M7.5 Z38 ALPHA20 X0.8 BETA0 H30
OCSMCARD NF内花键参数表 A300 M7.5 [Z38]        ← old A/M/Z form kept
```

- **Expression reverse-resolve**: `A = m(z + 0.4 + 2x)` (NF definition `x = (A − m(z+0.4))/(2m)`), `m = m`, `z = z`;
  NF's pressure angle is always 20°, direction always internal: `MARK=SPLINE`, `α=20`, `KIND` must not be `EX` (same wording as GB).
  The `A/m` reverse-resolved from the expression is still cross-checked against the p18 table row/`z`; outside the table everything is "—".

- **Why it is a mirror image**: NF E22-141 has an internal-spline dimension table (p18), inspection dimension tables (p23–p25), root fillet
  (p22/p26) and deviation tables (p35), but **no "drawing parameter table" layout**. This card follows the user's
  `~/桌面/GB/参数表/外花键参数表NF.dxf` **isomorphically mirrored**: 13 rows × 2 columns, the same outer frame/row heights/layers,
  with only the "quantities" changed — major diameter `Az`, minor diameter `D`, pin `V`, over-pin measurement `G` (NF's original symbols, paired internal/external).
- **Block geometry**: `OCSM_NFTABLE_NF_INT` = **66 LINE + 13 MTEXT (title + 12 labels) + 18 ATTDEF**
  (12 values at left/middle + 6 tolerance-hole baselines), entity by entity per the template within 1e-5. Three necessary corrections in the template:
  ① text style always `OCSM_GB` (do not copy the template's `PC_TEXTSTYLE`); ② the template's last-row label `公法线 W`
  starts at `x=−444.95`, outside the frame → moved to `x=−313.8426`; ③ the template's `\C3;` green inline code is removed (it is already on the green
  `6文字层`).
- **Input convention**: `A` (nominal diameter, NF's main parameter) and `m` **required** (the same A can correspond to different modules, e.g. A=300
  has m=7.5/N=38 and m=10/N=28); `z` optional, and if given it must match the p18 table's N, otherwise a clear error;
  centering defaults to **major-diameter centering** (`Az=A`), `齿面` → `Az=A+0.3m`; the machining method per p18 = **broaching**.
- **Value sources**: `Az/D/Do` are formula quantities (p04/p07, `D=A−2m`); `V/V1/G/G1` come from the p23–p25 inspection tables;
  `ri` (root fillet) and `x` (profile shift) come from the p21/p22 design dimension tables — these last two are given in the **preview readings/receipt**,
  not as new table rows (the external template has no such rows either). All data goes through the already-collected
  `nf_e22141_dims.csv` / `nf_e22141_check.csv` (with `source/flags` per row), **no extrapolation outside the table**:
  if not found, show "—".
- **Tolerances (p28 + p29)**: the 6 tolerance holes in the template's major-diameter/minor-diameter/over-pin rows **are all filled** —
  major diameter upper/lower deviation = ISO 286 **R7** (p28 §4: the internal spline's major diameter tolerance for broaching/major-dia. centering);
  minor diameter upper/lower deviation = ISO 286 **H7** (p28 §6: internal spline minor diameter tolerance, reference);
  over-pin upper/lower deviation = **p29 "tolerance values of inspection dimensions", internal spline E deviations** (µm→mm; the table title is the tooth base-tangent length tolerance).
  `配合类别` (fit class, p31/p34: loose/sliding/fixed/press, default fixed; **selected in the GUI form before producing the table**) only affects the **mating external spline**'s
  E/xm deviations in the preview readings (p29's four fit-class columns); p29's `xm 内花键` (internal spline xm) is also in the readings. When `(m,A)` is not in p29 or the ISO step is missing,
  the corresponding cell shows "—", with no extrapolation. Data: `assets/nf_e22141_e_xm_tol.csv` (p29's 14 rows, with source/raw/note per row)
  + `assets/nf_e22141_check.csv` (p29's two xm cells for external·loose re-checked against the original page and corrected to -88/-146).
- **Anchors (positive unit-test assertions)**: m=7.5 / A=300 / z=38 → major-dia. centering `Az=300`, flank centering `302.25`,
  `D=285`, `V=15`, `V1=12.6`, `G=G1=270.508`, `ri=1.343`, `x=0.8`; the external-spline reconciliation anchor
  (template `W=129.871` ↔ p25 row `E`) checked in the same table. Per-row standard values see
  `assets/nf_internal_card_anchor.csv` (each row has source/note).
- **Layout**: value attributes entity-level width factor 0.7 (without touching the global `OCSM_GB` style); tolerance attributes per the template 15 high / 0.667
  width factor; the geometric regression `nf_values_clear_tolerance_column` runs over **the full p18 table rows** (the value box does not leave its column,
  does not enter a tolerance cell, and does not leave the row band and outer frame).
- **Card text follows the language (③ card-face i18n batch, 2026-09-27)**: the title + 12 row labels go through the catalog
  (`card.nf.title.int` / `card.nf.label.*`), **symbols kept separate** (card face = translated name + "space + symbol"; `Az/D/Do/V/G`
  are identical in both languages; the standard number `NF E22-141`, numbers, `—`, ATTDEF tags and block names are not in the table). Three enumerated display values
  (centering / root form / machining method) follow the language (`card.nf.*`), English e.g. `Major dia. centering` /
  `Flank centering` / `Flat root` / `Broaching`.
  ★ **Why inline width codes**: the label rows are **MTEXT**, and acadrust's `MText` has **no entity-level `width_factor`**
  (only the wrap box `rectangle_width`), so the host renders with text style `OCSM_GB`'s 0.7; with the real font the English labels
  are far wider than the 146-wide label column (`Number of teeth z` 291, `Measurement over pins G` 412),
  so the ones that do not fit get **`\W<f>;`** in front of the value (effective width factor = 0.7 × f, supported by the host's MTEXT parser) —
  coordinates/height/layer/alignment/column width/style are all untouched, and the element list is still 66 lines + 13 MTEXT + 18 ATTDEF.
  On the Chinese side only the template's already-wide `基准尺寸 Do` (basic size; real font 148 > 146) is squeezed to `\W0.92`.
  The table-driven budget = `nf_table::label_inline_wf`, script `~/桌面/OCSM/review/i18n_检查/card_nf_width_plan.py`
  (real font × the repo's two conservative models, taking the tighter one, with ≥3% margin). The 12 **values** are ATTDEFs (they have an entity width factor) ⇒
  long English value text is squeezed via the entity width factor (centering 0.40).

### 9.6.2 NF External Spline Parameter Table (NF external, paired with the internal card; added from the user's screenshot on 2026-09-26)

```text
OCSMCARD NF外花键参数表 <nine-field expression> [中心 齿面|外径] [根 平|圆] [配合 松动|滑动|固定|压] [at x,y]
OCSMCARD NF外花键参数表 SPLINE EX M7.5 Z38 ALPHA20 X0.8 BETA0 H30
OCSMCARD NF外花键参数表 A300 M7.5 [Z38]        ← old A/M/Z form kept
```

- **Paired with the NF internal card, following the template as-is**: block `OCSM_NFTABLE_NF_EXT` = **66 LINE + 13 MTEXT
  (title "外花键参数表" + 12 labels) + 18 ATTDEF** (12 values at left/middle + 6 tolerance-hole baselines),
  entity by entity per `~/桌面/GB/参数表/外花键参数表NF.dxf` within 1e-5. Row names/symbols use **NF's original text**:
  `大径 Dee / 小径 Die / 跨测齿数 K / 公法线 W` (no GB symbols); the template's last-row label
  `公法线 W` originally started at `x=−444.95`, outside the frame → moved to `x=−313.8426`; text style always `OCSM_GB`.
- **Expression reverse-resolve**: the same mapping as the internal card `A=m(z+0.4+2x)`, `m`, `z`; NF pressure angle always 20°,
  **direction always external**: `MARK=SPLINE`, an explicit `KIND` can only be `EX` (writing `IN` reports 「表达式 KIND 写的是 IN（内），
  与卡片方向「外」不一致」 — the expression's KIND says IN (internal), which does not match the card direction "external"); spur, profile shift allowed.
- **Input convention**: `A`, `m` required (the same A can correspond to different modules); `z` optional, and if given it must match the p20–p22 table's N;
  centering defaults to **flank centering** (`Dee=A−0.2m`, the template example); `外径` → `Dee=A`; the root defaults to the flat root
  (`Die=A−2.4m`), `圆` → `A−2.694m`; basic size `Do=A`; machining method per the template = **hobbing**.
- **Value sources**: `K`/`W` come from the p23–p25 inspection tables (W = the E column = the K-teeth base tangent length); `Dee/Die/Do` are
  formula quantities (p04/p05/p07); `x/d/dB/s/sB/Rf/Rr/h` cross-checked against the p20–p22 rows are given in the preview readings.
  Data goes through the already-collected `nf_e22141_dims.csv` / `nf_e22141_check.csv`, **no extrapolation outside the table**: if not found, show "—".
- **Tolerances (conservative defaults, documented in `assets/nf_external_card_anchor.csv`)**:
  major diameter upper/lower deviation = ISO 286 **h12** (Ø298.5 → 0/−0.520; the template's measured 0/−0.460 for that cell was filled in separately by the template,
  and the card does not invent it back); minor diameter upper/lower deviation = ISO 286 **H7** (Ø282 → +0.052/0, consistent with the template);
  base tangent upper/lower deviation = **p29 "tolerance values of inspection dimensions", external spline E deviations** (µm→mm; four fit classes, default fixed) —
  the template example's +0.055/−0.042 is only used for reconciliation. The fit class (**selected in the GUI form before producing the table**, default fixed) only affects the base tangent tolerance, not the diameter tolerances.
- **Anchors (positive unit-test assertions)**: m=7.5 / A=300 / z=38 → `Dee=298.5`, `Die=282`, `K=6`,
  `W=129.871` (digit-for-digit consistent with the p25 row), major diameter 0/−0.520, minor diameter +0.052/0, base tangent +0.042/−0.042;
  `外径/圆齿根` → Dee=300 / Die=279.795; `A=210` (outside the table) → Dee=208.5/Die=192 still given,
  K/W/base-tangent deviations "—".
- **Layout**: value attributes entity-level width factor **0.6** (one step narrower than the internal card: even the longest value `279.795` does not enter a tolerance cell),
  tolerances per the template 15 high / 0.667 width factor; the geometric regression `nf_ext_values_clear_tolerance_column` runs over
  **the full p20–p22 table rows** + flank/round root + off-table long values; after OCS plotting there is also an independent interference check against the template DXF.
- **Card text follows the language**: the same catalog keys as the internal card (title `card.nf.title.ext`, row labels
  `card.nf.label.*`; major/minor diameter symbols `Dee`/`Die`, last two rows `跨测齿数 K` (span teeth K) / `公法线 W` (base tangent length W)),
  the compression convention same as the internal card (`\W` inline width code + 0.40 entity width factor for long English value text).

### 9.7 DIN 5480 Internal/External Spline Parameter Table (two single-column cards, split + layout fix on 2026-09-26)

```text
OCSMCARD DIN花键参数表 <nine-field expression> [N9H] [e2 …] [ae …] [as …] [tactn …] [teffn …] [at x,y]
OCSMCARD DIN花键参数表_外 <nine-field expression> [W8f] [e2 …] [ae …] [as …] [tactw …] [teffw …] [at x,y]
OCSMCARD DIN花键参数表 SPLINE IN M3 Z38 ALPHA30 X0.45 DA120 DF114 BETA0 H30 N9H W8f
OCSMCARD DIN花键参数表 M3 Z38 B120 [N9H] [W8f]          ← old form kept (old id = internal card; both N/W accepted, only the matching side is taken)
OCSMCARD DIN花键参数表 N 120×3×38×9H W 120×3×38×8f     ← §8 designations can also be given directly
```

- **One card per direction + single-column Bild 6** (user screenshot 2026-09-26): the former single "Nabe/Welle two-column combined table" is split into
  **DIN 5480 internal spline parameter table (Nabe)** and **DIN 5480 external spline parameter table (Welle)**; each draws only its own side's 13 rows,
  with column widths/width factors re-laid-out (label column 240 + value column 180 = 420 wide) — fixing the title overflow/line crossing inside the red box, the marker row leaving the right border,
  and the `齿根成形圆 d_Ff…` (root form diameter) and `槽宽 max. e_max` (space width max.) rows colliding with the neighbouring column.
- **Expression reverse-resolve**: `d_B = m(z + 1.1 + 2x)` (DIN 5480-2 nominal-table self-consistent formula), `m = m`, `z = z`;
  `MARK=SPLINE`, `α=30`, spur; **KIND must match the card's direction** (internal card `IN`, external card `EX`; the wrong side is a clear error).
  The Bild 6 example (m3/z38/dB120) is exactly the reverse-resolved result of `X0.45`.
- **Why 13 rows**: DIN's "parameter table" = DIN 5480-1:2006 **§9.1 Datenfeld / Bild 6**
  (the 2025-10 edition has it at §10.1 / Bild 6, isomorphic with only the clause number moved). Each side has 13 rows:
  designation (标记) / z / m / α / three diameters / e-s three limits / D_M / two M limits; **d_B, d, d_b, x·m, W_k
  are not in Bild 6** (they belong to the DIN 5480-2 nominal table) — this card does not stuff in GB-style 21 items, nor does it impose the GB convention.
- **Block geometry**: `OCSM_DINTABLE_DIN_INT` / `..._EXT` = each **18 LINE (15 horizontal + 3 vertical)
  + 13 MTEXT (title + 12 labels; the marker row gets no separate label) + 13 ATTDEF (marker + 12 values)**;
  text style **always `OCSM_GB`**; label/value entity-level width factors 0.75 / 0.6 (unit tests across the example/KISSsoft/gap/off-table four cases run all labels and values and check they stay inside their columns). **The whole table's INSERT is scaled 0.17** (without changing the block's internal entity geometry; ATTRIBs scale with the INSERT, and the pending-placement item carries the same scale).
- **Input convention**: `m`, `z`, `d_B` required; the internal card keeps only the hole fit (default `9H`), the external card only the shaft fit (default `8f`);
  `e2/ae/as` are shared, and tolerance overrides keep only their own side (internal `tactn/teffn`, external `tactw/teffw`).
- **Value sources (traceable cell by cell)**:
  - six diameters + `e₂=s₁`: `assets/din5480_2_nominal.csv` (exact (m,d_B,z) hit;
    not found → "—"); `α=30°`; diameter deviations per the nominal table's `A_df2/A_df1` (cutting convention; cold-rolling
    `−0,76m` is not applied).
  - `Ae`/`As`: `assets/din5480_1_table7_dev.csv` (**double-source 600 dpi OCR + cell-by-cell re-check on 2026-09-26**; 18 series × 9 diameter-step columns).
  - `Tact`/`Teff`: `assets/din5480_1_table7_tol.csv` (the 6–9 grades and module group 1,75–4 hard anchors reverse-derived from two complete examples: Bild 6 + a KISSsoft sample).
  - `D_M/M2/M1`: `assets/din5480_2_inspection.csv` (if a row exists); the Bild 6 example
    (N120×3×38×9H / W120×3×38×8f) takes the whole table's original printed values (`assets/din5480_1_bild6_anchor.csv`),
    with unit tests asserting "printed values == Table 6 formula recomputation".
  - six e/s values: DIN 5480-1 §10.8 Table 6 formulas (`emax=e₂+Ae+Tact+Teff`, `emin=e₂+Ae+Teff`,
    `evmin=e₂+Ae`, `svmax=s₁+As`, `smax=s₁+As−Teff`, `smin=s₁+As−Tact−Teff`).
- **Table 7 column-mapping confidence (a convention you must know)**: among the 9 sampled columns,
  `c4=100–200` and `c5=50–100` have **hard anchors** (Bild 6 example `As(f,120)=−28`; KISSsoft sample
  `As(f,70)=−25`); `c3=200–400`, `c6=25–50`, `c7=12–25`, `c8=≤12` are **inferred** from the header's
  400/200/100/50/25/12 step ladder (noted in the preview sources); `c1/c2` (the >400 side) and `c9` (the ≤12
  fine step) have no hard anchor — **not taken at runtime, shown as "—"**, and can be given explicitly with `ae=`/`as=`.
- **State gaps honestly**: in the scanned copy the lower part of Table 7 (8 grades × 3 module groups × 7 steps × Tact/Teff/Fp/fp/Fα/Fβ)
  has a font height of only about 2 pt, and this pass extracted only the 6–9 grade anchors; Tact/Teff for grades 10/11 and module groups other than 1,75–4,
  and D_M/M2/M1 with no inspection-table row are all "—" (the gap reasons go into the preview's `missing_note` and the receipt).
- **Anchors (positive unit-test assertions)**: example part → `N标记 = Nabe DIN 5480 – N120×3×38×9H`,
  `e_max=6.361 / e_min=6.305 / e_vmin=6.271 / s_vmax=6.243 / s_max=6.220 / s_min=6.180`,
  `D_M=5.25/6, M2max=109.266, M2min=109.169, M1max=126.017, M1min=125.956`;
  the KISSsoft part (m=3 / dB=70 / z=22 / 6f + 7H) recomputed along the formula path `evmin=5.117` etc.
  Data sources and gaps see `assets/din5480_1_table7_notes.md` and the header comments of the three CSVs.

### 9.8 Eleven-Card Expression Reverse-Resolve Convention (2026-09-26, unified)

**One single parser**: all eleven cards (GB internal/external / gear / ANSI four / NF internal/external / DIN internal/external) go through
`spline_table::parse_gear_expr(expr, card name)` → the existing `shaft::parse_program`
(nine fields `MARK KIND M Z ALPHA X DA DF BETA H`); each card only changes the error prefix, and no separate parser is written.
**Table-driven mapping**: `card_expr.rs`'s `ExprPolicy` (system/direction/pressure angle/spur/shift convention) + `ExprRule`
(field list) are given as consts by each card in its own module; the formulas are implemented in one place only (`ExprOp`), with one executor `resolve()`.

| Card | MARK | Direction (KIND) | α | β/X | expression → this card's fields |
|---|---|---|---|---|---|
| GB spline parameter table (int/ext) | must be `SPLINE` | must match the card direction (the old CLI's explicit `内`/`外` can still override) | 30/37.5/45 | spur | input is the expression; the root is reverse-resolved from `DA/DF` |
| Gear parameter table | must be `GEAR` | decided by the expression's `IN/EX` (the card covers internal/external gears) | any | helical/shift allowed | input is the expression; `ha*/c*` reverse-resolved from `DA/DF` |
| ANSI spline parameter table (int/ext × CN/EN) | must be `SPLINE` | must match the card direction | 30/37.5/45 | spur, **no shift** | `P=25.4/m`, `N=z`, profile by α |
| NF internal spline parameter table | must be `SPLINE` | must not explicitly say `EX` (the card is always internal) | 20 | spur | `A=m(z+0.4+2x)`, `m`, `z` |
| NF external spline parameter table | must be `SPLINE` | must not explicitly say `IN` (the card is always external) | 20 | spur | `A=m(z+0.4+2x)`, `m`, `z` |
| DIN internal spline parameter table | must be `SPLINE` | must not explicitly say `EX` (the card is always internal) | 30 | spur | `d_B=m(z+1.1+2x)`, `m`, `z` |
| DIN external spline parameter table | must be `SPLINE` | must not explicitly say `IN` (the card is always external) | 30 | spur | `d_B=m(z+1.1+2x)`, `m`, `z` |

- **Conflict wordings (equally specific)**:
  - direction: `ANSI 花键参数表：表达式 KIND 写的是 EX（外），与卡片方向「内」不一致`
    (the same sentence pattern as the GB spline card); NF internal/external likewise (the two cards are respectively always internal/always external).
  - system: `…：表达式 MARK 写的是 GEAR（齿轮），与卡片体系「SPLINE（花键）」不一致` (the expression's MARK says GEAR (gear), which does not match the card system "SPLINE (spline)").
  - pressure angle: `…：表达式压力角 20° 不在卡片体系允许的 α=30/37.5/45°（表达式与卡片体系不一致）` (the expression's pressure angle 20° is not one of the card system's allowed α=30/37.5/45° — expression and card system disagree).
  - shift/helix: `…：表达式径向变位 X=0.4 在卡片里没有对应输入（该体系不收变位；请用 X0 的表达式）` (the expression's radial shift X=0.4 has no matching input in the card — this system takes no shift; use an X0 expression).
- **CLI expression forms**: the ANSI/NF/DIN three cards accept a nine-field expression directly in the direction/main-parameter position; the expression is cut from
  `GEAR`/`SPLINE` up to the "this card's option keyword" (a short expression missing `DA/DF/BETA/H` also works); the old
  `P16 Z20` / `A300 M7.5` / `M3 Z38 B120` forms are all kept; giving both and having them disagree reports 「显式…与表达式反解…
  不一致」 (explicit … disagrees with the expression reverse-resolved …).
- **GUI fill-back**: after a successful generic preview, the card's controls are filled from the backend `fields` (only when the echoed `expr` matches, to prevent crosstalk;
  it does not steal a control being typed into; setting the value directly dispatches no `input` event, so there is no preview storm). `#status`'s red-box red-text
  priority follows `ocsm_gui_common.js`, unchanged.

### 9.9 One Card per Direction + Preview/Scale Convention (2026-09-26, the user's eight points)

- **One card per direction** (user: "the way internal/external spline switching works is inconsistent between the standards, please unify it"):
  **switching internal/external = choosing a card**, and the panel no longer offers a direction choice:
  `GB 花键参数表（内/外）` · `ANSI 花键参数表（内·中/外·中/内·英/外·英）` ·
  `NF E22-141 内/外花键参数表` · `DIN 5480 内/外花键参数表` ·
  `GB 花键参数表（内/外·精简版）` (expanded to **22 cards** on 2026-09-27, of which the "精简版" (lite) group has 11, see 9.11/9.12).
  The old `id` points at the default-direction card (old ids for GB/ANSI CN-EN/DIN = internal); the old CLI's explicit `内`/`外` can still override,
  deep links `?card=old id` still work; the expression's `KIND` is validated against each card's fixed direction (a wrong side still reports an error, keeping that capability).
- **No duplicated units in previews (general rule)**: `ocsm_gui_common.js::ocsmUnitSuffix(value, unit)` — when the value already carries a
  unit/symbol (e.g. `20°` with `°`, `FLAT ROOT SIDE FIT` with an empty unit) or is the placeholder "—", no unit is appended;
  all cards share one implementation (no more `30° °`).
- **The GB card no longer shows "system"**: the GUI field is deleted (the system is no longer an input; a `system` passed by old requests is ignored by serde),
  while the CLI `std GB` is still accepted (compatibility); other systems can still be added table-driven.
- **Two width-factor reductions on the gear card** (entity-level, without touching the global `OCSM_GB`): `齿轮副中心距及其极限偏差` (centre distance and limits),
  `检验项目代号` (inspection code) 0.667 → **0.55** (marked on an OCS screenshot).
- **NF tolerance shift right**: the last pair of tolerances (internal = over pins, external = base tangent) has the ATTDEF insertion point shifted right by **2 character widths**
  (= `2 × 15 × 0.667 = 20.01`, the same convention as the GB card's `TOL_X_SHIFT`); the geometric regression follows.
- **NF / DIN whole-table scale 0.17**: use an **INSERT scale** (`set_x/y/z_scale(0.17)`) — **without changing the block's internal entity geometry**;
  attribute text scales with the INSERT transform, and the GUI's pending-placement item carries the same scale; after scaling the interference check is re-run (uniform scaling,
  relative geometry unchanged; DIN's single-column width factors/column widths are re-stepped at the new scale).

### 9.10 Report (GUI "计算书" Button; Same Source as the Card, 2026-09-27)

- **Entry**: the "计算书" (report) button at the bottom of the window → `POST /api/card_report` (body = the current card model, the same shape as
  `/api/card_preview`). The window always shows a Markdown panel (`#reportMd`): you can "copy" the text,
  or "insert the summary table into the drawing" = insert this card via the existing `/api/spline_export|card_export` with **the same model**
  (the summary table = this card parameter table's key items, with no second table-drawing implementation); "close" only collapses the panel and does not affect the other buttons.
- **Same-source iron rule**: paragraph 1 "card items" comes verbatim from the same entry as `/api/card_preview`
  (`spline_tol::compute()` / each card's `values()`/`spec()`), matching the card's ATTRIBs item by item;
  paragraph 2 "engine report" calls `gear::build_report` (gear) / `invol_spline::build_report_card`
  (ANSI/NF/DIN) directly. The test `card_report::report_and_card_attdefs_share_one_source` asserts per card that
  "every card ATTRIB value appears in the report text". The card has no "effective length" input → the report does not print that row
  (`build_report_card`); errors always go through the shared `ocsmStatus(..., "bad")` red box, and the wording comes through verbatim.
- **Content**: input/reverse-resolve → parameter table (each item with its formula/convention and source) → derivation of the inspection quantities → missing-item notes.
- **ANSI Wn/Kn** (the ANSI B92.1 original inspection tables are not collected in this repo): the report gives an **involute-geometry derivation**, not passing it off as original table values —
  `Kn = round(N·α/180° + 0.5)`; `Wn = m·cosα·[(Kn−0.5)π + N·invα] + 2·x·m·sinα`;
  the card's 17 items' "公法线长度 / 跨测齿数" still show "—".
- **NF K/W**: `K` takes the table value from the NF E22-141 p23–p25 inspection tables (the standard gives it by N step; this repo has no separate formula → no invention);
  `W` (= the inspection table's `E` column) is substituted into the formula noted in the collected CSV's header
  `W = m·cos20°·[(K−0.5)π + N·inv20°] + 2·x·m·sin20°`, and the residual against the table value is given;
  outside the table (no p23–p25 hit) K/W are always "—".
- **One implementation of the formula**: `invol_spline::involute_span(m,z,α,x)` (the ANSI/NF derivation is in tune with the gear
  `GearParams::span_measurement`, no computing twice in two places).

### 9.11 GB Spline Parameter Table · Lite Version (internal/external two cards; ordered by the user on 2026-09-27)

- **Convention (the user's words)**: "no accuracy, only basic information" — the card face **lists only the governing standard, module, number of teeth, pressure angle,
  root form, major diameter, minor diameter + the main measured quantities** (internal: pin Dp, over-pin Md; external: span teeth Kn, base tangent Wn);
  **no tolerance column at all (no upper/lower deviations shown)**. For tolerance/grade detail use "GB 花键参数表（内/外）".
- **Purpose**: when you do not want the whole big table in the drawing; the dropdown puts it in the **"精简版" (lite) group** (`card_types[].group`;
  `GB花键精简表_内` / `GB花键精简表_外`, one card per direction; deep link `?card=` works).
- **Inputs (unified skeleton)**: `expr` (expression reverse-resolve, reusing the same parser) + `grade`/`fit` (only affects
  the computation convention of Dp/Md or Kn/Wn, not shown on the card face; the internal card accepts only H, the external card h/js/k/d/e/f) + `root` (auto/flat/round);
  the internal card has one extra `dp` (blank = automatic R40 choice).
- **Block geometry**: `OCSM_SPLITE_GB_INT` / `..._EXT` (**14 lines + 10 texts + 9 attributes**, no MTEXT;
  reusing the GB template frame coordinates/row height 6.89872/line layers and the `OCSM_GB` style; **the whole tolerance column is removed** — the right frame closes at the value column's right edge),
  INSERT scale = **1** (a compact table needs no whole-table scaling; unlike NF/DIN's 0.17).
- **Same source**: values come from **the same** `spline_tol::compute()` as the GB card (not a second computation); the report
  (`/api/card_report`) emits a "lite items (9 items)" section, matching the card's ATTRIBs item by item; table-driven: adding a system =
  one more field list + one `CARD_TYPES` row + one dispatch branch.

### 9.12 Lite Version Across Systems (NF / DIN / ANSI×4 / gear, 9 cards in total; ordered by the user on 2026-09-27)

- **Convention**: same meaning as the GB lite version — **only basic parameters + main measured quantities, the whole tolerance column removed**; one card per direction;
  the dropdown puts them all in the **"精简版" (lite) group** (`<optgroup>`); card ids: `NF花键精简表_内/外`,
  `DIN花键精简表_内/外`, `ANSI花键精简表_内/外_中文`, `ANSI花键精简表_内/外_英文`, `齿轮精简表`.
- **Content (each system's own convention, not copied from GB)**:
  * NF internal = governing standard · centering · m · z · a · root form · machining method · major dia. Az · minor dia. D + pin V / over pins G (11 items);
  * NF external = the same 9 items + span teeth K / base tangent W (11 items);
  * DIN internal/external = designation · z · m · α + root circle/root form circle/tip circle (external card: tip circle/root form circle/root circle) +
    measuring circle D_M + measuring-circle distances M2/M1 limits (10 items);
  * ANSI internal/external · CN/EN = type · z · P · α · base circle · pitch circle · major dia. · minor dia. + main measured quantities (internal M/Dp, external W/K; 10 items);
  * gear = m · z · α · shift + pitch circle/tip circle/root circle + base tangent W / span teeth K (9 items).
- **Table-driven implementation**: shared engine `card_lite.rs` — `LITE_CARDS` 9 rows (block name/title/full-card id/field list);
  values/validation/expression reverse-resolve are **all projected from the matching full card** (delegating to the same `preview_json` → picking items by key);
  the gear's three circles are computed via the same `GearParams::d()/da()/df()`. Adding a card = one row + a field list, without copying computation code.
- **Block geometry**: `OCSM_LITE_*` (the same skeleton as the GB lite version: row height 6.89872, `OCSM_GB`, no MTEXT, no tolerance column),
  row count follows the field count; INSERT scale = **1**.
- **Same source/convention**: the report (`/api/card_report`) emits a "lite items (N items; the same computation as the full card)" section;
  a table lookup miss → "—" **with no extrapolation**; full detail still uses the matching full card.
- **Card text follows the language (③ card-face i18n batch, 2026-09-27)**: the NF lite cards' titles and 11 row labels go through the catalog
  (title `card.nf.lite.title.*`, row labels **reusing the full card's** `card.nf.label.*` + `label_symbol(family)` to assemble
  the symbol — NF major/minor diameter symbols **follow the direction** `Az`/`Dee`, `D`/`Die`); for English entity width factors see
  `en_label_width_factor()` (budget script `i18n_检查/card_nf_width_plan.py`),
  long English value text (`Major dia. centering`) also uses `value_wf()` to squeeze to 0.64. Lite-card labels are **TEXT**
  (they have an entity width factor, so the NF full card's `\W` inline code is not needed).
- **GB card external-spline pin/over-pin panel (2026-09-27)**: the GB external card's 21 items are unchanged (template-faithful), but the GUI panel
  **can** use D_Re/M_Re (GB/T 3478.6 §3.2; standard solution + 3 alternatives + manual entry), with the title marked
  "**公法线为主、跨棒距备用**" (base tangent primary, over pins as backup); M_Re and its limits appear only in the panel (the report does not pile up the same quantity again).
