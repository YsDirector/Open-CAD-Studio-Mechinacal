# 11 Automation Interfaces (AI / Script-Driven OCS)

> English translation of `handbook/11-自动化接口.md` (the Chinese original is the source of truth).

> **Required reading for the AI side.** Three entry points: the host's native MCP, the plugin MCP, and the plugin's local HTTP.

## 1. The Three Entry Points

| Entry point | Start it | What you use |
| --- | --- | --- |
| **Host-native MCP** | launch OCS with `--mcp` (an MCP badge appears beside the command line) | tools: `ocs_sessions` / `ocs_read` / `ocs_execute` / `ocs_capture` |
| **Local browser control bridge** | while the host is running its automation service | descriptor `~/.config/OpenCADStudio/automation/<session_id>.json` (0600, contains port/token) → call it over raw TCP/HTTP |
| **Plugin MCP / HTTP** | run `OCSMMCP` in OCS (it prints the connection info) | plugin MCP: plugin capabilities such as standard parts; plugin HTTP: `http://127.0.0.1:23751` (tries ports from 23751 upward, at most +16) |

> The three can be mixed: **the host interface manages the "drawing"** (read state / run commands / take screenshots), while **the plugin interface manages "plugin-specific capabilities"** (parts library, bolt joints, the annotation windows' HTTP API).

## 2. The Four Host MCP Tools

### `ocs_sessions`
Lists/opens sessions. Returns `session_id` (all later calls need it).
`{"launch_if_none": false}` means: when there is no session, do not launch a new one.

### `ocs_read` (read-only)
`op` values (the common ones):

| op | Purpose |
| --- | --- |
| `state` | **must be called before every round of operations**: `document_id`, `revision`, current layer, selection-set size… |
| `query` | query entities/records: `type`, `layer`, `bounds`, `near`, `contains_point`, `handle(s)`, `detail` (summary/geometry/full). Note: **it also lists the members inside block definitions** (e.g. after inserting a drawing frame you will see a pile of `1轮廓实线层`/`2细线层` lines at 1:1 coordinates — that is block-definition content, not on-drawing copies; the inserted body itself is that `Insert`) |
| `history` | **command-line log** (including the plugin's `push_output` report lines and "撤销：xxx" (undo: xxx)) |
| `events` | event stream (including `kind:"selection"` **highlight changes** and `selection_revision`) |
| `commands` | command list (including plugin commands; filterable by `search`) |
| `records` | records/extended data (including `OCSM_PART`, `OCSM_EDIT`, hyperlinks…) |

### `ocs_execute` (modifies the drawing)
The `op` of `request`:

| op | Description |
| --- | --- |
| `run` | run a command (**plugin commands go through here too**): `{"op":"run","request_id":"…","document_id":N,"revision":R,"cmd":"OCSMJOINT at …"}` |
| ↑ direct frame insertion | `cmd:"TF a3_landscape 1:2 at 0,0"` — **no need to click a selection window** (`TF` with parameters = direct insert, see `02-sheet-and-frame.md`); only without parameters does it pop up a window |
| ↑ entity as the answer | `cmd:"ZX 62"` (hexadecimal handle) — the plugin must hand non-keyword tokens back to the host (`on_text_input → Ignored`), and only then does the host feed the handle into the pick step |
| `cancel` | cancel the current interactive command (= Esc); **it is recommended to cancel before sending the next command** |
| `undo` | undo one step (equivalent to Ctrl+Z) |
| `new` / `open` / `save` | create/open/save a document |

Key points:
- **`request_id` must be unique**; reusing one reports `Request payload changed` (see `12-troubleshooting.md`).
- A stale `revision` raises an error → use the latest value from `state`.
- A command's return value may be just "submission accepted" — **the real result is in the `history` log** (all plugin reports are written to that log).

### `ocs_capture`
`scope: viewport|window`, `max_dimension`. **Take a screenshot after every step and verify against it**; do not just look at the returned ok.

### Interactive commands (`start` + `input`) — must use the **same MCP session**

The host gates ownership of `input`: while a command is active it only lets it through when `control.owner == Some((tab, client_id))`,
and the MCP server gives **every client instance** a random `client_id` (`src/mcp.rs:326`).
So the "one new connection per call" approach (e.g. `mcporter call ...`) always returns
`command_busy` for `input` — you must use **one long-lived connection**: `tools/ocs_session.py` in the repo is exactly that (verified working):

```bash
python3 tools/ocs_session.py <<'EOF'
run CENTERMARK
input entity DD 315,350,0      # entity pick: handle + pick point
cancel                         # end that interactive command
run D2G
query line 3中心线层
EOF
```

* Plugin commands have another shortcut: as long as the plugin returns `CommandStep::Ignored` from `on_text_input`,
  you can send the **hexadecimal handle as the command answer** in one shot: `{"op":"run","cmd":"ZX 62"}`.
* Host-native commands (`CENTERMARK`/`DIMCENTER`/`DIMLINEAR`…) do not accept this kind of answer; they must go through `start`+`input`.

## 3. Plugin HTTP (shared by scripts / windows)

| Endpoint | Purpose |
| --- | --- |
| `GET /api/parts` | parts catalog (family/kind/size/length series/view) |
| `GET /api/part_svg?family&d&l&view` | single-part preview SVG (structural details have no `l`: grinding-wheel relief grooves pass `&b1=`, undercuts pass `&P=1.5&g1=…`) |
| `POST /api/part_pick` | parametrically insert a single part (JSON: family/d/l/view/x/y/rotation; structural details may also carry `"params":{"P":1.5}`) |
| `POST /api/part_export` | "出库" (export out of the library): create a block + register a pending part (used by the human-side window button) |
| `POST /api/joint` | **assemble a bolt joint** (part-chain JSON, see `04-bolt-joint.md`) |
| `POST /api/joint_plan` | compute only, write nothing: returns an SVG preview + report + numbers |
| `POST /api/joint_place` | create a preview block + register a pending part chain (used by the human-side window button) |
| `GET /gear` | **gear window** (parameter table + view buttons + live preview; switch between external/internal gear; opens when `OCSMGEAR` is run without parameters) |
| `GET /api/gear_svg?m&z&h&ha&c&beta&x&view&n` | SVG preview of the gear's current view (`view`=`section`/`side`/`simplified`/`front`; `beta` in degrees, right-hand helix positive; `n` = drawing-frame scale) |
| `GET /api/gear_info?…` (same parameters) | gear derived dimensions + hints (d/da/df/db/ρ/chamfer C/pitch angle/tip half-tooth angle/notes) |
| `POST /api/gear_export` | gear "生成到图纸" (generate to drawing): create a block + register a pending part (used by the human-side window button; it likewise drives `OCSMGEAR`). The body is JSON and may carry `"kind":"internal"` to output an internal gear. **If `OCSM` has never been run it returns 400 + the block reason** |
| `POST /api/gear_insert` (command line) | AI one-line direct insert: `run "OCSMGEAR 2 40 20 view front at 300,200 rot 0"` |
| `GET /shaft` | **shaft generator window** (segment table ↔ line text two-way sync + live preview; opens when `OCSMSHAFT` is run without parameters) |
| `GET/POST /api/shaft_parse` | DSL/JSON → normalized segment model (used to fill the segment table back from the GUI line text; errors carry "第 N 行/段 + 原因" (line/segment N + reason)). The POST body is `{"dsl":"…"}` or a JSON model / line DSL; GET uses `?dsl=` |
| `GET/POST /api/shaft_preview` | shaft side-view preview SVG (parameters as above; does not touch the drawing) |
| `POST /api/shaft_export` | shaft "生成到图纸": body = JSON model (+ optional `at`/`rot` + `tab`). Without `at` = create a block + register a pending part (used by the human-side window button); with `at` = insert directly. **If `OCSM` has never been run it returns 400 + the block reason** |
| `GET /hole` | **hole generator window** (simple/threaded/counterbore/countersink + blind/through + automatic switch + live section preview; opens when `OCSMHOLE`/`DK` are run without parameters) |
| `GET /api/hole_sizes` | the hole generator's four data tables (ISO 724 coarse/fine thread, drill pilot-hole diameter, GB/T 152.3 counterbore, GB/T 152.2 countersink, GB/T 5277 clearance), each with `source`/`note` |
| `POST /api/hole_preview` | hole model JSON → `{ok, svg, values}` (preview + derived values; does not touch the drawing) |
| `POST /api/hole_export` | hole "确定" (OK): body = JSON model (+ optional `at`/`rot` + `tab`). Without `at` = create a block + register a pending part; with `at` = insert directly. **If `OCSM` has never been run it returns 400 + the block reason** |
| `GET /api/guide?handle=` / `POST /api/apply` / `POST /api/apply_refresh` | read parameters / write the hyperlink / generate the annotation for a leader annotation |
| `POST /api/rough_apply` | generate a roughness symbol |
| `GET /guide.html?handle=` `GET /parts` `GET /joint` `GET /gear` `GET /shaft` `GET /hole` `GET /rough.html` | the individual configuration-window pages |
| `GET /api/page_ping?p=<页面>` | window heartbeat (avoids duplicate windows) |

## 4. Hard Rules (bought with lessons from the pits)

1. **Re-read `state` before touching the drawing**: `document_id`/`revision` can change every time (new document, someone else editing the drawing,
   the document lost after a host restart). **Never hard-code**.
2. **One transaction = one undo entry**: the plugin's batch drawing commands do their own `BeginUndo`/`CommitUndo`;
   when a script fires several `run`s in a row, those are multiple undo entries.
3. **Interactive commands "eat" subsequent input**: while in a placement/pick state, the next `run` you send is treated as a point/parameter.
   Send `op:"cancel"` before a new command.
4. **Highlights (selections) change**: use `events`' `selection_revision` to decide whether the selection must be re-read; do not cache handle sets for too long.
5. **Command names are case-insensitive and parameters are forwarded verbatim**; the plugin's reports (`OCSMJOINT：…`) live in `history`, not in the return value.
6. **Headless mode (`--serve`)**: `open` / `run <command + inline coordinates>` / `save` works reliably as strict request/response;
   `--export` only supports `.dwg`/`.dxf` (no png/pdf) → for image output use another rendering path (see `12-troubleshooting.md`).
7. **After a plugin hot-reload/restart**: documents are lost, `document_id` changes, but GUI-window heartbeats still respond (the old pages are still alive) →
   close the old windows before opening new ones.

## 5. Minimal Runnable Round (copy-paste ready)

```bash
MC=/home/ysdirector/.local/bin/mcporter
S=$($MC call ocs.ocs_sessions --args '{"launch_if_none":false}' --output json | python3 -c 'import json,sys;print(json.load(sys.stdin)["result"][0]["session_id"])')
$MC call ocs.ocs_read  --args "{\"ocs_session_id\":\"$S\",\"op\":\"state\"}" --output json     # get document_id/revision
$MC call ocs.ocs_execute --args "{\"ocs_session_id\":\"$S\",\"request\":{\"op\":\"run\",\"request_id\":\"r1\",\"document_id\":1,\"revision\":9,\"cmd\":\"OCSM\"}}" --output json
$MC call ocs.ocs_read  --args "{\"ocs_session_id\":\"$S\",\"op\":\"history\"}" --output json    # read the plugin report
$MC call ocs.ocs_capture --args "{\"ocs_session_id\":\"$S\",\"scope\":\"viewport\"}" --save-images /tmp/ocs --output json
```

## 6. Verification Checklist

- [ ] `ocs_sessions` returns a `session_id`; `ocs_read op:"commands"` lists the plugin commands (meaning the plugin is loaded).
- [ ] `state` was fetched before every write operation (no revision errors appear in the log).
- [ ] The final screenshot matches the `history` report.
