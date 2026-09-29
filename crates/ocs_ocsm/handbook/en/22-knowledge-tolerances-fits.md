# 22 Knowledge: Tolerances and Fits (Skeleton)

> English translation of `handbook/22-知识-公差与配合.md` (the Chinese original is the source of truth).

> Choosing fits, looking up deviations, writing them into annotations. The plugin has **ISO 286**'s 4 tables built in (hole/shaft × common tolerance bands); just pick a code in the settings window.

## 1. Basic Concepts

| Term | Meaning |
| --- | --- |
| Basic size | design size (e.g. Ø30) |
| Limit sizes | upper / lower limit (the largest/smallest allowed) |
| Deviation | upper deviation ES/es, lower deviation EI/ei (uppercase for holes, lowercase for shafts) |
| Tolerance | upper deviation − lower deviation (depends only on the tolerance grade) |
| Fundamental deviation | the deviation nearer the zero line (decided by the code, e.g. for `H` the lower deviation = 0, for `h` the upper deviation = 0) |
| Tolerance grade | IT01…IT18; the bigger the number the looser (IT6–IT9 are common for fit sizes) |
| Fit | clearance fit (H/g, H/f…), transition (H/js, H/k), interference (H/p, H/s…) |

**Hole basis preferred**: `H` + the shaft's fundamental deviation (`H7/k6`). Shaft basis is used for cold-drawn round steel/standard parts (`K7/h6`).

## 2. Common Fits (Quick Reference)

| Fit | Code | Typical use |
| --- | --- | --- |
| Clearance (easy to assemble) | `H7/g6`, `H8/f7` | sliding / needs relative motion, locating pins |
| Transition (removable, accurate location) | `H7/k6`, `H7/js6` | bearing inner ring on a shaft, gear on a shaft (with a key) |
| Interference (not meant to be taken apart) | `H7/p6`, `H7/s6` | press fits, pins |
| Hole-basis locating hole | `H7` (mating with standard parts) | bolt clearance holes are normally `H12/H13` (rough assembly) |

## 3. Geometric Tolerances (GB/T 1182)

- 14 characteristic symbols (straightness, flatness, roundness, cylindricity, parallelism, perpendicularity, angularity, position, coaxiality, symmetry,
  circular run-out, total run-out, profile of a line, profile of a surface) + modifier symbols (⌀ tolerance zone, Ⓕ maximum material, Ⓜ reciprocal…).
- Annotation: a frame (symbol | tolerance value | datum) + a leader line; datums use datum symbols (A/B/C).
- Practical points: position tolerance usually carries ⌀ (circular tolerance zone); coaxiality can be replaced by "circular run-out" (easier to measure); pick a functional face as the datum.

## 4. Surface Roughness (GB/T 131)

| Ra (µm) | Typical process |
| --- | --- |
| 12.5 / 6.3 | rough turning, drilling |
| 3.2 | fine turning, milling |
| 1.6 | fine grinding, broaching |
| 0.8 / 0.4 | fine grinding, lapping, polishing |
| below 0.2 | mirror finish (lapping/superfinishing) |

- Unspecified roughness is marked with "其余" (all others) in the upper-right of the drawing or in the technical requirements (e.g. "其余 6.3" = all other surfaces 6.3).
- Mating faces, sealing faces and sliding faces must be specified individually (and matched to the fit accuracy: an IT7 face ≈ Ra1.6, IT6 ≈ 0.8).

## 5. How OCSM Does It

- In the annotation settings window choose **TOLERANCE**: code mode (`⌀30H7`) or limit-deviation mode (`⌀30 +0.021/0`);
  the plugin computes the deviations from the ISO 286 tables and draws them with an **anonymous block + stacked MTEXT**.
- **GD&T** / **DATUM** are in the same window (feature-control frame / datum symbol, two GB drawing conventions).
- Roughness: `CC` (`OCSMRGH`) interactively pick the insertion point → fill in the form and value in the window.

## To Be Added

- [ ] IT grade value tables for common size steps (or query the plugin API directly: `GET /api/tolerance?…`)
- [ ] Recommended tolerance values for geometric tolerances (by machining method)
- [ ] Selection list for general tolerances (GB/T 1804 m/f/c/v)
