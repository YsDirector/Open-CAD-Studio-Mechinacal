# 13 Task Kickoff Protocol (**do this before starting work on any drawing**)

> English translation of `handbook/13-任务启动协议.md` (the Chinese original is the source of truth).

> The position the user fixed on 2026-09-15: **unless the holes/outline are already given and only parts have to be added, follow the normal design order;
> pushed to the very front that means "ask the user for the design inputs". The knowledge is nested layer by layer.**
> This file applies to **all** OCSM tasks (part drawings, assembly drawings, annotation, BOM — all alike).

## 1. First Thing: Classify the Task (this decides which path you take)

| Type | Criterion (how to recognise it) | Which path |
| --- | --- | --- |
| **A Complete / reverse-engineer** | The drawing **already has holes/outline/plate thickness**, and the user says "add a bolt here", "match this hole", "dimension it" | Read the conditions from the **existing geometry** (hole diameter, plate thickness, contour) → back-solve the specification → run the command (`OCSMJOINT`/`XL`/`GDIM`…). **Treat the upstream constraints as assumptions — and list the assumptions** |
| **B Design from scratch** | Only functional requirements ("join these two plates", "design this housing/cylinder") | Follow the **normal design order**: **ask the user for the design inputs first** (see §2), then compute down layer by layer |
| **C Copy / convert a drawing** | There is a reference drawing / original drawing / DXF | Measure the geometry from the drawing; take the specifications from the drawing notes; **ask before changing any GB difference (drawing method/font/layer)** |
| **D Modify / revise a drawing** | An existing drawing has to be changed | Read the current state first (`ocs_read state/query`) → change only the part that was asked for → **do not overturn existing conventions that were not confirmed** |

**If the criterion is unclear, ask one question** ("is this Ø10 hole given, or may I change it?") — asking wrongly costs far less than guessing wrongly.

## 2. Type B (design from scratch): **the first step is to ask for the inputs**

Walk through the list below item by item (ask a batch at a time, do not dribble; anything the user cannot answer gets recorded as
**"assumption X (to be confirmed)"** and listed at delivery):

1. **Function and load**: what is this joint for? Which forces/torques act on it (magnitude, direction, static/alternating)? Is there internal pressure / a sealing requirement (what happens if it leaks)?
2. **Environment**: temperature range, vibration/impact, corrosion/medium, indoors or outdoors?
3. **Structure and space**: material and thickness of the joint members, **are the holes already there**, how many bolts are available and how they are arranged, wrench/assembly clearance, weight limit?
4. **Constraints and standards**: strength grade or locking requirement (is it safety-critical?), company/industry standards, **must existing stock be used**, batch size and cost?
5. **Deliverable form**: part drawing / assembly drawing / BOM / technical requirements? Sheet format and scale? Who reads it (checker / machinist)?

> These five items are the top-level inputs of the design chain (see §3): **load → preload → number/size/grade of bolts → hole diameter → bearing pressure → torque → material/heat treatment → drawing**.

## 3. The Knowledge Is **Nested Layer by Layer** (each layer: input → output → feedback)

```
① Working load          input: function/load/sealing/environment  output: the required minimum clamping force F_req
② Joint design          input: F_req + space/count               output: n × M × grade (strength check)
③ Joint-member check    input: bolt size + material + hole dia.  output: p ≤ p_G? (bearing pressure/crushing)
④ Assembly parameters   input: allowed F (the smaller of bolt/base material)  output: tightening torque T + method (scatter)
⑤ Material and heat treatment  input: the stress level from ④ + environment    output: grade ↔ heat treatment, surface treatment
⑥ Drawing               input: all decisions                     output: XY geometry + annotation + BOM + technical requirements
```

**The nesting principle (three rules, always follow them)**:

1. **Missing an upstream input → go back up and ask for it**; do not push all the way down on "the usual practice" while an input is missing.
   Example: without knowing the load, do not just report "use M8 grade 8.8" — you may give "a common default + its basis + to be confirmed".
2. **Every layer feeds back**: bearing pressure fails → add a plain washer / more holes / another material → **go back to layer ②** and re-select;
   a wide torque scatter → change the method or add margin (layer ④); a corrosive environment → go back to layer ⑤ and change the material.
3. **Mark the reverse path**: when the user only gives a lower-layer condition ("this Ø10 hole", "it must be M8") →
   say explicitly "this is the reverse path: I am assuming the upstream constraints as X", and **write the assumption into the delivery note**.

## 4. Mapping Down to OCSM Commands

| Layer | Tool / handbook |
| --- | --- |
| ①–② Load and selection | `21-knowledge-fastener-selection.md` §0/§5 (`tools/crush_check.py --allow` back-calculates the allowed F and T, and tells you who is in control) |
| ③ Bearing-pressure check | `21` §4 + `tools/crush_check.py` (forward check) |
| ④ Torque and method | `21` §5.1/§5.3 (K-value table + method scatter) |
| ⑤ Material and heat treatment | `23-knowledge-materials-heat-treatment.md` |
| ⑥ Drawing | `04-bolt-joint.md` (`OCSMJOINT`), `03-standard-parts.md`, `06-leader-annotation.md`, `10-bom.md` |

**Type A (holes already given) shortest path** (doable today; this is the reverse path):

```bash
# 1) read the conditions from the drawing: hole diameter, plate thickness, span
python3 tools/crush_check.py M8 8.8 hex_ab dh=10 mat=6061    # check bearing pressure with the actual hole diameter/material
# 2) drop it into the drawing (the part chain = the decision result)
OCSMJOINT at 50,10 rot -90 bolt=hex_bolt_b_full:8 plate=10 plate=10 \
          washer=washer_971:8 nut=nut_c41:8
# 3) state the assumptions in the delivery note (the upstream load was not given: treated as an ordinary joint; material per the drawing notes)
```

## 5. Self-Check (ask yourself before starting)

- [ ] Have I told apart A/B/C/D? (criterion unclear → ask one question first)
- [ ] Type B: did I ask all five design inputs? Is anything I did not ask recorded as an "assumption (to be confirmed)"?
- [ ] Did I push downwards on my own while an upstream input was missing?
- [ ] At delivery, did I list the assumptions, the basis and the items still to be verified?
