pub mod external {
    pub use super::hik::{dublicates, *};

    use diesel_db::MultipleSettingsData;
    use domain::{stream::Resource, CameraId};
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    pub const DEFAULT_TIMEOUT: u64 = 4;

    #[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
    #[schema(as = api::source::HikvisionConfiguration)]
    pub struct HikvisionConfiguration {
        //Hikvision config
        pub external_projector: bool, // 5 line
        pub internal_projector: bool, // 7 line
        // One line only
        pub default_switch: bool,

        #[schema(value_type = api::source::ImageChannel)]
        pub image_channel: Option<ImageChannel>,
        #[schema(value_type = api::source::StreamingChannel)]
        pub streaming_channel: Option<StreamingChannel>,
    }

    impl Default for HikvisionConfiguration {
        fn default() -> Self {
            Self {
                external_projector: true,
                internal_projector: true,
                default_switch: true,

                image_channel: None,
                streaming_channel: None,
            }
        }
    }

    impl HikvisionConfiguration {
        pub fn is_color_correct(&self) -> Option<bool> {
            let color = self.image_channel.as_ref()?.color.as_ref()?;

            Some(
                color.saturation_level > 0
                    && color.brightness_level > 0
                    && color.contrast_level > 0,
            )
        }
    }

    #[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
    #[serde(rename_all = "snake_case")]
    #[schema(as = api::source::SpotlightMode)]
    pub enum SpotlightMode {
        Off,
        AlwaysOn,
        Strobe,
        ExposureActive,
        FrameTriggerWait,
        AcquisitionTriggerWait,
    }

    #[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
    #[schema(as = api::source::SpotlightConfiguration)]
    pub struct SpotlightConfiguration {
        pub io_line: usize,
        #[schema(value_type = api::source::SpotlightMode)]
        pub mode: SpotlightMode,
    }

    impl Default for SpotlightConfiguration {
        fn default() -> Self {
            Self {
                io_line: 0,
                mode: SpotlightMode::Off,
            }
        }
    }

    #[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
    #[schema(as = api::source::AdditionalConfiguration)]
    pub struct AdditionalConfiguration {
        //`Common` config
        #[schema(value_type = usize)]
        pub id: CameraId,
        pub is_day_now: Option<bool>,
        pub default_settings: Option<bool>,

        //`Basler` and `Daheng` config
        #[schema(value_type = api::source::SpotlightConfiguration)]
        pub spotlight: Option<SpotlightConfiguration>,
        //`Hikvision` config
        #[schema(value_type = api::source::HikvisionConfiguration)]
        pub hikvision: Option<HikvisionConfiguration>,
    }

    impl Default for AdditionalConfiguration {
        fn default() -> Self {
            Self {
                id: 0,
                is_day_now: None,
                default_settings: Some(false),

                spotlight: Some(Default::default()),
                hikvision: None,
            }
        }
    }

    impl From<&Resource> for AdditionalConfiguration {
        fn from(value: &Resource) -> Self {
            Self {
                id: value.id,
                is_day_now: None,
                default_settings: None,

                spotlight: None,
                hikvision: None,
            }
        }
    }

    impl AdditionalConfiguration {
        pub fn new(id: CameraId) -> Self {
            Self {
                id,
                ..Default::default()
            }
        }

        pub fn is_empty(&self) -> bool {
            self.spotlight.is_none()
                && self.hikvision.is_none()
                && self.is_day_now.is_none()
                && self.default_settings.is_none()
        }

        pub fn empty(id: CameraId) -> Self {
            Self {
                id,
                spotlight: None,
                hikvision: None,
                is_day_now: None,
                default_settings: None,
            }
        }

        pub fn get_simple_hik_configuration(&self) -> Option<HikvisionConfiguration> {
            let mut hik = self.hikvision.clone()?;

            hik.image_channel = None;
            hik.streaming_channel = None;

            Some(hik)
        }
    }

    impl MultipleSettingsData for AdditionalConfiguration {
        type Id = CameraId;

        fn get_settings_id(&self) -> Self::Id {
            self.id
        }
    }
}

pub mod axis {
    use serde::ser::SerializeMap;
    use serde::{Deserialize, Serialize, Serializer};

    #[derive(Default)]
    pub struct ApiVersion;

    impl Serialize for ApiVersion {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            "1.0".serialize(serializer)
        }
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    #[serde(tag = "method", content = "params")]
    pub enum RequestParams<P: AsRef<[Port]> + Serialize> {
        GetPorts,
        SetPorts { ports: P },
    }

    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    #[serde(rename_all = "camelCase")]
    pub struct Response<D> {
        pub api_version: String,
        pub method: String,
        pub data: D,
    }

    impl<P: AsRef<[Port]> + Serialize> Default for RequestParams<P> {
        fn default() -> Self {
            Self::GetPorts
        }
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Port {
        pub port: &'static str,
        // usage: String,
        // direction: PortDirection, //"input"|"output",
        // name: String,
        pub normal_state: PortState,
        pub state: PortState,
    }

    #[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
    #[serde(rename_all = "camelCase")]
    pub enum PortState {
        Open,
        Closed,
    }

    impl From<bool> for PortState {
        fn from(value: bool) -> Self {
            use PortState::*;

            match value {
                true => Closed,
                _ => Open,
            }
        }
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ProjectorsItem {
        pub port: String,
        pub state: PortState,
        pub normal_state: PortState,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ProjectorsData {
        #[allow(dead_code)]
        pub number_of_ports: u8,
        pub items: Vec<ProjectorsItem>,
    }

    #[derive(Deserialize, Debug)]
    pub struct SwitchData {
        #[allow(dead_code)]
        ports: [String; 1],
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ApiRequest<P: AsRef<[Port]> + Serialize> {
        pub api_version: ApiVersion,
        #[serde(flatten)]
        pub method: RequestParams<P>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SetTimeZoneRequest {
        pub time_zone: String,
    }

    pub struct SetDateTimeRequest {
        pub date_time: chrono::NaiveDateTime,
    }

    impl Serialize for SetDateTimeRequest {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut map = serializer.serialize_map(Some(1))?;

            let date_time = self.date_time.format("%FT%TZ").to_string();

            map.serialize_entry("dateTime", &date_time)?;

            map.end()
        }
    }

    impl From<SetDateTimeRequest> for GenericApiRequest<SetDateTimeRequest> {
        fn from(r: SetDateTimeRequest) -> Self {
            GenericApiRequest {
                api_version: ApiVersion,
                method: "setDateTime",
                params: r,
            }
        }
    }

    impl From<SetTimeZoneRequest> for GenericApiRequest<SetTimeZoneRequest> {
        fn from(r: SetTimeZoneRequest) -> Self {
            GenericApiRequest {
                api_version: ApiVersion,
                method: "setTimeZone",
                params: r,
            }
        }
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GenericApiRequest<P: Serialize + Send + 'static> {
        pub api_version: ApiVersion,
        pub method: &'static str,
        pub params: P,
    }

    impl<P: AsRef<[Port]> + Serialize> From<RequestParams<P>> for ApiRequest<P> {
        fn from(method: RequestParams<P>) -> Self {
            Self {
                method,
                api_version: ApiVersion,
            }
        }
    }
}
pub mod hik {
    use crate::FocusValue;
    use onvif::FpsValue;
    use serde::{
        ser::{SerializeStruct, Serializer},
        Deserialize, Serialize,
    };
    use utoipa::ToSchema;

    use std::fmt::Display;
    use thiserror::Error;

    const NIGHT_TO_DAY_FILTER_LEVEL_PTZ: u32 = 2;

    pub mod dublicates {
        use serde::{Deserialize, Serialize};
        use utoipa::ToSchema;

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
        #[serde(rename_all = "camelCase")]
        #[schema(as = api::source::dublicates::AdvancedMode)]
        pub struct AdvancedMode {
            pub spatial_level: i32,
            pub temporal_level: i32,
        }

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "lowercase")]
        pub enum TimeMode {
            #[serde(rename = "ALL")]
            ALL,
            MANUAL,
            #[serde(rename = "NTP")]
            NTP,
            LOCAL,
            SATELLITE,
            TIMECORRECT,
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::Defog)]
    pub struct Defog {
        pub enabled: bool,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct ControlAddress {
        pub enabled: bool,
        #[serde(rename = "Address")]
        pub address: Option<String>,
    }
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PTZRs485Para {
        pub baud_rate: i32,
        pub data_bits: i32,
        pub parity_type: String,
        pub stop_bits: i32,
        pub flow_ctrl: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PTZChannel {
        pub id: bool,
        pub enabled: bool,
        pub serial_number: Option<i32>,
        #[serde(rename = "videoInputID")]
        pub video_input_id: i32,
        pub pax_max_speed: Option<i32>,
        pub tilt_max_speed: Option<i32>,
        pub preset_speed: Option<i32>,
        pub auto_patrol_speed: Option<i32>,
        pub key_board_control_speed: Option<String>,
        pub control_protocol: Option<String>,
        pub control_address: Option<ControlAddress>,
        #[serde(rename = "defaultPresetID")]
        pub default_preset_id: Option<String>,
        #[serde(rename = "PTZRs485Para")]
        pub ptz_rs_485_para: Option<PTZRs485Para>,
        pub manual_control_speed: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::NoiseReduce2D)]
    #[serde(rename_all = "camelCase")]
    pub struct NoiseReduce2D {
        #[serde(rename = "noiseReduce2DEnable")]
        pub noise_reduce_2d_enable: bool,
        #[serde(rename = "noiseReduce2DLevel")]
        pub noise_reduce_2d_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::FlipAngle)]
    pub enum FlipAngle {
        #[serde(rename = "90")]
        _90,
        #[serde(rename = "180")]
        _180,
        #[serde(rename = "270")]
        _270,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::ImageFlipStyle)]
    pub enum ImageFlipStyle {
        LEFTRIGHT,
        UPDOWN,
        CENTER,
        AUTO,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::ImageFlip)]
    pub struct ImageFlip {
        pub enabled: bool,
        #[serde(rename = "ImageFlipStyle")]
        #[schema(value_type = api::source::ImageFlipStyle)]
        pub image_flip_style: Option<ImageFlipStyle>,
        #[schema(value_type = api::source::FlipAngle)]
        pub flip_angle: Option<FlipAngle>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "lowercase")]
    #[schema(as = api::source::WDRMode)]
    pub enum WDRMode {
        OPEN,
        CLOSE,
        AUTO,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::WDR)]
    pub struct WDR {
        #[schema(value_type = api::source::WDRMode)]
        pub mode: WDRMode,
        #[serde(rename = "WDRLevel")]
        pub wdr_level: Option<i32>,
        #[serde(rename = "WDRContrastLevel")]
        pub wdr_contrast_level: Option<i32>,
        #[serde(rename = "WDRLevel1")]
        pub wdr_level1: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::BLCMode)]
    pub enum BLCMode {
        UP,
        DOWN,
        LEFT,
        RIGHT,
        CENTER,
        #[serde(rename = "MULTI-AREA")]
        MULTIAREA,
        Region,
        AUTO,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::RegionCoordinates)]
    pub struct RegionCoordinates {
        #[serde(rename = "positionX")]
        pub position_x: i32,
        #[serde(rename = "positionY")]
        pub position_y: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::RegionCoordinatesList)]
    pub struct RegionCoordinatesList {
        #[serde(rename = "RegionCoordinates")]
        #[schema(value_type = Vec<api::source::RegionCoordinates>)]
        pub region_coordinates: Vec<RegionCoordinates>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::BLCRegion)]
    pub struct BLCRegion {
        pub id: i32,
        #[serde(rename = "RegionCoordinatesList")]
        #[schema(value_type = Vec<api::source::RegionCoordinatesList>)]
        pub region_coordinates_list: Vec<RegionCoordinatesList>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::BLCRegionList)]
    pub struct BLCRegionList {
        #[serde(rename = "BLCRegion")]
        #[schema(value_type = Vec<api::source::BLCRegion>)]
        pub blc_region: Option<Vec<BLCRegion>>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::BLC)]
    pub struct BLC {
        pub enabled: bool,
        #[serde(rename = "BLCMode")]
        #[schema(value_type = api::source::BLCMode)]
        pub blc_mode: Option<BLCMode>,
        #[serde(rename = "BLCLevel")]
        pub blc_level: Option<i32>,
        #[serde(rename = "BLCRegionList")]
        #[schema(value_type = api::source::BLCRegionList)]
        pub blc_region_list: Option<BLCRegionList>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::AdvancedMode)]
    pub struct AdvancedMode {
        pub frame_noise_reduce_level: i32,
        pub inter_frame_noise_reduce_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::GeneralMode)]
    pub struct GeneralMode {
        pub general_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::NoiseReduce)]
    pub struct NoiseReduce {
        #[serde(rename = "mode")]
        #[schema(value_type = api::source::NoiseReduceMode)]
        pub mode: NoiseReduceMode,
        #[schema(value_type = api::source::GeneralMode)]
        pub general_mode: Option<GeneralMode>,
        #[schema(value_type = api::source::BLCRegionList)]
        pub advanced_mode: Option<AdvancedMode>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "lowercase")]
    #[schema(as = api::source::WhiteBalanceStyle)]
    pub enum WhiteBalanceStyle {
        AUTO,
        MANUAL,
        INDOOR,
        OUTDOOR,
        AUTOTRACE,
        ONECE,
        SODIUMLIGHT,
        MERCURYLIGHT,
        AUTO0,
        AUTO1,
        AUTO2,
        FLUORESCENT,
        NATURALLIGHT,
        WARM,
        INCANDESCENT,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::WhiteBalance)]
    pub struct WhiteBalance {
        #[schema(value_type = api::source::WhiteBalanceStyle)]
        pub white_balance_style: WhiteBalanceStyle,
        #[serde(rename = "whiteBalanceLevel")]
        pub white_balance_level: Option<i32>,
        pub white_blance_red: Option<i32>,
        pub white_blance_blue: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::ExposureType)]
    pub enum ExposureType {
        #[serde(rename = "auto")]
        AUTO,
        #[serde(rename = "IrisFirst")]
        IRISFIRST,
        #[serde(rename = "ShutterFirst")]
        SHUTTERFIRST,
        #[serde(rename = "gainFirst")]
        GAINFIRST,
        #[serde(rename = "manual")]
        MANUAL,
        #[serde(rename = "plris")]
        PLRIS,
        #[serde(rename = "T5280-PQ1")]
        T5280PQ1,
        #[serde(rename = "T5289-PQ1")]
        T5289PQ1,
        #[serde(rename = "T1140-PQ1")]
        T1140PQ1,
        #[serde(rename = "T2712-PQ1")]
        T2712PQ1,
        #[serde(rename = "HV1250P-MPIR")]
        HV1250PMPIR,
        #[serde(rename = "plris-General")]
        PLRISGENERAL,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::OverexposeSuppressType)]
    pub enum OverexposeSuppressType {
        AUTO,
        MANUAL,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, ToSchema)]
    #[schema(as = api::source::OverexposeSuppress)]
    pub struct OverexposeSuppress {
        pub enabled: bool,
        #[serde(rename = "Type")]
        #[schema(value_type = api::source::OverexposeSuppressType)]
        pub ost: Option<OverexposeSuppressType>,
        #[serde(rename = "DistanceLevel")]
        pub distance_level: Option<i32>,
        #[serde(rename = "shortIRDistanceLevel")]
        pub short_ir_distance_level: Option<i32>,
        #[serde(rename = "longIRDistanceLevel")]
        pub long_ir_distance_level: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::Plris)]
    pub struct Plris {
        #[serde(rename = "plrisType")]
        #[schema(value_type = api::source::OverexposeSuppressType)]
        pub plris_type: Option<OverexposeSuppressType>,
        #[serde(rename = "IrisLevel")]
        pub iris_level: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::PlrisGeneral)]
    pub struct PlrisGeneral {
        pub iris_level: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::FaceExposure)]
    pub struct FaceExposure {
        pub enabled: Option<bool>,
        pub sensitivity: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::Exposure)]
    pub struct Exposure {
        #[schema(value_type = api::source::ExposureType)]
        pub exposure_type: ExposureType,
        #[serde(rename = "autoIrisLevel")]
        pub auto_iris_level: Option<i32>,
        #[schema(value_type = api::source::OverexposeSuppress)]
        pub overexpose_suppress: Option<OverexposeSuppress>,
        #[serde(rename = "plris")]
        #[schema(value_type = api::source::Plris)]
        pub plris: Option<Plris>,
        #[schema(value_type = api::source::PlrisGeneral)]
        pub plris_general: Option<PlrisGeneral>,
        #[serde(rename = "exposureLevel")]
        pub exposure_level: Option<i32>,
        #[serde(rename = "faceExposure")]
        #[schema(value_type = api::source::FaceExposure)]
        pub face_exposure: Option<FaceExposure>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::GammaCorrection)]
    pub struct GammaCorrection {
        pub gamma_correction_enabled: bool,
        pub gamma_correction_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::Sharpness)]
    pub struct Sharpness {
        #[schema(value_type = api::source::OverexposeSuppressType)]
        pub sharpness_mode: Option<OverexposeSuppressType>,
        pub sharpness_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::PowerLineFrequencyMode)]
    pub enum PowerLineFrequencyMode {
        #[serde(rename = "50hz")]
        _50HZ,
        #[serde(rename = "60hz")]
        _60HZ,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::PowerLineFrequency)]
    pub struct PowerLineFrequency {
        #[schema(value_type = api::source::PowerLineFrequencyMode)]
        pub power_line_frequency_mode: Option<PowerLineFrequencyMode>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "lowercase")]
    #[schema(as = api::source::ImageModeType)]
    pub enum ImageModeType {
        STANDARD,
        INDOOR,
        OUTDOOR,
        #[serde(rename = "dimLight")]
        DIMLIGHT,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::Recommendation)]
    pub struct Recommendation {
        pub brightness_level: Option<i32>,
        pub contrast_level: Option<i32>,
        pub sharpness_level: Option<i32>,
        pub saturation_level: Option<i32>,
        pub hue_level: Option<i32>,
        pub de_noise_level: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::ImageMode)]
    pub struct ImageMode {
        #[serde(rename = "type")]
        #[schema(value_type = api::source::ImageModeType)]
        pub imt: ImageModeType,
        #[serde(rename = "recommendation")]
        #[schema(value_type = api::source::Recommendation)]
        pub recommendation: Option<Recommendation>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::BrightEnhance)]
    pub struct BrightEnhance {
        pub bright_enhance_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::ImageModeList)]
    pub struct ImageModeList {
        #[schema(value_type = Vec<api::source::ImageMode>)]
        pub image_mode: Vec<ImageMode>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::TimeRange)]
    pub struct TimeRange {
        pub begin_time: String,
        pub end_time: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::Schedule)]
    pub struct Schedule {
        #[schema(value_type = api::source::ScheduleType)]
        pub schedule_type: ScheduleType,
        #[serde(rename = "TimeRange")]
        #[schema(value_type = api::source::TimeRange)]
        pub time_range: TimeRange,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "lowercase")]
    #[schema(as = api::source::ScheduleType)]
    pub enum ScheduleType {
        DAY,
        NIGHT,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "lowercase")]
    #[schema(as = api::source::ISPModeType)]
    pub enum ISPModeType {
        AUTO,
        SCHEDULE,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::ISPMode)]
    pub struct ISPMode {
        #[serde(rename = "mode")]
        #[schema(value_type = api::source::ISPModeType)]
        pub mode: ISPModeType,
        #[schema(value_type = api::source::Schedule)]
        pub schedule: Option<Schedule>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::Shutter)]
    pub struct Shutter {
        pub shutter_level: String,
        #[serde(rename = "maxShutterLevelLimit")]
        pub max_shutter_level_limit: Option<String>,
        #[serde(rename = "minShutterLevelLimit")]
        pub min_shutter_level_limit: Option<String>,
    }

    #[derive(Debug, Clone, Deserialize, PartialEq, Serialize, ToSchema)]
    #[schema(as = api::source::PTZ)]
    pub struct PTZ {
        pub enabled: bool,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::FocusConfiguration)]
    pub struct FocusConfiguration {
        pub focus_style: Option<String>,
        pub focus_limited: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::LensInitialization)]
    pub struct LensInitialization {
        pub enabled: bool,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::DSS)]
    pub struct DSS {
        pub enabled: bool,
        #[serde(rename = "DSSLevel")]
        pub dss_level: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::IrLight)]
    pub struct IrLight {
        pub mode: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::ZoomLimit)]
    pub struct ZoomLimit {
        pub zoom_limit_ratio: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::Iris)]
    pub struct Iris {
        pub iris_level: Option<i32>,
        #[serde(rename = "maxIrisLevelLimit")]
        pub max_iris_level_limit: Option<i32>,
        #[serde(rename = "minIrisLevelLimit")]
        pub min_iris_level_limit: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::ImageFreeze)]
    pub struct ImageFreeze {
        pub enabled: bool,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::Proportionalpan)]
    pub struct Proportionalpan {
        pub enabled: bool,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LaserLight {
        pub mode: Option<String>,
        pub brightness_level: Option<i32>,
        pub laserangle: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::EIS)]
    pub struct EIS {
        pub enabled: bool,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum ShutterLevel {
        #[serde(rename = "1")]
        _1,
        #[serde(rename = "2")]
        _2,
        #[serde(rename = "3")]
        _3,
        #[serde(rename = "6")]
        _6,
        #[serde(rename = "12")]
        _12,
        #[serde(rename = "25")]
        _25,
        #[serde(rename = "50")]
        _50,
        #[serde(rename = "75")]
        _75,
        #[serde(rename = "100")]
        _100,
        #[serde(rename = "120")]
        _120,
        #[serde(rename = "125")]
        _125,
        #[serde(rename = "150")]
        _150,
        #[serde(rename = "175")]
        _175,
        #[serde(rename = "215")]
        _215,
        #[serde(rename = "225")]
        _225,
        #[serde(rename = "300")]
        _300,
        #[serde(rename = "400")]
        _400,
        #[serde(rename = "425")]
        _425,
        #[serde(rename = "600")]
        _600,
        #[serde(rename = "1000")]
        _1000,
        #[serde(rename = "1250")]
        _1250,
        #[serde(rename = "1750")]
        _1750,
        #[serde(rename = "2500")]
        _2500,
        #[serde(rename = "3500")]
        _3500,
        #[serde(rename = "6000")]
        _6000,
        #[serde(rename = "10000")]
        _10000,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::GrayScaleMode)]
    pub enum GrayScaleMode {
        #[serde(rename = "indoor")]
        INDOOR,
        #[serde(rename = "outdoor")]
        OUTDOOR,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::GrayScale)]
    pub struct GrayScale {
        #[serde(rename = "grayScaleMode")]
        #[schema(value_type = api::source::GrayScaleMode)]
        pub gray_scale_mode: GrayScaleMode,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::Color)]
    pub struct Color {
        pub brightness_level: i32,
        pub contrast_level: i32,
        pub saturation_level: i32,
        pub hue_level: Option<i32>,
        #[schema(value_type = api::source::GrayScale)]
        pub gray_scale: Option<GrayScale>,
        pub night_mode: Option<bool>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::GainWindow)]
    pub struct GainWindow {
        #[schema(value_type = api::source::RegionCoordinatesList)]
        pub region_coordinates_list: Option<RegionCoordinatesList>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::Gain)]
    pub struct Gain {
        pub gain_level: i32,
        #[schema(value_type = api::source::GainWindow)]
        pub gain_window: Option<GainWindow>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::ImageMultishut)]
    pub struct ImageMultishut {
        pub double_shut_enable: bool,
        pub codec_type: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::JPEGParam)]
    pub struct JPEGParam {
        #[serde(rename = "JPEGSize")]
        pub jpeg_size: Option<i32>,
        #[serde(rename = "MergeJPEGSize")]
        pub merge_jpeg_size: Option<i32>,
        #[serde(rename = "EXIFInformationEnabled")]
        pub exif_information_enabled: Option<bool>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::GrayRange)]
    pub struct GrayRange {
        pub gray_value_type: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::SnapColor)]
    pub struct SnapColor {
        pub brightness_level: i32,
        pub contrast_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::SnapShutter)]
    pub struct SnapShutter {
        pub snap_shutter_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::SnapWhiteBalance)]
    pub struct SnapWhiteBalance {
        pub white_balance_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::SnapGain)]
    pub struct SnapGain {
        pub snap_gain_level: i32,
        pub light_snap_gain_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::CarWindowEnhancement)]
    pub struct CarWindowEnhancement {
        pub enabled: bool,
        pub brighten_level: i32,
        pub defog_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::ITCImageSnap)]
    pub struct ITCImageSnap {
        #[schema(value_type = api::source::SnapColor)]
        pub snap_color: Option<SnapColor>,
        #[schema(value_type = api::source::SnapShutter)]
        pub snap_shutter: Option<SnapShutter>,
        #[schema(value_type = api::source::SnapGain)]
        pub snap_gain: Option<SnapGain>,
        #[schema(value_type = api::source::CarWindowEnhancement)]
        pub car_window_enhancement: Option<CarWindowEnhancement>,
        #[schema(value_type = api::source::SnapWhiteBalance)]
        pub snap_white_balance: Option<SnapWhiteBalance>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AdvancedModeExt {
        pub spatial_level: i32,
        pub temporal_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::RecordNoiseReduceExt)]
    pub struct RecordNoiseReduceExt {
        #[serde(rename = "mode")]
        #[schema(value_type = api::source::NoiseReduceMode)]
        pub mode: NoiseReduceMode,
        #[schema(value_type = api::source::GeneralMode)]
        pub general_mode: GeneralMode,
        #[schema(value_type = api::source::dublicates::AdvancedMode)]
        pub advanced_mode: dublicates::AdvancedMode,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::RecordGain)]
    pub struct RecordGain {
        pub gain_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::RecordShutter)]
    pub struct RecordShutter {
        pub shutter_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::RecordColor)]
    pub struct RecordColor {
        pub brightness_level: i32,
        pub contrast_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::ImageRecord)]
    pub struct ImageRecord {
        #[schema(value_type = api::source::RecordColor)]
        pub record_color: Option<RecordColor>,
        #[schema(value_type = api::source::RecordShutter)]
        pub record_shutter: Option<RecordShutter>,
        #[schema(value_type = api::source::RecordGain)]
        pub record_gain: Option<RecordGain>,
        #[schema(value_type = api::source::RecordNoiseReduceExt)]
        pub record_noise_reduce_ext: Option<RecordNoiseReduceExt>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "lowercase")]
    #[schema(as = api::source::DehazeMode)]
    pub enum DehazeMode {
        OPEN,
        CLOSE,
        AUTO,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[schema(as = api::source::CaptureMode)]
    pub struct CaptureMode {
        pub mode: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "lowercase")]
    #[schema(as = api::source::NoiseReduceMode)]
    pub enum NoiseReduceMode {
        CLOSE,
        GENERAL,
        ADVANCED,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::NoiseReduceExt)]
    pub struct NoiseReduceExt {
        #[serde(rename = "mode")]
        #[schema(value_type = api::source::NoiseReduceMode)]
        pub mode: NoiseReduceMode,
        #[schema(value_type = api::source::GeneralMode)]
        pub general_mode: GeneralMode,
        #[schema(value_type = api::source::dublicates::AdvancedMode)]
        pub advanced_mode: dublicates::AdvancedMode,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "lowercase")]
    #[schema(as = api::source::TempRangeMode)]
    pub enum TempRangeMode {
        AUTOMATIC,
        MANUAL,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::TempRange)]
    pub struct TempRange {
        #[schema(value_type = api::source::TempRangeMode)]
        pub mode: Option<TempRangeMode>,
        pub temperature_upper_limit: Option<i32>,
        pub temperature_lower_limit: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::Dehaze)]
    pub struct Dehaze {
        #[schema(value_type = api::source::DehazeMode)]
        pub dehaze_mode: Option<DehazeMode>,
        pub dehaze_level: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::PlateBright)]
    pub struct PlateBright {
        pub plate_bright_enabled: Option<bool>,
        pub plate_bright_sensitivity: Option<i32>,
        pub correct_factor_enabled: Option<bool>,
        pub correct_factor: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::HLC)]
    pub struct HLC {
        pub enabled: bool,
        #[serde(rename = "HLCLevel")]
        pub hlc_level: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::ImageChannel)]
    pub struct ImageChannel {
        pub id: i32,
        pub enabled: bool,
        #[serde(rename = "videoInputID")]
        pub video_input_id: Option<i32>,
        #[serde(rename = "Defog")]
        #[schema(value_type = api::source::Defog)]
        pub defog: Option<Defog>,
        #[serde(rename = "NoiseReduce2D")]
        #[schema(value_type = api::source::NoiseReduce2D)]
        pub noise_reduce_2d: Option<NoiseReduce2D>,
        #[serde(rename = "Focusconfiguration")]
        #[schema(value_type = api::source::FocusConfiguration)]
        pub focus_configuration: Option<FocusConfiguration>,
        #[serde(rename = "Lensinitialization")]
        #[schema(value_type = api::source::LensInitialization)]
        pub lens_initialization: Option<LensInitialization>,
        #[serde(rename = "ImageFlip")]
        #[schema(value_type = api::source::ImageFlip)]
        pub image_flip: Option<ImageFlip>,
        #[serde(rename = "ImageFreeze")]
        #[schema(value_type = api::source::ImageFreeze)]
        pub image_freeze: Option<ImageFreeze>, //unused
        #[serde(rename = "WDR")]
        #[schema(value_type = api::source::WDR)]
        pub wdr: Option<WDR>,
        #[serde(rename = "BLC")]
        #[schema(value_type = api::source::BLC)]
        pub blc: Option<BLC>,
        #[serde(rename = "NoiseReduce")]
        #[schema(value_type = api::source::NoiseReduce)]
        pub noise_reduce: Option<NoiseReduce>,
        #[serde(rename = "ImageEnhancement")]
        pub image_enhancement: Option<String>, //unused
        #[serde(rename = "DSS")]
        #[schema(value_type = api::source::DSS)]
        pub dss: Option<DSS>,
        #[serde(rename = "WhiteBalance")]
        #[schema(value_type = api::source::WhiteBalance)]
        pub white_balance: Option<WhiteBalance>,
        #[serde(rename = "Exposure")]
        #[schema(value_type = api::source::Exposure)]
        pub exposure: Option<Exposure>,
        #[serde(rename = "Sharpness")]
        #[schema(value_type = api::source::Sharpness)]
        pub sharpness: Option<Sharpness>,
        #[schema(value_type = api::source::GammaCorrection)]
        pub gamma_correction: Option<GammaCorrection>,
        #[schema(value_type = api::source::PowerLineFrequency)]
        pub power_line_frequency: Option<PowerLineFrequency>,
        #[serde(rename = "Color")]
        #[schema(value_type = api::source::Color)]
        pub color: Option<Color>,
        #[serde(rename = "IrcutFilter")]
        #[schema(value_type = api::source::IrcutFilter)]
        pub ircut_filter: Option<IrcutFilter>,
        #[serde(rename = "ImageModeList")]
        #[schema(value_type = api::source::ImageModeList)]
        pub image_mode_list: Option<ImageModeList>,
        #[serde(rename = "BrightEnhance")]
        #[schema(value_type = api::source::BrightEnhance)]
        pub bright_enhance: Option<BrightEnhance>,
        #[serde(rename = "ISPMode")]
        #[schema(value_type = api::source::ISPMode)]
        pub isp_mode: Option<ISPMode>,
        #[serde(rename = "Shutter")]
        #[schema(value_type = api::source::Shutter)]
        pub shutter: Option<Shutter>,
        #[serde(rename = "Gain")]
        #[schema(value_type = api::source::Gain)]
        pub gain: Option<Gain>,
        #[serde(rename = "ImageIcrE")]
        #[schema(value_type = api::source::ImageIcrE)]
        pub image_icr_e: Option<ImageIcrE>,
        #[serde(rename = "ImageMultishut")]
        #[schema(value_type = api::source::ImageMultishut)]
        pub image_multi_shut: Option<ImageMultishut>,
        #[serde(rename = "PlateBright")]
        #[schema(value_type = api::source::PlateBright)]
        pub plate_bright: Option<PlateBright>,
        #[serde(rename = "JPEGParam")]
        #[schema(value_type = api::source::JPEGParam)]
        pub jpeg_param: Option<JPEGParam>,
        #[serde(rename = "DarkEnhance")]
        pub dark_enhance: Option<String>, //unused
        #[serde(rename = "Hdr")]
        pub hdr: Option<String>, //unused
        #[serde(rename = "LSE")]
        pub lse: Option<String>, //unused
        #[serde(rename = "MCE")]
        pub mce: Option<String>, //unused
        #[serde(rename = "Svce")]
        pub svce: Option<String>, //unused
        #[serde(rename = "SectionCtrl")]
        pub section_ctrl: Option<String>, //unused
        #[serde(rename = "AutoContrast")]
        pub auto_contrast: Option<String>, //unused
        #[serde(rename = "GrayRange")]
        #[schema(value_type = api::source::GrayRange)]
        pub gray_range: Option<GrayRange>,
        #[serde(rename = "LSEDetail")]
        pub lse_detail: Option<String>, //unused
        #[serde(rename = "ITCImageSnap")]
        #[schema(value_type = api::source::ITCImageSnap)]
        pub itc_image_snap: Option<ITCImageSnap>,
        #[serde(rename = "ImageRecord")]
        #[schema(value_type = api::source::ImageRecord)]
        pub image_record: Option<ImageRecord>,
        #[serde(rename = "Scene")]
        #[schema(value_type = api::source::Scene)]
        pub scene: Option<Scene>,
        #[serde(rename = "EPTZ")]
        pub eptz: Option<String>, //unused
        #[serde(rename = "EIS")]
        #[schema(value_type = api::source::EIS)]
        pub eis: Option<EIS>,
        #[serde(rename = "HLC")]
        #[schema(value_type = api::source::HLC)]
        pub hlc: Option<HLC>,
        #[serde(rename = "ZoomLimit")]
        #[schema(value_type = api::source::ZoomLimit)]
        pub zoom_limit: Option<ZoomLimit>,
        #[serde(rename = "corridor")]
        pub corridor: Option<String>, //unused
        #[serde(rename = "Dehaze")]
        #[schema(value_type = api::source::Dehaze)]
        pub dehaze: Option<Dehaze>,
        #[serde(rename = "ImageMode")]
        #[schema(value_type = api::source::ImageModeType)]
        pub image_mode: Option<ImageModeType>,
        #[serde(rename = "enableImageLossDetection")]
        pub enable_image_loss_detection: Option<bool>,
        #[serde(rename = "CaptureMode")]
        #[schema(value_type = api::source::CaptureMode)]
        pub capture_mode: Option<CaptureMode>,
        #[serde(rename = "IrLight")]
        #[schema(value_type = api::source::IrLight)]
        pub ir_light: Option<IrLight>,
        #[serde(rename = "LensDistortionCorrection")]
        pub lens_distortion_correction: Option<String>, //unused
        #[serde(rename = "ExposureSync")]
        pub exposure_sync: Option<String>, //unused
        #[serde(rename = "BrightnessSuddenChangeSuppression")]
        pub brightness_sudden_change_suppression: Option<String>, //unused
        #[serde(rename = "TempRange")]
        #[schema(value_type = api::source::TempRange)]
        pub temp_range: Option<TempRange>,
        #[serde(rename = "NoiseReduceExt")]
        #[schema(value_type = api::source::NoiseReduceExt)]
        pub noise_reduce_ext: Option<NoiseReduceExt>,
        #[serde(rename = "PTZ")]
        #[schema(value_type = api::source::PTZ)]
        pub ptz: Option<PTZ>,
        #[serde(rename = "Iris")]
        #[schema(value_type = api::source::Iris)]
        pub iris: Option<Iris>,
        #[schema(value_type = api::source::Proportionalpan)]
        pub proportionalpan: Option<Proportionalpan>,
    }

    #[derive(Debug, Clone, Deserialize, PartialEq, Serialize, ToSchema)]
    #[schema(as = api::source::Scene)]
    pub struct Scene {
        pub mode: Option<String>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Serialize)]
    pub struct FocusData {
        pub focus: i32,
    }

    impl From<FocusValue> for FocusData {
        fn from(value: FocusValue) -> Self {
            Self {
                focus: value as i32,
            }
        }
    }

    #[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub enum StatusCode {
        #[serde(rename = "ok")]
        OK,
        DeviceBusy,
        DeviceError,
        InvalidOperation,
        InvalidXMLFormat,
        InvalidXMLContent,
        BadXmlContent,
        RebootRequired,
        AdditionalError,
        #[serde(rename = "Unknow")]
        Unknow,
    }

    pub type SubStatusCode = StatusCode;

    impl From<u8> for StatusCode {
        fn from(sc: u8) -> Self {
            use StatusCode::*;

            match sc {
                0 | 1 => OK,
                2 => DeviceBusy,
                3 => DeviceError,
                4 => InvalidOperation,
                5 => InvalidXMLFormat,
                6 => InvalidXMLContent,
                7 => RebootRequired,
                8 => AdditionalError,
                9 => AdditionalError,
                _ => Unknow,
            }
        }
    }

    impl Display for StatusCode {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.to_string())
        }
    }

    #[derive(Error, Debug)]
    pub enum ErrorCode {
        //StatusCode = 1
        #[error("Operation completed")]
        OK,
        #[error("Risky password")]
        RiskPassword,
        #[error("Arming process")]
        ArmProcess,

        //StatusCode = 2
        #[error("Insufficient memory")]
        NoMemory,
        #[error("The service is not availiable")]
        ServiceUnavailiable,
        #[error("Upgrading")]
        Upgrading,
        #[error("The device is busy or no response")]
        DeviceBusy,
        #[error("The video server is reconnected")]
        ReConnectIpc,
        #[error("Transmitting device upgrade data failed")]
        TransferUpgradePackageFailed,
        #[error("Starting upgrading device failed")]
        StartUpgradeFailed,
        #[error("Getting upgrade status failed")]
        GetUpgradeProcessfailed,
        #[error("The Authentication certificate already exists")]
        CertificateExist,

        //StatusCode = 3
        #[error("Hardware error")]
        DeviceError,
        #[error("Flash operation error")]
        BadFlash,
        #[error("The 28181 configuration is not initialized")]
        _28181Uninitialized,
        #[error("Connecting to socket failed")]
        SocketConnectError,
        #[error("Receive response message failed")]
        RecieveError,
        #[error("Deleting picture failed")]
        DeletePictureError,
        #[error("Too large picture size")]
        PictureSizeExceedLimit,
        #[error("Clearing cache failed")]
        ClearCacheError,
        #[error("Updating database failed")]
        UpdateDatabaseError,
        #[error("Searching in the database failed")]
        SearchDatabaseError,
        #[error("Writing to database failed")]
        WriteDatabaseError,
        #[error("Deleting database element failed")]
        DeleteDatabaseError,
        #[error("Getting number of database elements failed")]
        SearchDatabaseElementError,
        #[error("Downloading upgrade packet from cloud and upgrading failed")]
        CloudAutoUpgradeException,
        #[error("HBP exception")]
        HBPException,
        #[error("UDEP exception")]
        UDEPException,
        #[error("Elastic exception")]
        ElasticSearchException,
        #[error("Kafka exception")]
        KafkaException,
        #[error("HBase exception")]
        HBaseException,
        #[error("Spark exception")]
        SparkException,
        #[error("Yarn exception")]
        YarnException,
        #[error("Cache exception")]
        CacheException,
        #[error("Monitoring point big data server exception")]
        TrafficException,
        #[error("Human face big data server exception")]
        FaceException,
        #[error("SSD file system error (Error occurs when it is non-Ext4 file system)")]
        SSDFileSystemIsError,
        #[error("Insufficient SSD space for person frequency detection")]
        InsufficientSSDCapacityForFPD,
        #[error("Wi-Fi big data server exception")]
        WifiException,
        #[error("Video parameters structure server exception")]
        StructException,
        #[error("Data collection timed out")]
        CaptureTimeout,
        #[error("Low quality of collected data")]
        LowScore,

        //StatusCode = 4
        //TODO
        #[error("Unknown error")]
        Unknown,
    }

    impl From<u64> for ErrorCode {
        fn from(ec: u64) -> Self {
            use ErrorCode::*;

            match ec {
                0x1 => OK,
                0x10000002 => RiskPassword,
                0x10000005 => ArmProcess,

                0x20000001 => NoMemory,
                0x20000002 => ServiceUnavailiable,
                0x20000003 => Upgrading,
                0x20000004 => DeviceBusy,
                0x20000005 => ReConnectIpc,
                0x20000006 => TransferUpgradePackageFailed,
                0x20000007 => StartUpgradeFailed,
                0x20000008 => GetUpgradeProcessfailed,
                0x2000000B => CertificateExist,

                0x30000001 => DeviceError,
                0x30000002 => BadFlash,
                0x30000003 => _28181Uninitialized,
                0x30000005 => SocketConnectError,
                0x30000007 => RecieveError,
                0x3000000A => DeletePictureError,
                0x3000000C => PictureSizeExceedLimit,
                0x3000000D => ClearCacheError,
                0x3000000F => UpdateDatabaseError,
                0x30000010 => SearchDatabaseError,
                0x30000011 => WriteDatabaseError,
                0x30000012 => DeleteDatabaseError,
                0x30000013 => SearchDatabaseElementError,
                0x30000016 => CloudAutoUpgradeException,
                0x30001000 => HBPException,
                0x30001001 => UDEPException,
                0x30001002 => ElasticSearchException,
                0x30001003 => KafkaException,
                0x30001004 => HBaseException,
                0x30001005 => SparkException,
                0x30001006 => YarnException,
                0x30001007 => CacheException,
                0x30001008 => TrafficException,
                0x30001009 => FaceException,
                0x30001013 => SSDFileSystemIsError,
                0x30001014 => InsufficientSSDCapacityForFPD,
                0x3000100A => WifiException,
                0x3000100D => StructException,
                0x30006000 => CaptureTimeout,
                0x30006001 => LowScore,
                //TODO
                _ => Unknown,
            }
        }
    }

    #[derive(Debug, Clone, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Response {
        #[serde(rename = "requestURL")]
        pub request_url: String,
        pub status_code: u8,
        pub status_string: String,
        pub sub_status_string: Option<String>,
        pub id: Option<u32>,
        pub sub_status_code: SubStatusCode,
        pub error_code: Option<u64>,
        pub error_msg: Option<String>,
        pub additional_err: Option<AdditionalErr>,
    }

    impl Response {
        pub fn is_ok(&self) -> bool {
            self.status_code == 0 || self.status_code == 1
        }
    }

    #[derive(Debug, Clone, Deserialize, PartialEq)]
    pub struct AdditionalErr {
        #[serde(rename = "AdditionalError")]
        pub additional_error: AdditionalError,
    }

    #[derive(Debug, Clone, Deserialize, PartialEq)]
    pub struct AdditionalError {
        #[serde(rename = "StatusList")]
        pub status_list: StatusList,
    }

    #[derive(Debug, Clone, Deserialize, PartialEq)]
    pub struct StatusList {
        #[serde(rename = "Status")]
        pub status: Status,
    }

    #[derive(Debug, Clone, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Status {
        pub id: Option<u32>,
        pub status_code: u8,
        pub status_string: String,
        pub sub_status_code: SubStatusCode,
    }

    #[derive(Debug, Clone)]
    pub struct SimpleResponse {
        pub request_url: String,
        pub status_code: StatusCode,
        pub status_string: String,
        pub sub_status_code: SubStatusCode,
    }

    impl From<Response> for SimpleResponse {
        fn from(r: Response) -> Self {
            Self {
                request_url: r.request_url,
                status_code: r.status_code.into(),
                status_string: r.status_string,
                sub_status_code: r.sub_status_code,
            }
        }
    }

    impl Display for SimpleResponse {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{} -> ({:?}:{}:{:?})",
                self.request_url, self.status_code, self.status_string, self.sub_status_code
            )
        }
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub enum DefaultStatus {
        High,
        Low,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub enum OutputStatus {
        High,
        Low,
        Pulse,
    }

    #[derive(Debug, Deserialize)]
    pub struct SyncSignalOutputList {
        #[serde(rename = "SyncSignalOutput")]
        pub sync_signal_output_list: Vec<SyncSignalOutput>,
    }

    impl Serialize for SyncSignalOutputList {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut state = serializer.serialize_struct("SyncSignalOutputList", 1)?;
            for e in &self.sync_signal_output_list {
                state.serialize_field("SyncSignalOutput", e)?;
            }
            state.end()
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SyncSignalOutput {
        pub id: u8,
        pub output_status: OutputStatus,
        pub video_flash_enable: bool,
        pub detect_brightness_enable: bool,
    }

    impl From<bool> for SyncSignalOutputList {
        fn from(enabled: bool) -> Self {
            match enabled {
                false => Self::unset_all(),
                _ => Self::set_all(),
            }
        }
    }

    impl SyncSignalOutputList {
        pub fn unset_all() -> Self {
            let mut sync_signal_output_list = Vec::new();
            for id in 1..=7 {
                sync_signal_output_list.push(SyncSignalOutput::unset(id));
            }

            Self {
                sync_signal_output_list,
            }
        }

        pub fn set_all() -> Self {
            let mut sync_signal_output_list = Vec::new();
            for id in 1..=7 {
                sync_signal_output_list.push(SyncSignalOutput::set(id));
            }

            Self {
                sync_signal_output_list,
            }
        }

        pub fn set_some(lines: Vec<u8>) -> Self {
            let mut sync_signal_output_list = Vec::new();
            for id in lines {
                sync_signal_output_list.push(SyncSignalOutput::set(id));
            }

            Self {
                sync_signal_output_list,
            }
        }

        pub fn unset_some(lines: Vec<u8>) -> Self {
            let mut sync_signal_output_list = Vec::new();
            for id in lines {
                sync_signal_output_list.push(SyncSignalOutput::unset(id));
            }

            Self {
                sync_signal_output_list,
            }
        }
    }

    impl SyncSignalOutput {
        pub fn set(id: u8) -> Self {
            Self {
                id,
                output_status: OutputStatus::Pulse,
                video_flash_enable: false,
                detect_brightness_enable: false,
            }
        }

        pub fn unset(id: u8) -> Self {
            Self {
                id,
                output_status: OutputStatus::High,
                video_flash_enable: true,
                detect_brightness_enable: true,
            }
        }

        #[allow(dead_code)]
        pub fn is_set(&self) -> bool {
            self.output_status == OutputStatus::Pulse
                && self.video_flash_enable == false
                && self.detect_brightness_enable == false
        }

        #[allow(dead_code)]
        pub fn is_unset(&self) -> bool {
            !self.is_set()
        }
    }

    #[derive(Debug, Deserialize, PartialEq, Serialize, Clone, Copy)]
    pub enum FirmwareVerison {
        #[serde(rename = "V5.7.3")]
        V573,
        #[serde(rename = "V5.0.2")]
        V502,
        #[serde(rename = "V5.1.4")]
        V514,
        #[serde(rename = "V5.5.820")]
        V55820,
        #[serde(rename = "V5.5.800")]
        V55800,
    }

    impl Default for FirmwareVerison {
        fn default() -> Self {
            FirmwareVerison::V502
        }
    }

    #[derive(Debug, Default)]
    pub struct SPSettings {
        pub enabled: bool,
    }

    impl From<bool> for SPSettings {
        fn from(enabled: bool) -> Self {
            Self { enabled }
        }
    }

    impl From<SPSettings> for bool {
        fn from(ss: SPSettings) -> Self {
            ss.enabled
        }
    }

    impl From<IrcutFilter> for SPSettings {
        fn from(icr: IrcutFilter) -> Self {
            Self {
                enabled: icr.ircut_filter_type.into(),
            }
        }
    }

    impl From<SPSettings> for IrcutFilter {
        fn from(ss: SPSettings) -> Self {
            Self {
                ircut_filter_type: ss.enabled.into(),
                night_to_day_filter_level: Some(NIGHT_TO_DAY_FILTER_LEVEL_PTZ),
                night_to_day_filter_time: None,
            }
        }
    }

    impl From<ImageIcrE> for SPSettings {
        fn from(icr: ImageIcrE) -> Self {
            let enabled = icr.icr_ctrl.manual_mode.unwrap().manual_preset_val.into();

            Self { enabled }
        }
    }

    impl From<SPSettings> for ImageIcrE {
        fn from(ss: SPSettings) -> Self {
            Self {
                icr_ctrl: ICRCtrl {
                    icr_ctrl_mode: ICRCtrlMode::Manual,
                    manual_mode: Some(ManualMode {
                        manual_preset_val: ss.enabled.into(),
                    }),
                    time_mode: None,
                    auto_mode: None,
                },
            }
        }
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Clone, Default)]
    #[serde(rename_all = "camelCase")]
    pub struct DeviceInfo {
        pub device_name: String,
        #[serde(rename = "deviceID")]
        pub device_id: String,
        pub device_description: String,
        pub device_location: String,
        pub system_contact: String,
        pub model: String,
        pub serial_number: String,
        pub mac_address: String,
        #[serde(rename = "firmwareVersion")]
        pub firmware_verison: FirmwareVerison,
        pub firmware_released_date: String,
        pub i_beacon_version: String,
        pub encoder_version: String,
        pub encoder_released_date: String,
        pub boot_version: String,
        pub boot_released_date: String,
        pub hardware_version: String,
        pub device_type: String,
        #[serde(rename = "telecontrolID")]
        pub telecontrol_id: String,
        pub support_beep: String,
        pub support_video_loss: String,
        pub sub_channel_enabled: bool,
        pub thr_channel_enabled: bool,
        pub fourth_channel_enabled: bool,
        pub fifth_channel_enabled: bool,
        pub transparent_enabled: String,
        pub customized_info: String,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Clone, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::ICRCtrlMode)]
    pub enum ICRCtrlMode {
        Manual,
        Time,
        Auto,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Clone, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::ManualMode)]
    pub struct ManualMode {
        #[serde(rename = "ManualPresetVal")]
        #[schema(value_type = api::source::IrcutFilterTypes)]
        pub manual_preset_val: IrcutFilterTypes,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Clone, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::ICRCtrl)]
    pub struct ICRCtrl {
        #[serde(rename = "ICRCtrlMode")]
        #[schema(value_type = api::source::ICRCtrlMode)]
        pub icr_ctrl_mode: ICRCtrlMode,
        #[serde(rename = "ManualMode")]
        #[schema(value_type = api::source::ManualMode)]
        pub manual_mode: Option<ManualMode>,
        #[serde(rename = "TimeMode")]
        #[schema(value_type = api::source::TimeMode)]
        pub time_mode: Option<TimeMode>,
        #[serde(rename = "AutoMode")]
        #[schema(value_type = api::source::AutoMode)]
        pub auto_mode: Option<AutoMode>,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Clone, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::ImageIcrE)]
    pub struct ImageIcrE {
        #[serde(rename = "ICRCtrl")]
        #[schema(value_type = api::source::ICRCtrl)]
        pub icr_ctrl: ICRCtrl,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Clone, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::IrcutFilterTypes)]
    pub enum IrcutFilterTypes {
        Auto,
        Day,
        Night,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::AutoMode)]
    pub struct AutoMode {
        #[serde(rename = "ICRAutoSwitch")]
        pub icr_auto_switch: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::TimeMode)]
    pub struct TimeMode {
        #[schema(value_type = api::source::SwitchList)]
        pub switch_list: SwitchList,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::TimeSwitch)]
    pub struct TimeSwitch {
        pub time_id: i32,
        #[serde(rename = "PresetVal")]
        #[schema(value_type = api::source::IrcutFilterTypes)]
        pub preset_val: IrcutFilterTypes,
        pub start_hour: i32,
        pub start_minute: i32,
        pub end_hour: i32,
        pub end_minute: i32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::SwitchList)]
    pub struct SwitchList {
        #[schema(value_type = Vec<api::source::TimeSwitch>)]
        pub time_switch: Vec<TimeSwitch>,
    }

    impl From<bool> for IrcutFilterTypes {
        fn from(enabled: bool) -> Self {
            match enabled {
                true => IrcutFilterTypes::Night,
                _ => IrcutFilterTypes::Day,
            }
        }
    }

    impl From<IrcutFilterTypes> for bool {
        fn from(ift: IrcutFilterTypes) -> Self {
            use IrcutFilterTypes::*;

            match ift {
                Night => true,
                _ => false,
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::IrcutFilter)]
    pub struct IrcutFilter {
        #[serde(rename = "IrcutFilterType")]
        #[schema(value_type = api::source::IrcutFilterTypes)]
        pub ircut_filter_type: IrcutFilterTypes,
        pub night_to_day_filter_level: Option<u32>,
        pub night_to_day_filter_time: Option<u32>,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Clone, ToSchema)]
    #[schema(as = api::source::VideoEncoding)]
    pub enum VideoEncoding {
        #[serde(rename = "H.264")]
        H264,
        #[serde(rename = "H.265")]
        H265,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Clone, ToSchema)]
    #[serde(rename_all = "lowercase")]
    #[schema(as = api::source::VideoScanType)]
    pub enum VideoScanType {
        INTERLACED,
        PROGRESSIVE,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    pub enum VideoCodeType {
        MJPEG,
        MPEG4,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
    #[serde(rename_all = "lowercase")]
    pub enum VideoQualityControlType {
        CBR,
        VBR,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Clone, ToSchema)]
    #[serde(rename_all = "lowercase")]
    #[schema(as = api::source::SVCMode)]
    pub enum SVCMode {
        #[serde(rename = "close_svc")]
        CLOSE,
        MANUAL,
        AUTO,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Clone, ToSchema)]
    #[serde(rename_all = "lowercase")]
    #[schema(as = api::source::SVC)]
    pub struct SVC {
        pub enabled: Option<bool>,
        #[serde(rename = "SVCMode")]
        #[schema(value_type = api::source::SVCMode)]
        pub svc_mode: Option<SVCMode>,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Clone, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::SVACProfile)]
    pub enum SVACProfile {
        Baseline,
        Main,
        High,
        Extended,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Clone, ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = api::source::H264Profile)]
    pub enum H264Profile {
        Baseline,
        Main,
        High,
        Extended,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::Video)]
    pub struct Video {
        pub enabled: bool,
        #[serde(rename = "videoInputChannelID")]
        pub video_input_channel_id: u8,
        #[schema(value_type = api::source::VideoScanType)]
        pub video_scan_type: Option<VideoScanType>,
        #[schema(value_type = api::source::VideoEncoding)]
        pub video_codec_type: VideoEncoding,
        pub video_resolution_width: i32,
        pub video_resolution_height: i32,
        pub video_position_x: Option<i32>,
        pub video_position_y: Option<i32>,
        pub video_quality_control_type: Option<String>,
        pub constant_bit_rate: Option<i32>,
        pub key_frame_interval: Option<i32>,
        pub mirror_enabled: Option<bool>,
        pub rotation_degree: Option<i32>,
        pub snap_shot_image_type: Option<String>,
        pub fixed_quality: i32,
        pub vbr_upper_cap: Option<i32>,
        #[schema(value_type = u64)]
        pub max_frame_rate: FpsValue,
        #[serde(rename = "SVC")]
        #[schema(value_type = api::source::SVC)]
        pub svc: Option<SVC>,
        #[serde(rename = "H264Profile")]
        #[schema(value_type = api::source::H264Profile)]
        pub h264_profile: Option<H264Profile>,
        #[serde(rename = "SVACProfile")]
        #[schema(value_type = api::source::SVACProfile)]
        pub svac_profile: Option<SVACProfile>,
        #[serde(rename = "GovLength")]
        pub gov_length: Option<u32>,
        pub smoothing: Option<u32>,
        #[serde(rename = "SmartCodec")]
        #[schema(value_type = api::source::SmartCodec)]
        pub smart_codec: Option<SmartCodec>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone, PartialEq, ToSchema)]
    #[schema(as = api::source::SmartCodec)]
    pub struct SmartCodec {
        pub enabled: bool,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum RtpTransportType {
        #[serde(rename = "RTP/UDP")]
        UDP,
        #[serde(rename = "RTP/TCP")]
        TCP,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct Unicast {
        pub enabled: bool,
        #[serde(rename = "interfaceID")]
        pub interface_id: Option<String>,
        pub rtp_transport_type: Option<RtpTransportType>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct Multicast {
        pub enabled: bool,
        #[serde(rename = "destIPAddress")]
        pub dest_ip_address: Option<String>,
        pub video_dest_port_no: Option<i32>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum CertificateType {
        #[serde(rename = "digest")]
        DIGEST,
        #[serde(rename = "digest/basic")]
        BASIC,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct Security {
        pub enabled: bool,
        pub certificate_type: Option<CertificateType>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum StreaminTransport {
        RTSP,
        RTP,
        HTTP,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct ControlProtocol {
        pub streaming_transport: Vec<StreaminTransport>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct ControlProtocolList {
        pub control_protocol: Option<Vec<ControlProtocol>>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct Transport {
        pub rtsp_port_no: u32,
        pub max_packet_size: u32,
        #[serde(rename = "ControlProtocolList")]
        pub control_protocol_list: Option<ControlProtocolList>,
        #[serde(rename = "Unicast")]
        pub unicast: Option<Unicast>,
        #[serde(rename = "Multicast")]
        pub multicast: Option<Multicast>,
        #[serde(rename = "Security")]
        pub security: Option<Security>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = api::source::StreamingChannel)]
    pub struct StreamingChannel {
        pub id: u32,
        pub channel_name: String,
        pub enabled: bool,
        #[serde(rename = "Video")]
        #[schema(value_type = api::source::Video)]
        pub video: Video,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct Time {
        pub time_mode: dublicates::TimeMode,
        pub local_time: String,
        pub time_zone: String,
        pub satellite_interval: Option<i32>,
        pub platform_no: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum AddresingFormatType {
        IPADDRESS,
        HOSTNAME,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct NTPServer {
        pub id: i32,
        pub addresing_format_type: AddresingFormatType,
        pub host_name: Option<String>,
        pub ip_address: Option<String>,
        pub ip6_address: Option<String>,
        pub port_no: Option<i32>,
        pub synchronize_interval: Option<i32>,
    }
}
pub mod dahua {
    use serde::Serialize;

    #[derive(Clone, Copy)]
    #[repr(u8)]
    pub enum AlarmMode {
        Automatic = 0,
        ForceOn = 1,
        #[allow(dead_code)]
        ForceOff = 2,
    }

    impl Default for AlarmMode {
        fn default() -> Self {
            Self::Automatic
        }
    }

    impl Serialize for AlarmMode {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            (*self as u8).serialize(serializer)
        }
    }

    #[derive(Default)]
    pub struct AlarmName;

    impl Serialize for AlarmName {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            "Noname".serialize(serializer)
        }
    }

    #[derive(Default, Serialize)]
    pub struct Config {
        #[serde(rename = "FlashLight.Enable")]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub spotlight: Option<bool>,
        #[serde(rename = "FlashLight.Brightness")]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub brightness: Option<u8>,
        #[serde(rename = "Encode[0].MainFormat[0].Video.FPS")]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub fps: Option<f64>,

        #[serde(flatten)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub external_spotlight: Option<ExternalSpotlight>,
    }

    #[derive(Serialize)]
    pub struct ExternalSpotlight {
        #[serde(rename = "AlarmOut[0].Mode")]
        pub alarm_mode: AlarmMode,
        #[serde(rename = "AlarmOut[0].Name")]
        pub alarm_name: AlarmName,
    }

    impl From<bool> for ExternalSpotlight {
        fn from(value: bool) -> Self {
            match value {
                true => Self::enabled(),
                _ => Self::disabled(),
            }
        }
    }

    impl ExternalSpotlight {
        pub fn enabled() -> Self {
            Self {
                alarm_mode: AlarmMode::ForceOn,
                alarm_name: AlarmName,
            }
        }

        pub fn disabled() -> Self {
            Self {
                alarm_mode: AlarmMode::Automatic,
                alarm_name: AlarmName,
            }
        }
    }
}
pub mod stilsoft {}
