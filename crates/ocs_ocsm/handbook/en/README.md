# OCSM Handbook — English Edition

> English mirror of the Chinese handbook in `handbook/`. **The Chinese original is the source of truth**;
> when the two disagree, fix the translation, not the original.
> Started 2026-09-27 (stage 4 of the OCSM bilingual i18n roadmap; see `~/桌面/OCSM/review/handbook_英译_进度.md`).
> **Stage 4 is complete: all 23 pages are translated** (2026-09-27); this table is the zh→en file mapping.

## Files

| English | Chinese original | Status |
| --- | --- | --- |
| `00-overview.md` | `00-总览.md` | ✅ translated |
| `01-init-and-layers.md` | `01-初始化与图层.md` | ✅ translated |
| `02-sheet-and-frame.md` | `02-图幅与图框.md` | ✅ translated |
| `03-standard-parts.md` | `03-标准件库.md` | ✅ translated |
| `04-bolt-joint.md` | `04-螺栓副装配.md` | ✅ translated (batch 7) |
| `05-smart-dimension-d.md` | `05-智能标注D.md` | ✅ translated |
| `06-leader-annotation.md` | `06-引导线标注.md` | ✅ translated (batch 6) |
| `07-one-click-gb.md` | `07-一键转国标.md` | ✅ translated |
| `08-tolerance-roughness-gdt.md` | `08-公差粗糙度与GDT.md` | ✅ translated (batch 6) |
| `09-edit-annotations-text.md` | `09-改标注与文字.md` | ✅ translated |
| `10-bom.md` | `10-明细表BOM.md` | ✅ translated (big page, batch 7) |
| `11-automation-api.md` | `11-自动化接口.md` | ✅ translated (batch 7) |
| `12-troubleshooting.md` | `12-故障与坑.md` | ✅ translated |
| `13-task-kickoff-protocol.md` | `13-任务启动协议.md` | ✅ translated (batch 6) |
| `14-item-numbers.md` | `14-序号标注.md` | ✅ translated (batch 6) |
| `15-centerlines.md` | `15-中心线.md` | ✅ translated (batch 6) |
| `16-gears.md` | `16-齿轮.md` | ✅ translated (big page, batch 5) |
| `17-hole-generator.md` | `17-孔生成器.md` | ✅ translated (batch 7) |
| `20-knowledge-gb-drafting.md` | `20-知识-国标制图画法.md` | ✅ translated (batch 6) |
| `21-knowledge-fastener-selection.md` | `21-知识-紧固件选型与防松.md` | ✅ translated |
| `22-knowledge-tolerances-fits.md` | `22-知识-公差与配合.md` | ✅ translated |
| `23-knowledge-materials-heat-treatment.md` | `23-知识-材料与热处理.md` | ✅ translated |
| `24-bilingual-copy-and-entries.md` | `24-双语文案与新增词条.md` | ✅ translated (big page, batch 7) |

## Translation conventions

* **Never translated**: command names/aliases and command keywords, CLI argument tokens, layer/linetype/style names
  (`1轮廓实线层`, `OCSM_GB`…), standard symbols (`D_ei`, `Az`, `Wn`, `Kn`, `d_B`, `m`, `z`, `αD`…), standard numbers
  (GB/T …, ISO …, DIN …, ANSI …, NF …), table `name` values (`G1/8`…), catalog keys, file names, block names and ATTDEF tags,
  numbers and formulas.
* **Code blocks**: commands/output samples stay verbatim; only narrative text (including comments) is translated.
  Where a product string is still Chinese (output i18n batch not landed yet) it is quoted as emitted and glossed in
  English in the surrounding prose — e.g. `未知命令` (unknown command).
* **Terminology**: taken from the product catalog `crates/ocs_ocsm/src/i18n.rs` (read-only reference) to stay consistent
  with the bilingual GUI/command/card strings: layer, linetype, text style, dimension style, parts library, standard parts,
  structural details, bolt joint assembly, smart dimension, leader annotation, one-click GB conversion, BOM, item balloon,
  centerline, gear, internal gear (ring), shaft generator, hole generator, spline table, smart card, tolerance, roughness,
  geometric tolerance, section view, end/front view, …
* **Cross-references** point to the English file names in this table (not to the Chinese originals).
* Each file starts with a note giving its Chinese original — used to spot stale translations when the original changes.

## Maintenance

1. Edit the Chinese original first, then update the matching English file (and the table above).
2. A translation is "current" when the English file mentions the same section/file names as the original; the
   `handbook/英译` progress file tracks what has been translated.
3. Since commit `48ed9462` (stage 3⑤) the `handbook/en/` files **are** served by the manual window: `guide_server::manual_topics_in()`
   picks topics per language (zh → the Chinese top-level pages; en → the `en/` pages, falling back to the Chinese original with `fallback=true`
   when a translation is missing), and `find_manual()` looks for `<dir>/en/<slug>.md` first in an `en` environment.
   `en/README.md` itself is not listed as a topic.
