# 23 Knowledge: Materials and Heat Treatment (Skeleton)

> English translation of `handbook/23-知识-材料与热处理.md` (the Chinese original is the source of truth).

> Choosing materials and writing them into the title block and the technical requirements. This page grows "as we hit things".

## 1. Common Materials Quick Reference

| Material | Designation examples | Strength / characteristics | Typical use |
| --- | --- | --- | --- |
| Low-carbon steel | Q235, 20 | cheap, easy to weld, good ductility | welded frames, brackets, backing plates |
| Medium-carbon steel | 45 | good overall mechanical properties after quenching and tempering | shafts, gears, fasteners |
| Alloy structural steel | 40Cr, 20CrMnTi, 42CrMo | good hardenability, high strength | gears, shafts, high-strength parts |
| Stainless steel | 304, 316, 2Cr13, 3Cr13 | corrosion / heat / wear resistant | food and chemical equipment, visible parts |
| Castings | HT200 (grey cast iron), QT450 (ductile iron), ZL104 (aluminium) | casts complex shapes, damps vibration | gearbox bodies, housings, pulleys |
| Aluminium | 6061-T6, 7075 | light, easy to machine (7075 is hard to weld) | lightweight structures, aerospace parts |
| Copper alloys | H62 (brass), QSn6.5-0.1 (tin bronze) | wear resistant, conductive | bearings, worm wheels, conductive parts |
| Engineering plastics | POM, PA66, PTFE, PEEK | self-lubricating, corrosion resistant, light | gears, bushings, seals |

## 2. Heat Treatment Quick Reference

| Process | Effect | Typical resulting hardness |
| --- | --- | --- |
| Annealing | relieves stress, softens, makes machining easier | ≤ 200 HBW |
| Normalizing | refines the grain, homogenizes the structure | 150–250 HBW |
| Quenching and tempering (quench + high-temperature temper) | overall mechanical properties (first choice for shafts) | 220–320 HBW |
| Quenching + low-temperature tempering | high hardness, wear resistant | 45–60 HRC |
| Carburizing and quenching | hard surface, tough core (gears) | surface 58–62 HRC |
| Nitriding | extremely high surface hardness, little distortion | surface ~1000 HV |
| Stress relief (ageing) | stabilizes dimensions (precision parts) | — |

## 3. Surface Treatment Quick Reference

| Process | Effect | Notes |
| --- | --- | --- |
| Blackening (oxidation) | rust protection, appearance | standard for steel parts, thin film |
| Zinc plating (blue-white / yellow) | rust protection | most common for standard parts |
| Chrome plating (hard / decorative) | wear resistance / appearance | dimensions change; state "plated dimensions" on the drawing |
| Anodizing | corrosion protection / appearance for aluminium parts | can be dyed |
| Sand blasting / powder coating / painting | appearance, corrosion protection | write the RAL colour code into the technical requirements |

## 4. Habits to Write Into the Drawing

- Put the designation in the "material" cell of the title block (`45`, `6061-T6`, `HT200`), and put heat treatment / surface treatment into the **technical requirements**:

  ```
  技术要求
  1. 材料 45；调质 220–250 HBW。
  2. 未注圆角 R2，未注倒角 C1。
  3. 未注尺寸公差按 GB/T 1804-m。
  4. 未注表面粗糙度 Ra 6.3。
  5. 表面处理：发黑（GB/T 15519）。
  ```

  (Technical requirements: 1. material 45; quenched and tempered 220–250 HBW. 2. unspecified fillets R2, unspecified chamfers C1.
  3. unspecified dimensional tolerances per GB/T 1804-m. 4. unspecified surface roughness Ra 6.3. 5. surface treatment: blackening (GB/T 15519).)
- The "default values" for unspecified tolerances/roughness must be declared on the drawing; do not let the machinist guess.

## To Be Added

- [ ] Material selection decisions for common parts (shaft / gear / housing / bracket)
- [ ] Material–surface treatment–roughness matching table
- [ ] Materials and weld preparations for weldments (GB/T 985 series)
