//! Request/response envelopes exchanged between the host and a plugin process.
//!
//! A single bidirectional socket is used. Each side sends either a request
//! (expecting a response) or a response (to a previous request). This lets the
//! host handle plugin RPCs inline while it waits for the result of a host→plugin
//! request such as `Dispatch`, avoiding the need for two sockets or threads.
//!
//! Compatibility rule: new variants are appended at the end of every public enum
//! (`HostRequest`, `HostResponse`, `PluginRequest`, `PluginResponse`,
//! `RunnerHandshake`) so older plugins keep their bincode discriminant indices.
//! The V4 additions are a separate frame layer in [`crate::ipc::v4`] and do not
//! alter these enums.
//!
//! The pre-shared runner token is delivered through [`PLUGIN_TOKEN_ENV`]
//! (`OCS_PLUGIN_TOKEN`). The runner must present the same token immediately
//! after connecting or the host closes the connection.

use serde::{Deserialize, Serialize};

use crate::host::{CommandSource, CommandStep, HostSettingValue};
use crate::manifest::ApiVersion;
use crate::ribbon::owned::{OwnedPluginManifest, OwnedRibbonGroup};

pub use codec::xdata::{ExtendedDataRecord, XDataValue};
pub use codec::{CadDocument, EntityType, Handle};
pub use crate::host::PreviewWire;

/// Events the host forwards to an active plugin `InteractiveCommand`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractiveEvent {
    /// User clicked or specified a point coordinate.
    Point([f64; 3]),
    /// User pressed Enter or Return to complete input.
    Enter,
    /// User selected an existing entity in the drawing.
    ///
    /// fork: `snapped` distinguishes an OSNAP hit from the raw click point (the
    /// host only fills it for plugins that opt into object-pick snapping — see
    /// `InteractiveCommand::on_object_pick_snapped`). ⚠ This is the one wire
    /// shape that differs from upstream's 2-field `ObjectPick`.
    ObjectPick {
        handle: Handle,
        pt: [f64; 3],
        snapped: bool,
    },
    /// User cancelled the prompt (e.g. pressed ESC), resetting interactive collection.
    /// (upstream's index 3 — kept ahead of the fork's own events so plugins built
    /// against upstream decode it correctly)
    Cancel,
    /// fork: typed command-line text (keyword letter, value, or override).
    Text(String),
    /// fork: mouse moved to `pt`; the plugin returns a *preview entity* to render
    /// transiently (see `InteractiveCommand::on_mouse_move`). Upstream's separate
    /// *wire* preview travels as `HostRequest::CursorMove` / `HostResponse::PreviewWires`.
    MouseMove([f64; 3]),
}

/// Initial handshake sent by the plugin runner immediately after connecting.
///
/// The runner proves it was spawned by this host by presenting a pre-shared
/// token delivered through the `OCS_PLUGIN_TOKEN` environment variable. A
/// mismatch causes the host to close the connection.
///
/// `TokenV4` is appended as the last variant so the existing `Token(String)`
/// variant keeps its bincode discriminant index (0), preserving V2/V3 wire
/// compatibility.
#[derive(Debug, Serialize, Deserialize)]
pub enum RunnerHandshake {
    Token(String),
    TokenV4 { token: String, protocol_version: u32 },
}

/// Environment variable through which the host passes the pre-shared
/// authentication token to the plugin runner child process.
pub const PLUGIN_TOKEN_ENV: &str = "OCS_PLUGIN_TOKEN";

/// Requests the host sends to the plugin runner.
#[derive(Debug, Serialize, Deserialize)]
pub enum HostRequest {
    GetManifest,
    GetRibbon,
    Dispatch {
        cmd: String,
    },
    InteractiveEvent {
        command_id: u64,
        event: InteractiveEvent,
    },
    GetPrompt {
        command_id: u64,
    },
    NeedsEntityPick {
        command_id: u64,
    },
    Shutdown,
    ExecuteCode {
        command_id: u64,
        source: CommandSource,
        code: String,
        tab_index: usize,
    },
    // ── upstream v6/v7 additions first, then the fork's own additions ───────
    // (ordering keeps upstream's bincode discriminant indices intact; the fork's
    // plugin is rebuilt with this crate. See docs/fork-patches.md §A-1.)
    /// V7: release an interactive command after completion or cancellation.
    DropInteractive { command_id: u64 },
    /// V7: cursor move preview update during an interactive command.
    CursorMove {
        command_id: u64,
        pt: [f64; 3],
    },
    // ── fork: the fork's own additions follow (see docs/fork-patches.md §A-1) ─
    /// API v5: whether the interactive command wants typed text input.
    WantsTextInput {
        command_id: u64,
    },
    /// API v5: whether the interactive command wants mouse-move previews.
    WantsMouseMove {
        command_id: u64,
    },
    /// API v5: whether the interactive command wants object-snap at
    /// entity-pick clicks (POWERDIM toggles pick-point vs segment-select mode).
    EntityPickOsnap {
        command_id: u64,
    },
}

/// Responses the plugin runner sends back for `HostRequest`.
#[derive(Debug, Serialize, Deserialize)]
pub enum HostResponse {
    Bool(bool),
    CommandStep(Box<CommandStep>),
    Text(String),
    Ribbon(Vec<OwnedRibbonGroup>),
    Manifest(OwnedPluginManifest),
    Error(String),
    CodeExecutionResult(crate::host::ExecutionResult),
    // ── upstream v6/v7 additions first, then the fork's own additions ───────
    // (ordering keeps upstream's bincode discriminant indices intact; the fork's
    // plugin is rebuilt with this crate. See docs/fork-patches.md §A-1.)
    PreviewWires(Vec<PreviewWire>),
    /// API v5: the plugin's preview entity for the current cursor position.
    Preview(Option<EntityType>),
}

/// Requests the plugin runner sends to the host.
#[derive(Debug, Serialize, Deserialize)]
pub enum PluginRequest {
    PushInfo(String),
    PushOutput(String),
    PushError(String),
    AddEntity(EntityType),
    /// Replace the existing entity carrying this entity's handle in place.
    UpdateEntity(EntityType),
    /// Delete the entity with `handle`.
    RemoveEntity {
        handle: Handle,
    },
    BumpGeometry,
    ReadRecord {
        handle: Handle,
        app_name: String,
    },
    WriteRecord {
        handle: Handle,
        record: ExtendedDataRecord,
    },
    RemoveRecord {
        handle: Handle,
        app_name: String,
    },
    PushUndo {
        label: String,
    },
    SetDirty,
    StartInteractive {
        command_id: u64,
    },
    DocumentSnapshot,
    /// Ask the host to create/refresh a shared-memory document view and return
    /// the file path + current version.
    OpenDocumentView,
    /// Add multiple entities in a single request.
    AddEntities(Vec<EntityType>),
    /// V4: ask the host to create/refresh a tab-keyed shared-memory document
    /// view and return the file path + current version.
    OpenDocumentViewV4 { tab_id: u64 },
    /// V4: ask the host to close the tab-keyed shared-memory document view.
    CloseDocumentViewV4 { tab_id: u64 },
    /// V4: ask the host for the stable tab identifier of the active tab.
    GetTabId,
    /// V5: ask the host for the filesystem path of the document in `tab_id`.
    DocumentPath { tab_id: u64 },
    // ── upstream v6/v7 additions first, then the fork's own additions ───────
    // (ordering keeps upstream's bincode discriminant indices intact; the fork's
    // plugin is rebuilt with this crate. See docs/fork-patches.md §A-1.)
    /// Add a layer to the active document with full initial properties.
    AddLayer(crate::host::LayerConfig),
    /// Modify specified properties of an existing layer in the active document.
    ModifyLayer(crate::host::LayerConfig),
    /// Run a command on the active document tab's command line (AutoLISP style).
    ExecuteCommand(String),
    /// Read a host-managed setting without nested command dispatch.
    GetSystemVariable { name: String },
    /// Change a host-managed setting without nested command dispatch.
    SetSystemVariable { name: String, value: HostSettingValue },
    /// V7: validate and replace existing entities in a single undo step.
    UpdateEntitiesTransaction { label: String, entities: Vec<EntityType> },
    /// V7: synchronous selection read/write for the dispatch tab.
    GetSelection,
    SetSelection { handles: Vec<Handle> },
    /// V7 (additive): kernel-backed solid create or transform.
    SolidOperation { operation: crate::host::SolidOperation },
    /// V7 (additive): drawing table record create/modify/rename/delete.
    TableOperation { operation: crate::host::TableOperation },
    /// V7 (additive): drive an OCS command.
    RunCommand { request: crate::host::CommandRequest },
    /// API v5: handles of the currently selected entities.
    SelectedHandles,
    /// API v5: set the current layer by name.
    SetCurrentLayer(String),
    /// API v5: create missing layers.
    EnsureLayers(Vec<crate::host::LayerDef>),
    /// API v5: create missing linetypes.
    EnsureLinetypes(Vec<crate::host::LinetypeDef>),
    /// API v5: create missing text styles.
    EnsureTextStyles(Vec<crate::host::TextStyleDef>),
    /// API v5: open the host frame picker modal with the given frames.
    ShowFramePicker(Vec<crate::host::FrameItem>),
    /// API v5: take the pending frame selection, if any.
    TakePendingFrameSelection,
    /// API v5: load a frame DWG and define it as a block.
    ImportFrameBlock(crate::host::ImportFrameBlockRequest),
    /// API v5: create missing dimension styles.
    EnsureDimStyles(Vec<crate::host::DimStyleDef>),
    /// API v5: create a block definition whose members are `entities` (world
    /// coordinates, insertion at origin). Used for baked dimension graphics
    /// (anonymous `*D` blocks). Fails when the block already exists.
    AddBlockRecord {
        name: String,
        entities: Vec<EntityType>,
    },
    /// API v6: open an undo transaction. Unlike `PushUndo`, the snapshot stays
    /// pending across host message boundaries so a plugin flow that issues
    /// several requests for one user action (the local HTTP surface) still
    /// produces a single undo entry. Close it with `CommitUndo`.
    BeginUndo {
        label: String,
    },
    /// API v6: close the transaction opened by `BeginUndo` (commits the entry).
    CommitUndo,
}

/// Responses the host sends back for `PluginRequest`.
#[derive(Debug, Serialize, Deserialize)]
pub enum PluginResponse {
    Ok,
    Bool(bool),
    Handle(Handle),
    Record(Option<ExtendedDataRecord>),
    Document(Box<CadDocument>),
    Error(String),
    /// Path to the memory-mapped file and the current snapshot version.
    DocumentView {
        path: String,
        version: u64,
    },
    Handles(Vec<Handle>),
    /// V4: path to the tab-keyed memory-mapped file and current version.
    DocumentViewV4 {
        path: String,
        version: u64,
    },
    /// V4: stable tab identifier of the active tab.
    TabId(u64),
    /// V5: filesystem path of the document in the requested tab, if any.
    DocumentPath(Option<std::ffi::OsString>),
    // ── upstream v6/v7 additions first, then the fork's own additions ───────
    // (ordering keeps upstream's bincode discriminant indices intact; the fork's
    // plugin is rebuilt with this crate. See docs/fork-patches.md §A-1.)
    /// Optional entity handle (e.g. from AddLayer).
    OptHandle(Option<Handle>),
    SystemVariable(Option<HostSettingValue>),
    SystemVariableResult(Result<HostSettingValue, String>),
    EntityTransactionResult(Result<(), String>),
    Selection(Vec<Handle>),
    SelectionResult(Result<(), String>),
    SolidResult(Result<Handle, String>),
    TableResult(Result<Handle, String>),
    CommandResult(Result<crate::host::CommandOutcome, String>),
    /// API v5: count returned by `ensure_layers` / `ensure_linetypes` /
    /// `ensure_text_styles` (number of entries created).
    Count(usize),
    /// API v5: pending frame selection from the picker modal.
    FrameSelection(Option<crate::host::FrameSelection>),
    /// API v5: result of a frame block import (ATTDEFs in draw order).
    ImportFrameBlock(Result<Vec<codec::entities::AttributeDefinition>, String>),
}

/// Messages sent from the host to the plugin runner.
#[derive(Debug, Serialize, Deserialize)]
pub enum HostToPlugin {
    Request(HostRequest),
    Response(Box<PluginResponse>),
}

/// Messages sent from the plugin runner to the host.
#[derive(Debug, Serialize, Deserialize)]
pub enum PluginToHost {
    Request(Box<PluginRequest>),
    Response(HostResponse),
}

/// Convenience helper for manifest serialization.
impl From<&'static crate::manifest::PluginManifest> for OwnedPluginManifest {
    fn from(m: &'static crate::manifest::PluginManifest) -> Self {
        Self {
            id: m.id.to_string(),
            name: m.name.to_string(),
            version: m.version.to_string(),
            description: m.description.to_string(),
            api_version: m.api_version.major,
            ribbon_order: m.ribbon_order,
            xdata_apps: m.xdata_apps.iter().map(|s| s.to_string()).collect(),
            command_prefixes: m.command_prefixes.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl OwnedPluginManifest {
    pub fn api_version(&self) -> ApiVersion {
        ApiVersion {
            major: self.api_version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lt() -> crate::host::LinetypeDef {
        crate::host::LinetypeDef {
            name: "X".into(),
            description: "d".into(),
            elements: vec![1.0, -2.0],
        }
    }

    fn layer() -> crate::host::LayerDef {
        crate::host::LayerDef {
            name: "A".into(),
            color: codec::types::Color::from_index(1),
            linetype: "Continuous".into(),
            lineweight: codec::types::LineWeight::from_value(25),
            plottable: true,
            off: false,
        }
    }

    fn style() -> crate::host::TextStyleDef {
        crate::host::TextStyleDef {
            name: "S".into(),
            font_file: "f".into(),
            big_font_file: String::new(),
            true_type_font: "t".into(),
            height: 3.5,
            width_factor: 0.7,
            annotative: true,
            is_shape_file: false,
            is_vertical: false,
        }
    }

    fn frame_item() -> crate::host::FrameItem {
        crate::host::FrameItem {
            path: "/a.dwg".into(),
            label: "a".into(),
        }
    }

    fn dim_style() -> crate::host::DimStyleDef {
        crate::host::DimStyleDef {
            name: "OCSM_GB".into(),
            make_current: true,
            dimtxt: 2.5,
            dimasz: 2.5,
            dimcen: 2.5,
            dimexe: 2.0,
            dimexo: 0.0625,
            dimgap: 1.0,
            dimdli: 10.0,
            dimdle: 0.0,
            dimscale: 1.0,
            dimtad: 1,
            dimjust: 0,
            dimdec: 4,
            dimlunit: 2,
            dimzin: 8,
            dimaunit: 0,
            dimadec: 0,
            dimtih: false,
            dimtoh: true,
            dimtix: false,
            dimsoxd: false,
            dimtofl: true,
            dimtol: false,
            dimtp: 0.0,
            dimtm: 0.0,
            dimtdec: 2,
            dimclrd: 130,
            dimclre: 130,
            dimclrt: 3,
            dimlwd: -1,
            dimlwe: -1,
            dimtxsty: "OCSM_GB".into(),
            dimpost: String::new(),
            dimlfac: 1.0,
            dimtfac: 0.71,
            dimazin: 0,
            dimfrac: 0,
            dimtmove: 0,
            annotative: false,
        }
    }

    #[test]
    fn v5_plugin_request_variants_round_trip_stably() {
        // bincode 序列化往返必须稳定（插件与宿主可能不同版本编译）。
        let reqs = vec![
            PluginRequest::SelectedHandles,
            PluginRequest::SetCurrentLayer("1轮廓实线层".into()),
            PluginRequest::EnsureLayers(vec![layer()]),
            PluginRequest::EnsureLinetypes(vec![lt()]),
            PluginRequest::EnsureTextStyles(vec![style()]),
            PluginRequest::ShowFramePicker(vec![frame_item()]),
            PluginRequest::TakePendingFrameSelection,
            PluginRequest::ImportFrameBlock(crate::host::ImportFrameBlockRequest {
                path: "/a.dwg".into(),
                block_name: "a".into(),
            }),
            PluginRequest::EnsureDimStyles(vec![dim_style()]),
        ];
        for req in reqs {
            let bytes = bincode::serialize(&req).unwrap();
            let back: PluginRequest = bincode::deserialize(&bytes).unwrap();
            assert_eq!(bincode::serialize(&back).unwrap(), bytes);
        }
    }

    #[test]
    fn v5_interactive_events_round_trip_with_snap_text_and_mouse() {
        let events = vec![
            InteractiveEvent::ObjectPick {
                handle: Handle::new(7),
                pt: [1.0, 2.0, 0.0],
                snapped: true,
            },
            InteractiveEvent::Text("A".into()),
            InteractiveEvent::MouseMove([3.0, 4.0, 0.0]),
            InteractiveEvent::Point([5.0, 6.0, 0.0]),
            InteractiveEvent::Enter,
        ];
        for ev in events {
            let bytes = bincode::serialize(&ev).unwrap();
            let back: InteractiveEvent = bincode::deserialize(&bytes).unwrap();
            assert_eq!(bincode::serialize(&back).unwrap(), bytes);
        }
    }

    #[test]
    fn v5_host_requests_wants_queries_round_trip() {
        let reqs = vec![
            HostRequest::WantsTextInput { command_id: 1 },
            HostRequest::WantsMouseMove { command_id: 2 },
            HostRequest::EntityPickOsnap { command_id: 3 },
        ];
        for req in reqs {
            let bytes = bincode::serialize(&req).unwrap();
            let back: HostRequest = bincode::deserialize(&bytes).unwrap();
            assert_eq!(bincode::serialize(&back).unwrap(), bytes);
        }
    }

    #[test]
    fn v5_preview_response_round_trips_entity() {
        let preview = HostResponse::Preview(Some(EntityType::Line(codec::entities::Line::new())));
        let bytes = bincode::serialize(&preview).unwrap();
        let back: HostResponse = bincode::deserialize(&bytes).unwrap();
        assert_eq!(bincode::serialize(&back).unwrap(), bytes);
        assert!(matches!(
            back,
            HostResponse::Preview(Some(EntityType::Line(_)))
        ));
    }

    #[test]
    fn v5_plugin_response_variants_round_trip_stably() {
        let resps = vec![
            PluginResponse::Count(3),
            PluginResponse::FrameSelection(Some(crate::host::FrameSelection {
                path: "p".into(),
                scale_v1: 1,
                scale_v2: 2,
            })),
            PluginResponse::FrameSelection(None),
            PluginResponse::ImportFrameBlock(Ok(vec![])),
            PluginResponse::ImportFrameBlock(Err("boom".into())),
        ];
        for resp in resps {
            let bytes = bincode::serialize(&resp).unwrap();
            let back: PluginResponse = bincode::deserialize(&bytes).unwrap();
            assert_eq!(bincode::serialize(&back).unwrap(), bytes);
        }
    }
}
