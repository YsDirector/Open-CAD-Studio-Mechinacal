# 24 Bilingual Copy (zh/en) and Adding New Entries

> English translation of `handbook/24-双语文案与新增词条.md` (the Chinese original is the source of truth).

> Settled by the user 2026-09-27: **give OCSM a full English translation so it supports both Chinese and English**.
> This file is the developer-side note (how to add entries, how to switch language, how to test); the full string inventory and batch plan are in
> `~/桌面/OCSM/review/i18n_盘点.md` (stage 1 inventory, 2026-09-27).

## 1. Three Fixed Ground Rules (set by the user)

1. **Card-face text must be translated too** (not just the UI). Implementation route: **there is only one card definition, and card-face labels render per language**
   — **forbidden** to copy a second English card per card.
2. **Standard symbols are not translated**: `D_ei` / `Az` / `Wn` / `Kn` / `d_B` / `m` / `z` / `αD` / `MM` / `IT` … stay as-is.
3. **Language mechanism**: follow the host language + manual override; every user-facing Chinese string must go into the **single catalog**,
   never remain a scattered hard-coded string (except **protocol tokens** such as command keywords / lookup keys / layer names, see §6).

## 2. Code Locations and the Lookup API

* Single catalog: `crates/ocs_ocsm/src/i18n.rs` (`CATALOG`, table-driven, **one line per key: `zh` / `en`**).
* Lookup:
  * `i18n::t("cmd.tf.usage")` — looks up in the current language, returns `String` (a `_lang` variant can be added if needed);
  * `i18n::t_fmt("cmd.tf.err.unknown_param", &[("arg", tok), ("usage", &usage)])`
    — `{name}` placeholder interpolation; **the placeholder sets of the two languages must match** (checked by `catalog_problems()`);
  * `i18n::t_lang(i18n::Lang::En, key)` / `t_fmt_lang(...)` — explicit language (for tests/GUI pre-rendering).
* Language:
  * `i18n::lang()` / `i18n::set_lang(i18n::Lang::En)` / `i18n::set_lang_auto()`;
  * `i18n::env_lang_source()` returns the environment variable currently in effect and its value (for troubleshooting).

### Language Sources (highest priority first)

| Priority | Source | Notes |
| --- | --- | --- |
| 1 | `i18n::set_lang()` | in-program override: GUI language switch `/api/i18n?lang=`, command switch, unit tests |
| 2 | environment variable `OCSMLANG` | manual override, e.g. `OCSMLANG=en` (the most common for deployment/debugging) |
| 3 | environment variable `OCSM_LANG` | compatibility alias |
| 4 | `LC_ALL` → `LC_MESSAGES` → `LANG` | proxy for the host/system locale (values accept `zh*` / `en*`; case-insensitive, `.UTF-8` suffix insensitive) |
| 5 | default `zh` | conservative default: all existing output is Chinese, and the default does not change current behaviour |

> The host API (`ocs_plugin_api::host::HostApi`) currently has **no** language/locale field (every method has been checked),
> so stage 1 falls back to environment variables; `ocs_plugin_api/**` was **not changed**. When the host exposes a language setting,
> only `resolve_env_lang()` needs to change; all other call sites stay put.

### Conservative Defaults for Missing Keys (never return a silent empty string)

* key present in the catalog but the target-language string is empty → **fall back to `zh`**, and record it in `i18n::missing_keys()`;
* key not in the catalog at all → return **the key itself** (a visible placeholder, spotted at a glance), and record it in `missing_keys()`;
* no panics at runtime; misses are caught by `catalog_problems()` (static) and `missing_keys()` (runtime).

## 2b. How GUI Page Copy Gets into the Catalog (stage 2 ① batch)

The pages (`*_gui.html`) and the shared JS (`ocsm_gui_common.js`) are **templates** compiled into the `.so` with `include_str!`:
the templates contain **no Chinese fallback**, only placeholders; the plugin's HTTP server renders them in the current language
**on every request** before sending.

| Placeholder | Used in | Escaping | Form |
| --- | --- | --- | --- |
| `{{i18n:gui.xxx}}` | HTML text nodes / attributes | HTML escaping | `<h1>{{i18n:gui.parts.h1}}</h1>` |
| `{{i18njs:gui.xxx}}` | JS string literals inside `<script>` | JS string escaping | `el("s").textContent = "{{i18njs:gui.parts.export}}";` |

* **Interpolation**: write `{name}` in the catalog, and use `.replace("{name}", v)` in the page JS, e.g.
  `"{{i18njs:gui.parts.err.not_number}}".replace("{label}", it.label)`;
  `catalog_problems()` checks that the placeholder sets of the two languages match.
* Do not write extra Chinese constants in HTML/JS ("the catalog is the single source"); command keywords/layer names/standard symbols are still untranslated (see §6).
* The actual entry points (`guide_server.rs`): `render_i18n_text[_lang]()` (placeholder substitution) +
  `render_page_i18n()` (injects the language control into the page); `page_response()` routes every page through it;
  the shared JS route `/ocsm_gui_common.js` goes through the same substitution.
* **Language-switch entry point**: the server injects a floating control `#ocsmlang` before `</body>` (its labels also come from the catalog,
  `data-lang` marks the current language); clicking the other language → `GET /api/i18n?lang=en|zh` → page reload.
  Switching is **process-wide** (`set_lang`): switch it in one drawing and every OCSM window/command line follows;
  the host environment variables (`OCSMLANG`/locale) remain the initial default.
* `GET /api/i18n[?lang=…]` returns `{ok, lang, languages, source, catalog}` (the whole table for the current language, for integration/debugging).
* Checklist for wiring a new page: ① add entries in the GUI section of `i18n.rs` and pass `catalog_problems()`;
  ② replace HTML/JS with `{{i18n*:…}}` placeholders; ③ if JS needs a value, use `.replace("{…}", v)`;
  ④ just run the seven smokes (`tests/i18n_zh_fixture.mjs` renders placeholders as zh; test-only, not a product fallback).

## 3. Adding One Entry (checklist)

1. In the **matching section** of `CATALOG` in `i18n.rs`, add one line: `Msg::new("域.对象.用途", "中文", "English")`.
   Sections: command output / errors / GUI / card-face labels / reports / handbook (key prefixes `cmd.` / `gui.` / `card.` / `report.` / `doc.`).
2. Replace the hard-coded string on the product path with `i18n::t(...)` / `i18n::t_fmt(...)`
   — **only for strings "shown to the user / written into the drawing / put into the GUI"**; command keywords, lookup keys and layer names do not go into the catalog (§6).
3. If interpolation is needed, write `{name}` placeholders; **the zh and en placeholder sets must match name by name**.
4. Card-face/drawing labels: one card definition only, labels looked up at render time; when the same sentence appears in both the GUI and an ATTDEF,
   it **must share one key**, or the same wording will differ in the two places.
5. Tests:
   * under `cargo test` the language override is **per test thread** (`i18n::set_lang` uses the `TEST_OVERRIDE` thread-local,
     see §8); the product build is still process-wide (GUI `/api/i18n`).
   * `set_lang(Zh)` → assert Chinese; `set_lang(En)` → assert English; call `set_lang_auto()` at the end;
   * existing cases asserting Chinese copy and the "switch language" cases should still take `global_state_test_lock()`
     (serializing environment variables and other global state — an existing repo convention);
   * a vertical-slice example is `lib.rs::tests::tf_usage_and_unknown_param_switch_language_via_catalog`
     and `i18n::tests::*` (completeness / fallback / switching / interpolation / env parsing);
     batch ② examples: `hole::tests::hole_messages_switch_language_with_data_intact` (10 error kinds + key data),
     `hole::tests::hole_paths_leave_no_missing_catalog_keys` (`missing_keys()` empty),
     `centerline::tests::centerline_messages_switch_language_keeping_data`,
     `guide_server::tests::command_catalog_summaries_are_bilingual_and_render_per_language`.
6. Gates (per repo rules): `cargo test -p ocs_ocsm --lib` (10 consecutive runs), `cargo build --release -p ocs_ocsm`
   (compile only, do not deploy to the user's main plugin directory), the seven smokes (each with the correct HTML paths).

## 4. Using the Language (user side)

* By default it follows the system locale; to force English (or Chinese):

  ```sh
  OCSMLANG=en   # launch OCS/plugin with it; or put it into the startup environment
  OCSMLANG=zh   # force Chinese
  ```

* **Switching in the GUI** (available since stage 2 ① batch): every OCSM page has a floating language control in the bottom-right corner
  (`#ocsmlang`, showing "语言 / 中文 / English"); click it once → process-wide plugin switch → page reload,
  and all OCSM windows/command lines then speak the new language (**existing card faces/annotation text on old drawings are not rewritten**;
  only new tables/new windows are affected).

## 5. Testing and Debugging

* `cargo test -p ocs_ocsm --lib i18n` — catalog completeness (duplicate keys / either side empty / placeholder mismatch),
  missing-key fallback, language switching, `OCSMLANG`/locale resolution order.
* Page rendering/endpoints: `guide_server::tests::page_i18n_render_*`, `api_i18n_*`,
  `embedded_page_i18n_keys_all_exist_in_catalog` (guards against mistyped keys in page placeholders).
* The seven smokes (`tests/*.mjs`) also pass against the raw HTML: `tests/i18n_zh_fixture.mjs` reproduces
  the server-side rendering as zh (test-only, not a product fallback).
* Bilingual DOM review (review directory, real chromium rendering + real font metrics):
  `python3 ~/桌面/OCSM/review/i18n_检查/gui_dom_check_i18n.py` — run once each for zh/en,
  asserting that key copy changes with the language, that text does not leave the viewport/overlap, and that the page really turns English after clicking English;
  there is also `ocs_live_check.py` (temporary OCS + real plugin .so + sha256 triple protection).
* `i18n::missing_keys()` names keys missed at runtime during integration work; `clear_missing_keys()` clears it.
* Inventory script (read-only on the repo, re-runnable): `python3 ~/桌面/OCSM/review/i18n_scan.py`
  (`--md i18n_盘点.md` regenerates the inventory table).

## 6. Explicitly Not Translated / Not Reworked

| Object | Treatment |
| --- | --- |
| standard data such as `assets/*.csv` | **not translated** (standard values/table numbers; if `*_notes.md` wording needs Englishization, open a separate ticket) |
| command keywords (`内`/`外`/`公称`/`平`/`圆`…), lookup keys | do not enter the catalog; CLI behaviour **does not change with language** |
| CAD layer/linetype/text-style names (`1轮廓实线层`, `OCSM_GB`…) | protocol tokens; drawing interoperability comes first, not translated |
| standard symbols, units, pure numeric format templates (`{} mm`) | not translated / kept as-is |
| comments, unit-test assertions, `tests/*.mjs` smoke copy | not counted in the catalog; sync the tests when changing entries |

## 7. Stage Boundaries (delivered / not done)

* **Stage 1 (delivered)**: `src/i18n.rs` skeleton (catalog + lookup + language sources + conservative missing-key defaults + unit tests);
  one vertical slice = the `TF` usage string + the "unrecognised parameter" error (bilingual and switchable on a real product path);
  full inventory in `~/桌面/OCSM/review/i18n_盘点.md`.
* **Stage 2 ① batch (delivered, 2026-09-27)**: the `{{i18n*:}}` path + `/api/i18n` + the page language control;
  all 10 pages wired, with guide/parts/hole/spline/joint/rough/bom/gear/shaft/manual's
  page headers/window names/main buttons/key states and form labels going through the catalog (parts/manual/rough are the most complete);
  the shared JS error copy also goes through the catalog; `COMMAND_CATALOG` long summaries, deep in-page guide notes and
  deep per-card form labels are left to later batches.
* **Stage 2 ② batch (first part delivered, 2026-09-27)**: command output and errors:
  the 25 `COMMAND_CATALOG` long summaries (third field = a `cmd.catalog.*` key, `/api/manual` renders per language) +
  the full `OCSMHOLE`/`DK` errors/hints/usage/receipts (`hole.rs` + `thread.rs` + `lib.rs::cmd_hole` + placement prompt) +
  centerlines `OCSM CENTERLINE`/`ZX` + the 5 item-balloon errors in `balloon.rs` + TF/frame family (`parse_frame_args`,
  `frame_files`/`resolve_frame`, insert receipts/hints, `FramePlace` prompt).
* **Stage 2 ③ batch (part delivered, 2026-09-27)**: command output and errors of the remaining command families:
  `OCSMJOINT` (joint.rs), `OCSMDIM2GB`/`D2G` (dim2gb.rs + `lib.rs::cmd_dim2gb`),
  the BOM family (bom.rs / bom_xlsx.rs; column headers/in-table tags belong to the ③ card-face batch),
  `OCSMPART`/`XL` (lib.rs + all `partgen*.rs` generation errors + `guide_server::apply_part_pick`),
  `OCSMRGH`/`CC` (`RoughnessPlace` + `apply_roughness`), plus the 8 endpoint `请求 JSON 无效` strings unified.
  **Not done (later small batches)**: OCSMCARD per-card validation/wording (card*.rs + spline_tol/gear_table/
  ansi_table/nf*/din*/spline_table/spline_lite/card_lite/card_expr), shaft `shaft.rs`,
  gears/involute splines `gear.rs`/`invol_spline.rs`, `guide_url.rs` (GUI label and protocol aliases),
  `detail.rs` (structural-detail previews/errors), `OCSMPART` GUI catalog metadata (`name`/`base_hint`/
  `len_label`/view names/tree paths), `BOM`/`parts` GUI deep labels.

* **Stage 2 ④ batch (part delivered, 2026-09-27)**:
  * ① OCSMCARD in-card validation/wording (**done**): `guide_server` card-type dispatch (9 strings) + the parse/validation/lookup errors of `card_expr`/`spline_tol`/`spline_table`/
    `gear_table`/`ansi_table`/`nf_table`/`nf_ext_table`/`din_table`/`spline_lite`/`card_lite`,
    `lib.rs` card-command receipts and `card::usage_line()`, GUI card export receipts; the card-name error prefix goes through `i18n::card_display()`.
  * ② shaft `shaft.rs` (**line-DSL part done**): all `parse_program` `{label}：…` errors, GB/T 1095/1096/1097 lookups,
    `ocsm_ready` prechecks, `USAGE` → `pub fn usage()` (full catalog text), `lib.rs::cmd_shaft` receipt.
    **Not done**: `parse_json`'s JSON input errors, `build`/geometry-validation `第 N 段：…` (~85 strings) and the report body.
  * **Not started**: ③ `gear.rs`/`invol_spline.rs`, ④ `detail.rs`, ⑤ `guide_url.rs`, ⑥ GUI deep metadata (OCSMPART catalog etc.).

* **Stage 2 ⑤ batch (2026-09-27, part committed + finished in this session)**:
  * shaft `shaft.rs`'s `parse_json` JSON input errors + `build`/geometry validation (`第 N 段：…`) + `build_report` report:
    done (commit `ba41b888`); this session added a live recheck with the real plugin (en 69 OK / zh 0 errors, triple protection + both directions).
* **Stage 2 ⑥ batch (this session, uncommitted) — the "gear/details/leader" three families + the involute entry layer**:
  * **③a `gear.rs` core (done)**: all `cmd.gear.*` 153 strings: `validate`/`notes`/`parse_request`/`params_from_query`/
    `ocsm_ready`/`spec`/`gear_report` report; `kind_label()`/`view_label()`; `M_DP_CONFLICT_MSG` →
    `m_dp_conflict_msg()` (guide_server synced).
  * **③b `invol_spline.rs` entry layer (part)**: `cmd.invol.*` 36 strings: the four unified error messages turned into functions,
    `parse_ansi_pitch_syntax`/`ansi_pitch_series_check`/`ansi_prefixed`/`ansi_column_pitch_check`,
    `resolve_spline` entry validation. **Not done**: per-system table validation/lookup errors, NF A derivation, long wording and `build_report`.
  * **④ `detail.rs` (done)**: `cmd.detail.*` 62 strings: family/view/grinding-wheel relief groove/undercut/hub keyway/rectangular spline/GB/T 3 Table 1;
    new `display_name()` (family names follow the language); `join("、")` changed to a language-dependent separator.
  * **⑤ `guide_url.rs` (done)**: `cmd.guide.*` 13 GUI display names (`label()` → `String`); protocol aliases (H/V/A,
    LEFT/RIGHT, M/R, LIN/CAV/CVX/DBL/ZIG and the Chinese aliases) are never translated.
  * **⑥ GUI deep metadata (not done)**: `partgen*`/`detail`'s `name`/`base_hint`/`len_label`/`tree_dir`/
    `view_labels`/`source`, BOM column headers/tags, deep notes of the remaining pages (the trait returns `&'static str` and needs a uniform rework).
  * Gates (this session): `cargo test -p ocs_ocsm --lib` **10 consecutive runs, all 834 passed / 0 failed / 25 ignored**;
    all seven smokes pass; `cargo build --release -p ocs_ocsm` passes (compile only, no deployment);
    live recheck with the real plugin: en **0 errors** / zh literals **0 errors** (temporary XDG, deployed replacement plugin.toml,
    sha256 = `target/release 0a097d79…`, runner pointed at the temporary .so); the user's main plugin untouched (`4b7850db…`).
* **Stage 3 ③ family (2026-09-27, uncommitted)**: bilingual card faces for the DIN internal/external + lite cards; ★ **the NF/DIN card faces are MTEXT, and acadrust `MText` has no entity width factor ⇒ English compression uses the inline `\W<f>;` (effective width factor = `OCSM_GB` 0.7 × f)**, while TEXT/ATTDEF still use the entity width factor; GUI deep metadata (`AnsiLang::label`/`AnsiColumnSpec`/`CardTypeSpec.label+group`/`partgen*` tree and view names/the rest of `detail.rs`) and `bom::CELL_TAGS` compatibility (a new English tag set; the read side accepts both zh/en, and the exported header follows the language) have entered the catalog.

* **Stage 3 ⑤ batch (2026-09-27, GUI metadata wrap-up + `handbook/en/` wired into the manual window)**:
  * **Card-type summaries**: `card.rs::CardTypeSpec.summary` → `summary_key` + `summary()` (22 `gui.card.*.summary` strings);
    `card_types_json` and the report's "card wording" line share one lookup source (`card_report.rs` goes through `card.summary()`).
  * **Three column-wording tables**: `gear_table::GearColumnSpec` / `nf_table::NfColumnSpec` / `nf_ext_table::NfExtColumnSpec`
    changed from `label/formula/source` → **`label_key`/`formula_key`/`source_key` + accessors**
    (`gui.gear|nf|nf_ext.col|formula|source.*`; repeated sources share one key).
  * **DIN**: `din_table::RowSpec.formula/source` → keys + accessors (`gui.din.formula|source.*`);
    preview/option `missing_note` → `gui.din.missing_note`; the form's missing-field note → `gui.form.din.missing_note`.
  * **GB lite card field notes** (`spline_lite::LiteFieldSpec`) and the **gear lite three circles** (`card_lite::LiteFieldSpec`)
    `formula/source` → keys (`gui.gb_lite.*` / `gui.gear_lite.*`).
  * **Form hints**: `CardFormSpec.note`/`missing_note` → `note_key`/`missing_note_key` + accessors
    (8 forms: gear/ansi/nf/nf_ext/din×2/gb_lite internal/external; `gui.form.*`).
  * ★ **`handbook/en/` wired into the manual window**: `guide_server::manual_topics_in()` now **picks topics per language**
    (zh lists only the Chinese top-level pages; en lists `en/` pages where a translation exists, **falls back to the Chinese original for missing translations and marks `fallback=true`**,
    showing "中文原文（该篇尚无英文版）" (Chinese original — no English version of this page yet) in the window); `find_manual` looks for `<dir>/en/<slug>.md` first in an en environment;
    `/api/manual` gains `lang`/`fallback` per entry; zh and en pages are paired by filename prefix (`00-…`), and `en/README.md` does not count as a page.
  * The Chinese side is **unchanged character by character** (231 zh strings automatically extracted by script from `git HEAD`, not hand-copied); the regression net =
    `card::tests::gui_metadata_batch_switches_language_without_cjk_or_missing_keys` (260+ strings collected for zh and en,
    zero Chinese characters in English + every entry containing Chinese follows the language + `missing_keys()` empty).

## 8. Known Pitfalls (already stepped on)

* **`spline_gui.html` / `ocsm_gui_common.js` are `include_str!`-ed into the `.so`**:
  after editing a page you must `cargo build --release` and deploy only into a temporary plugin directory (do not touch `~/.config/OpenCADStudio/plugins/`).
* **Old drawings are not translated**: once a card-face label is in an ATTDEF it is drawing content; a language switch only affects **new tables**.
* **Interpolation in page JS** must not concatenate Chinese literals directly: write `"{{i18njs:key}}".replace("{name}", v)`;
  for HTML contexts use `{{i18n:key}}` (the two escape modes differ; do not mix them). (stage 2 ① batch)
* **The language is a process-wide global**: in the product build `set_lang` writes the global (GUI `/api/i18n` takes effect for the whole plugin once chosen);
  **but under `cargo test` it is per test thread** (`#[cfg(test)] TEST_OVERRIDE` in `i18n.rs`).
  This is a pitfall from batch ②: if `set_lang` in a test also wrote the global, a case switching to En would, when run in parallel,
  crash the cases asserting Chinese copy (measured once: 6 false reds out of 20 consecutive runs); the thread-local override made it disappear.
  Cases asserting Chinese copy should still hold `global_state_test_lock()` (serializing environment variables and other global state).
* **Card faces/annotation text already on old drawings are not rewritten**; card-face labels/field-table labels still belong to later batches.
* `catalog_problems()` only checks static problems; "is this key actually used by anyone" depends on the runtime `missing_keys()` + a human look.
