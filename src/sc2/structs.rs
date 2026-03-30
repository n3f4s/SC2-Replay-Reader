use num_bigint::{ BigInt, BigUint };
use num_derive::{ FromPrimitive, ToPrimitive };
use std::collections::HashMap;
use std::fmt;

use crate::utils::*;

#[derive(Clone, Debug)]
pub enum DataType {
    Array(Vec<Box<DataType>>),
    VInt(BigInt),
    Blob(Vec<u8>),
    Optional(Option<Box<DataType>>),
    HashMap(HashMap<BigInt, Box<DataType>>),
    UInt8(u8),
    UInt32(u32),
    UInt64(u64),
    Str(String),
}

impl DataType {
    fn pretty_print(&self, level: usize) -> String {
        use DataType::*;
        match self { // FIXME handle level
            UInt8(i)  => format!("{}_u8", i),
            UInt32(i) => format!("{}_u32", i),
            UInt64(i) => format!("{}_u64", i),
            Str(s) => format!("\"{}\"", s),
            Blob(v) => {
                let b = BigUint::from_bytes_le(v);
                format!("{}_bytes", b)
            }
            VInt(v) => format!("{}_vint", v),
            Optional(Some(b)) => format!("Some({})", b.pretty_print(level)),
            Optional(None) => String::from("None"),
            Array(vec) => {
                let str_ = vec.iter().map(|d| "  ".repeat(level) + &d.pretty_print(level+1) + ",\n").collect::<Vec<String>>().join("");
                format!("[\n{}{}]", str_, "  ".repeat(level))
            },
            HashMap(hash) => {
                let str_ = hash
                    .iter()
                    .map(|( k, v )| format!("{}{}: {},\n",
                                            "  ".repeat(level+1),
                                            k,
                                            v.pretty_print(level+2))).collect::<Vec<String>>().join("");
                format!("{{\n{}{}}}", str_, "  ".repeat(if level > 0 { level-1 } else { level }))
            }
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pretty_print(0))
    }
}

#[derive(Debug)]
pub struct BNet {
    pub region: BigInt,
    pub program_id: u32,
    pub subregion: BigInt,
    pub uid: BigInt
}

#[derive(Debug)]
pub struct Color {
    pub a: BigInt,
    pub r: BigInt,
    pub g: BigInt,
    pub b: BigInt,
}

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub bnet: BNet,
    pub race: String,
    pub color: Color,
    pub control: BigInt,
    pub team: BigInt,
    pub handicap: BigInt,
    pub observe: BigInt,
    pub result: BigInt,
}

#[derive(Debug)]
pub struct Details {
    pub players: Vec<Player>,
    pub map_name: String,
    pub difficulty: String,
    pub thumbnail: String,
    pub blizz_map: u8,
    pub file_time: BigInt,
    pub utc_adjustment: BigInt,
    pub description: String,
    pub image_file_path: String,
    pub map_file_name: String,
    pub cache_handle: Vec<String>,
    pub mini_save: u8,
    pub default_difficulty: BigInt,
    pub game_speed: BigInt,
}

#[derive(FromPrimitive, ToPrimitive)]
pub enum Events {
    UnknownEvent                                             = 0,
    FinishedLoadingSyncEvent                                 = 5,
    BankFileEvent                                            = 7,
    BankSectionEvent                                         = 8,
    BankKeyEvent                                             = 9,
    BankValueEvent                                           = 10,
    BankSignatureEvent                                       = 11,
    UserOptionsEvent                                         = 12,
    SaveGameEvent                                            = 22,
    SaveGameDoneEvent                                        = 23,
    PlayerLeaveEvent                                         = 25,
    GameCheatEvent                                           = 26,
    CommandEvent                                             = 27,
    SelectionDeltaEvent                                      = 28,
    ControlGroupUpdateEvent                                  = 29,
    SelectionSyncCheckEvent                                  = 30,
    ResourceTradeEvent                                       = 31,
    TriggerChatMessageEvent                                  = 32,
    AiCommunicateEvent                                       = 33,
    SetAbsoluteGameSpeedEvent                                = 34,
    AddAbsoluteGameSpeedEvent                                = 35,
    BroadcastCheatEvent                                      = 37,
    AllianceEvent                                            = 38,
    UnitClickEvent                                           = 39,
    UnitHighlightEvent                                       = 40,
    TriggerReplySelectedEvent                                = 41,
    TriggerSkippedEvent                                      = 44,
    TriggerSoundLengthQueryEvent                             = 45,
    TriggerSoundOffsetEvent                                  = 46,
    TriggerTransmissionOffsetEvent                           = 47,
    TriggerTransmissionCompleteEvent                         = 48,
    CameraUpdateEvent                                        = 49,
    TriggerAbortMissionEvent                                 = 50,
    TriggerPurchaseMadeEvent                                 = 51,
    TriggerPurchaseExitEvent                                 = 52,
    TriggerPlanetMissionLaunchedEvent                        = 53,
    TriggerPlanetPanelCanceledEvent                          = 54,
    TriggerDialogControlEvent                                = 55,
    TriggerSoundLengthSyncEvent                              = 56,
    TriggerConversationSkippedEvent                          = 57,
    TriggerMouseClickedEvent                                 = 58,
    TriggerPlanetPanelReplayEvent                            = 63,
    TriggerSoundtrackDoneEvent                               = 64,
    TriggerPlanetMissionSelectedEvent                        = 65,
    TriggerKeyPressedEvent                                   = 66,
    TriggerMovieFunctionEvent                                = 67,
    TriggerPlanetPanelBirthCompleteEvent                     = 68,
    TriggerPlanetPanelDeathCompleteEvent                     = 69,
    ResourceRequestEvent                                     = 70,
    ResourceRequestFulfillEvent                              = 71,
    ResourceRequestCancelEvent                               = 72,
    TriggerResearchPanelExitEvent                            = 73,
    TriggerResearchPanelPurchaseEvent                        = 74,
    TriggerResearchPanelSelectionChangedEvent                = 75,
    LagMessageEvent                                          = 76,
    TriggerMercenaryPanelExitEvent                           = 77,
    TriggerMercenaryPanelPurchaseEvent                       = 78,
    TriggerMercenaryPanelSelectionChangedEvent               = 79,
    TriggerVictoryPanelExitEvent                             = 80,
    TriggerBattleReportPanelExitEvent                        = 81,
    TriggerBattleReportPanelPlayMissionEvent                 = 82,
    TriggerBattleReportPanelPlaySceneEvent                   = 83,
    TriggerBattleReportPanelSelectionChangedEvent            = 84,
    TriggerVictoryPanelPlayMissionAgainEvent                 = 85,
    TriggerMovieStartedEvent                                 = 86,
    TriggerMovieFinishedEvent                                = 87,
    DecrementGameTimeRemainingEvent                          = 88,
    TriggerPortraitLoadedEvent                               = 89,
    TriggerCustomDialogDismissedEvent                        = 90,
    TriggerGameMenuItemSelectedEvent                         = 91,
    TriggerCameraMoveEvent                                   = 92,
    TriggerPurchasePanelSelectedPurchaseItemChangedEvent     = 93,
    TriggerPurchasePanelSelectedPurchaseCategoryChangedEvent = 94,
    TriggerButtonPressedEvent                                = 95,
    TriggerGameCreditsFinishedEvent                          = 96,
}

#[derive(Debug)]
pub struct PlayerEvent {}

#[derive(Debug)]
pub struct Event {
    pub player: Option<PlayerEvent>,
    pub frame: u64,
    pub second: u64,
    pub is_local: bool,
    pub pid: u64,
    pub name: String,
    pub event_id: u64,
}
