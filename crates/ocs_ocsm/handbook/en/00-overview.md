# 00 Overview: Command Index + Task Index

> English translation of `handbook/00-总览.md` (the Chinese original is the source of truth).

> This file is the first stop for every drawing task. Read the command index → follow the "Task index" into the matching topic file.

>
> **Step 0 (do this first)**: classify the task — **holes/outline already given, only add parts** (reverse-engineering) or **design from scratch** (ask for design inputs first).
> See `13-task-kickoff-protocol.md`. **Do not force your way through when the upstream inputs are missing**; the knowledge is nested layer by layer (load → selection → bearing pressure → torque → material → drawing).

## 1. Command Index (all OCSM commands)

Database format: `command` (alias) — purpose — type (**Window** = opens a browser GUI; **Interactive** = pick in the drawing; **Direct** = one line of arguments writes straight to the drawing)

| Command | Alias | Purpose | Type |
| --- | --- | --- | --- |
| `OCSM` | — | **Initialize**: create the 10 layers/linetypes/text style `OCSM_GB`/dimension style `OCSM_GB` (and set them current); also opens the parts library window. **Runs automatically on a new empty drawing** (old drawings / drawings with content are not touched automatically; the three criteria are in `01`) | Direct write + Window |
| `1`…`9`, `10` | — | Number keys switch layers; **when objects are selected**, move the selected objects to that layer (the current layer does not change) | Direct write |
| `TF` | `OCSMFRAMEINIT` | Open the **frame picker window** (lists `frame/*.dwg` in the plugin directory); **with args = insert in one line**: `TF <name> [scale] [at x,y] [rot deg]` (for AI/automation; no window is opened) | Window/Direct |
| `OCSMFRAMEINSERT` | — | Insert the frame chosen in the previous step at the chosen scale (cursor-follow, scale-aware dimension styles); args also work for one-line insert (same as `TF`) | Interactive/Direct |
| `OCSMPART` | `XL` | **Standard part / structural detail insertion** (ribbon "Standard parts" group, "Parts library" button): no args = open the parts library window (left "Standard parts" tree + right "Structural details" tree) + enter placement mode; with args = insert in one line (standard part `XL family d l`; structural detail `XL detail_thread_relief 20 P 1.5`) | Window/Interactive/Direct |
| `OCSMJOINT` | — | **Bolt joint assembly** (part chain: plate/washer/nut; computes length, occlusion clipping, single undo): no args = open the assembly window + placement mode; with args = assemble in one line | Window/Interactive/Direct |
| `OCSMPOWERDIM` | `D` | **Smart dimension**: point-pick mode for linear/aligned/radius/diameter (Enter switches to segment picking) | Interactive |
| `OCSMDIMGULIDE` | `GDIM` | **Leader annotation**: pick a leader first → open the annotation settings window (dimension/section/auxiliary view/detail/angle/arc length/weld/leader/item number/tolerance/geometric tolerance) | Window |
| Item numbers (balloons), horizontal/vertical layout | `GDIM` type "item number" | `14-item-numbers.md` | Window |
| `OCSMEDIT` | `ME` | **Edit annotation**: select an OCSM-generated annotation → change parameters in the same settings window → regenerate (replace) | Window |
| `OCSMRGH` | `CC` | **Surface roughness**: interactively pick the insertion point → roughness settings window (anonymous block + ATTDEF) | Interactive + Window |
| `OCSMDIM2GB` | `D2G` | **One-click GB conversion**: convert native dimensions (or the selected ones) to OCSM_GB style + anonymous blocks; **smart center marks are also converted to `3中心线层` centerlines** (Ø + n×6) | Direct |
| `OCSMCENTERLINE` | `ZX` | **Centerlines**: click a circle/arc → cross; click two lines → angle bisector (length = diameter/projected length + n×6, on `3中心线层`) | Interactive/Direct |
| `OCSMGEAR` | — | **Gear** (external gear / **internal gear ring**): no args = open the **gear window** (switch external/internal gear; view buttons + parameter table + live preview) + placement mode; with args = insert in one line (`OCSMGEAR 2 40 20 view 剖视图`, `OCSMGEAR int 2 40 30 view 剖视图`) | Window/Interactive/Direct |
| `OCSMSHAFT` | — | **Shaft generator** (ribbon "Shaft" group, "Shaft generator" button): no args = open the **shaft generator window** (segment table ↔ line text two-way sync + live preview) + placement mode; with args = insert in one line (line DSL/JSON: segment concatenation + end chamfers + grinding relief grooves + thread segments `M` (with `TL/RO/SD/RL` partial threads/relief grooves) + segment-level relief grooves `RL@L/@R` + gear segments `GEAR` + parallel-key shaft keyways `KEY` (**shaft segment type**, fills the KEY column of the segment table: key type single dropdown driven by the key-type table (parallel keys A/B/C + guide keys A/B) + key length L + position centered/end; b×h determined from GB/T 1095 by the segment diameter d; slot length converted automatically by type; optional `双槽` = two slots 180° apart, section view only, about 1.5× single-key torque; choosing "guide key" = GB/T 1097 (DSL `KEY 导向A 25`, old `KEY A 25 导向` still accepted): only A/B, centered only, L from the 1097 series, slot length = key length, two fixing-screw threaded holes d0×L0 added above the slot automatically (mutually exclusive with double slot)) | Window/Interactive/Direct |
| `OCSMCARD` | — | **Smart card** (general card generator; "Cards" toolbar group button): **22 cards (one card per direction; the panel no longer selects internal/external or system; the dropdown "精简版" group holds 11 cards)**: `GB 花键参数表（内/外）`, `GB 花键参数表（内/外·精简版，dropdown "精简版" group）` / `齿轮参数表（GB/T 10095）` / `ANSI 花键参数表（内·中/外·中/内·英/外·英）` / `NF E22-141 内/外花键参数表` / `DIN 5480 内/外花键参数表`; card names carry the standard number and direction and are visible in the closed dropdown): no args = open the **GUI** (card type switch; GB spline card: tolerance grade·fit + pin Dp standard solution/3 alternatives with point-pick and manual entry + nine-field tooth-profile expression back-solving for m/z/αD/x/Da/Df + 21 live results (the system field was removed; CLI `std GB` still accepted); gear card: nine-field expression + mating teeth/drawing number/accuracy grade/center distance + 19 items; ANSI cards (four): tooth profile + P/N (or nine-field expression back-solving) + 17 items; NF internal card: A/m/z (or expression back-solving) + centering/root/fit + 18 items (tolerance = p28 R7/H7 + p29 E); NF external card: A/m/z (or expression back-solving) + centering/root/fit + 18 items (template original: Dee/Die/K/W; tolerance = h12/H7 + p29 external spline E); DIN cards (internal/external): M/z/d_B (or expression back-solving) + this-side fit + 13 items (Bild 6 single column, whole-table INSERT scale **0.17**); GB lite cards (internal/external): nine-field expression + grade/fit (affects measuring values only) + root → 9 items (basic parameters + Dp/Md or Kn/Wn, **no tolerance columns**, scale 1); lite cards for the other systems (NF/DIN/ANSI×4/gear, 9 cards, sharing the `card_lite` engine: values/formulas are all projected from the matching full card, see handbook 03 §9.12); card types/definition tables are delivered by the backend) + placement mode; with args: `OCSMCARD 花键参数表 [std GB] <nine-field expression> [内 6H\|外 5f] [dp 4.5] [root 平\|圆] [at x,y]` (external card `花键参数表_外`; old `内/外` token still accepted) / `OCSMCARD 齿轮参数表 <nine-field expression> [mate z₂] [dwg drawing no.] [grade accuracy grade] [center a] [at x,y]` / `OCSMCARD ANSI花键参数表_中文(或_外_中文/…_英文/…) <nine-field expression> [profile code] [at x,y]` (old `内 P16 Z20` accepted) / `OCSMCARD NF内花键参数表 <nine-field expression> (or A300 M7.5 [Z38]) [中心 外径\|齿面] [根 平\|圆] [配合 松动\|滑动\|固定\|压] [at x,y]` / `OCSMCARD NF外花键参数表 <nine-field expression> (or A300 M7.5 [Z38]) [中心 齿面\|外径] [根 平\|圆] [配合 …] [at x,y]` / `OCSMCARD DIN花键参数表(_外) <nine-field expression> (or M3 Z38 B120) [N9H\|W8f] [ae …] [as …] [at x,y]` → attributes computed → table inserted (table-block geometry is built in; value/tolerance columns use condensed solid-font width; items not collected — such as GB/T 10095-88 / ANSI Table 4/5 / NF card tolerances being p28+p29 real values (outside the table still "—") / DIN Table 7 >400 side and tolerance-table-body gaps — show "—") + `OCSMSHAFT REPORT` spline-section sync; the window's "Report" button = **same-source Markdown** (card items + engine formula → substituted → result; ANSI Wn/Kn derived from involute geometry, NF K/W table values + formula reconciliation), optional "drop summary table onto drawing" = this card's parameter table (the same computation); **old short command `XLT` was removed** (its error points the way); GUI usage see handbook 03 §9.2–9.10 | Window/Interactive/Direct |
| `OCSMHOLE` | `DK` | **Hole generator** (ribbon "Hole" group, "Hole generator" button): no args = open the **hole generator window** (simple/threaded/counterbore/countersink; blind/through; counterbore/countersink may be threaded; when unthreaded the subtype = drill size/custom/bolt clearance) + placement mode; with args = insert in one line. Automatic thread length = 1.5d, automatic hole depth = effective depth + 2P (threaded blind holes only); through holes have no 118° cone | Window/Interactive/Direct |
| `OCSMBOM` | `BOM` | **BOM**: build table / refresh (`BOM 30` = 30 rows in the first column this time) | Direct |
| `OCSMBOMSYNC` | `BOMSYNC` | **BOM balloon sync**: reorder/rebuild from item balloons (the only coupling between balloons and table = the item number) | Direct |
| `OCSMBOMLOCK` | `BOMLOCK` | **Lock quantity** (`BOMLOCK 5 3` locks to 3, `BOMLOCK 5` locks at the current value, `BOMLOCK 5 off` unlocks) | Direct |
| `OCSMBOMXLSX` | `BOMXLSX` | **Export BOM to xlsx** (with a "lock quantity" column; default `<drawing name>-明细表.xlsx`) | Direct |
| `OCSMBOMXLSXI` | `BOMXLSXI` | **Import BOM from xlsx/csv** (non-empty cells overwrite, blanks are kept, manually changed quantities are locked automatically) | Direct |
| `OCSMBOMCFG` | `BOMCFG` | **BOM configuration** (headers/columns/format) | Direct |
| `OCSMMCP` | — | Print MCP/HTTP access info (for external AI/scripts) | No drawing write |

> Command names are **case-insensitive**; `rest` (the argument part) is forwarded verbatim.

## 2. Task Index (what to do → which file to read)

| I want to… | Read in order |
| --- | --- |
| Draw an A3 part drawing from scratch | `01` initialization → `02` sheet (`TF a3_landscape 1:1 at 0,0` one-line insert) → `05` smart dimension (or `06` leader annotation) → `08` tolerance/roughness |
| Draw an assembly drawing from scratch (with a bolt joint) | **`13` task kick-off** (ask for design inputs first) → `01` → `02` → `21` selection/check → `03`/`04` placement → `06` dimensioning → `10` BOM |
| Holes already on the drawing; only add bolts/dimensions | `13` §1 class A (reverse-engineering) → `03`/`04` (size inferred from the existing geometry) |
| The user only says "connect these two plates" | `13` §2 **ask for design inputs first** → `21` design chain |
| Insert an M8×35 bolt with nut and washer | `03`; assemble as a joint → `04` |
| Insert a socket screw (cylindrical head GB/T 70.1 / button head GB/T 70.2 / low head with hexalobular socket GB/T 2671.1) | `03` (a screw ≠ a bolt: no nut, not handled by `OCSMJOINT`) |
| Work out what bolt length two 10 mm plates need | `04` (length rules) + `21` (selection) |
| Decide whether to add plain washer/spring washer/double nut | `21` knowledge — fastener selection and locking (the command does not decide for you) |
| Will this aluminium part / thin plate be crushed by the bolt? | `21` §4 (can be computed: criterion + p_G table + `manual/tools/crush_check.py`) |
| What tightening torque? Is the bolt or the base material limiting the torque? | `21` §5 (`crush_check.py --allow` back-solves F/T and identifies the controlling side) |
| How to design a threaded connection from scratch (load → preload → torque → material)? | `21` §0 design order |
| Dimension a hole's diameter + tolerance + roughness | `06` (diameter leader) → `08` (tolerance/fit/GDT) |
| Existing native dimensions should be unified to OCSM style | `07` D2G (also converts **smart center marks** into centerlines) |
| Draw a gear (section/side/simplified/regular front view) | `16` gears (`OCSMGEAR`; external gear: spur/helical/profile-shifted; placement same as `XL`) |
| Draw an **internal gear (ring)** | `16` gears §4.2 (`OCSMGEAR int 2 40 30 view 剖视图`; only section view + end view; the section view is not hatched and the ring's outer wall is left for the user to extend) |
| Draw a shaft (shoulders/chamfers/grinding relief grooves/threads/gear segments/parallel-key keyways) | `OCSMSHAFT` (no args opens the shaft generator window: segment table ↔ line text two-way sync + live preview; relief grooves via segment-level `RL@L/@R` or a short small-diameter segment; gear segments via `GEAR`) |
| Draw a hole (simple/threaded/counterbore/countersink; blind/through; threaded or not) | `17` hole generator (`OCSMHOLE`/`DK`; automatic thread length 1.5d, automatic depth +2P; through holes have no 118° cone) |
| Add centerlines / symmetry centerlines | `15` centerlines (`ZX`; circle→cross, two lines→angle bisector) |
| Produce a BOM | `10` (only parts carrying XDATA from `03`/`04` have content) |
| I want to drive OCS drawing from scripts/AI | `11` automation API (**must read**) + `12` troubleshooting |
| Add Chinese/English entries for the GUI/commands/card faces (bilingual) | `24` bilingual copy and new entries (mechanism + new-entry checklist + `OCSMLANG` switching) |
| A command does not respond / revision error / a window does not open | `12` troubleshooting |

## 3. Global Hard Rules (excerpt)

- Units **mm**, model space **1:1**; scale lives only in the frame and dimension styles.
- Layer/colour/linetype/lineweight: use OCSM's 10 layers, **always ByLayer**.
- Every drawing change must leave an undo point; batch drawing writes use one transaction = one undo entry.
- Leaders (`10引导线层`) are **annotation intent**, not printed; commands generate annotations from the leaders.
- Do not draw standard parts, bolt joints or annotations by hand — use the commands.
- AI side: before touching the drawing, **re-read `state` every time** to get `document_id`/`revision`; check `selection_revision` for the selection.

## 4. Layer List (produced by `OCSM` initialization)

| Key | Layer | Linetype | Contents |
| --- | --- | --- | --- |
| 1 | `1轮廓实线层` | Continuous | Visible outlines, thread crests (thick continuous lines) |
| 2 | `2细线层` | Continuous | Thread roots, thin lines outside hatch boundaries, break lines |
| 3 | `3中心线层` | CENTER2 | Axes, symmetry centerlines |
| 4 | `4虚线层` | DASHED2 | Hidden outlines (dashed lines) |
| 5 | `5剖面线层` | Continuous | Hatch section lines (ANSI31 etc.) |
| 6 | `6文字层` | Continuous | Text / technical requirements |
| 7 | `7标注层` | Continuous | Dimensions (including the output of D and GDIM) |
| 8 | `8符号标注层` | Continuous | Roughness, weld, geometric tolerance and other symbols |
| 9 | `9双点划线层` | DIVIDE2 | Phantom projections, extreme positions, outlines of moving parts |
| 10 | `10引导线层` | Continuous | **Leaders (annotation intent), off by default, not printed** |

## 5. Knowledge Ladder (nested: the previous layer's output = the next layer's input)

| Layer | Content | File |
| --- | --- | --- |
| 0 | Task classification + asking for design inputs | `13-task-kickoff-protocol.md` |
| 1 | Service loads (gearbox radial/axial, cylinder seal pressure) | `21` §5.2 (skeleton, detailed calculation pending) |
| 2 | Joint selection (count×size×grade, strength) | `21` §0/§5 |
| 3 | Bearing-pressure check of the clamped parts (p ≤ p_G) | `21` §4 + `tools/crush_check.py` |
| 4 | Assembly parameters (torque, method, scatter) | `21` §5 |
| 5 | Materials and heat treatment | `23-knowledge-materials-heat-treatment.md` |
| 6 | Drawing (drafting/dimensioning/BOM) | `20` GB drafting, `04`/`06`/`10` |
| — | Tolerances and fits (cross-cutting: relevant to layers 2, 3 and 5) | `22-knowledge-tolerances-fits.md` |

## 6. Next

- New practices / pitfalls hit → write them into the matching topic file and add one line to the routing table in `SKILL.md`.
- Human side: the same content can be opened inside OCS with `OCSMHELP` (the window reads this handbook's md directly).
