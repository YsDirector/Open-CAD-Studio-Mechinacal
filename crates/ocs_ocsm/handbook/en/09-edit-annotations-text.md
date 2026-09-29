# 09 Editing Dimensions and Text (`ME` / `OCSMEDIT`)

> English translation of `handbook/09-改标注与文字.md` (the Chinese original is the source of truth).

> The dimensions/symbols OCSM generates are **parametric products**: change the parameters → regenerate; do not drag them by hand.

## 1. `ME` (`OCSMEDIT`)

```
1) select an OCSM-generated annotation (size/weld/leader/tolerance/view/roughness…)
2) ME            (= OCSMEDIT) → opens the **same** annotation configuration window
3) the window reads that annotation's OCSM_EDIT record from /api/guide → restores all parameters (text/style/offset/type…)
4) after editing click "应用并刷新" (apply & refresh) → the plugin **deletes the old annotation and regenerates it from the original guide geometry** (replace, not stack)
```

- With no annotation selected: `OCSM: 请先选中一个 OCSM 生成的标注（引导生成的尺寸/焊接/引线/公差等），再执行 OCSMEDIT。`
  (OCSM: select an OCSM-generated annotation first — a guide-generated size/weld/leader/tolerance etc. — then run OCSMEDIT.)
- If the record has no `OCSM_EDIT` (e.g. a native dimension, or a standard part) → the command does not apply (convert it into an OCSM annotation with `D2G` first).

## 2. Text

| Requirement | How |
| --- | --- |
| technical requirements / notes | use the host's `MTEXT`/`TEXT` on `6文字层`, with style `OCSM_GB` |
| dimension text | **do not edit dimension text by hand**; change it in the configuration window (then "应用并刷新") |
| moving dimension text outwards | dragging the text extends the dimension line automatically (already implemented), or adjust the offset in the window |
| inconsistent fonts | use `OCSM_GB` throughout; mixed fonts degrade to substituted fonts (e.g. simplex for Latin) |

## 3. AI-Side Recipe

```jsonc
// 1) find the annotation to edit (handle)
{ "op": "read", "op": "query", "parameters": { "type": "Dimension", "layer": "7标注层" } }
// 2) select it and open the configuration window
{ "op": "run", "request_id": "m1", "document_id": 1, "revision": 50, "cmd": "SELECT 3F" }   // selection set
{ "op": "run", "request_id": "m2", "document_id": 1, "revision": 50, "cmd": "ME" }
// 3) read its record (parameters + guide geometry)
GET http://127.0.0.1:23751/api/guide?handle=0x3F
// 4) change parameters and regenerate (equivalent to "应用并刷新" in the window)
POST /api/apply_refresh  { "handle": "0x3F", "type": "DIAMETER", "text": "⌀30H7", … }
// 5) verify: the old annotation is gone, the new one has a different handle
{ "op": "read", "op": "query", "parameters": { "type": "Dimension" } }
```

## 4. Pitfalls

1. **"apply & refresh" = delete the old and create a new one** → the annotation handle changes; if you referenced the old handle elsewhere (scripts, hyperlinks), fetch it again.
2. **Do not edit the geometry of a generated annotation by hand** (dragging arrows, changing text): the next refresh restores it; change the parameters instead.
3. **The guide line must still exist**: `ME` regenerates from the "original guide geometry" → if the guide line was deleted you can only draw it again.
4. **The same guide line can be refreshed repeatedly** (the guide line carries a URL and is the persistent carrier of the "intent").
5. **The sender channel must not be used inside the command** (it would deadlock with the main thread) — this is a plugin implementation constraint; users can ignore it, but remember it if you are writing a plugin.

## 5. Verification Checklist

- [ ] The parameters in the window opened by `ME` equal the actual values of the annotation in the drawing (which means the record was read).
- [ ] After apply & refresh, there is exactly one such annotation in the drawing (nothing stacked).
- [ ] Text/style conform to GB (verify with a screenshot).
