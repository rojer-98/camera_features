mod models;
mod utils;

use models::{axis::*, dahua::*, hikvision::*, stilsoft::*};
use utils::handler::*;

use domain::stream::Resource;
use domain::CameraModelName;
use onvif::FpsValue;

pub use crate::utils::{error::IpCamerasError, focus::*, serde::external::*};

#[derive(Debug)]
pub enum CameraModelHttp {
    Dahua(DahuaHttp),
    Axis(AxisHttp),
    Stilsoft(StilsoftHttp),
    Hikvision(HikvisionHttp),
    Unknown,
}

impl Default for CameraModelHttp {
    fn default() -> Self {
        Self::Unknown
    }
}

impl From<CameraModelName> for CameraModelHttp {
    fn from(cmn: CameraModelName) -> Self {
        match cmn {
            CameraModelName::Axis => CameraModelHttp::Axis(AxisHttp {
                ..Default::default()
            }),
            CameraModelName::Dahua => CameraModelHttp::Dahua(DahuaHttp {
                ..Default::default()
            }),
            CameraModelName::Stilsoft => CameraModelHttp::Stilsoft(StilsoftHttp {
                ..Default::default()
            }),
            CameraModelName::Hikvision => CameraModelHttp::Hikvision(HikvisionHttp {
                ..Default::default()
            }),
            _ => CameraModelHttp::Unknown,
        }
    }
}

impl From<Resource> for CameraModelHttp {
    fn from(r: Resource) -> Self {
        let mut resource = r.clone();
        let mut o = r.onvif.unwrap();

        let host = o.host.as_mut().map(|host| host.replace("http://", ""));
        resource.onvif = Some(onvif::OnvifParams { host, ..o });

        let r = resource;
        match r.model_name {
            CameraModelName::Axis => CameraModelHttp::Axis(AxisHttp::from(r)),
            CameraModelName::Dahua => CameraModelHttp::Dahua(DahuaHttp::from(r)),
            CameraModelName::Stilsoft => CameraModelHttp::Stilsoft(StilsoftHttp::from(r)),
            CameraModelName::Hikvision => CameraModelHttp::Hikvision(HikvisionHttp::from(r)),
            _ => CameraModelHttp::Unknown,
        }
    }
}

macro_rules! implement_inner {
    ( $fun:ident $(| $args:ident: $type:ty |)* => $ret:ty ) => {
        pub async fn $fun(&self $(, $args:$type )*) -> Result<$ret, IpCamerasError> {
            use CameraModelHttp::*;

            match self {
                Axis(c) => c.$fun($( $args ),*).await,
                Stilsoft(c) => c.$fun($( $args ),*).await,
                Dahua(c) => c.$fun($( $args ),*).await,
                Hikvision(c) => c.$fun($( $args ),*).await,
                _ => Err(IpCamerasError::NotAvialiableApi),
            }
        }
    };
}

impl CameraModelHttp {
    pub fn name(&self) -> String {
        use CameraModelHttp::*;

        match self {
            Axis(_) => "Axis".to_string(),
            Dahua(_) => "Dahua".to_string(),
            Stilsoft(_) => "Stilsoft".to_string(),
            Hikvision(_) => "Hikvision".to_string(),
            _ => "Unknown".to_string(),
        }
    }
    // function_name | arg: type | => return_type
    implement_inner!(init  => ());

    implement_inner!(set_fps |fps: FpsValue| => ());
    implement_inner!(get_fps => FpsValue);

    implement_inner!(switch_spotlight |enabled: bool| => ());
    implement_inner!(get_spotlight_state => bool);

    implement_inner!(get_focus_capabilities => FocusCapabilities);
    implement_inner!(get_focus_continuous => FocusContinuous);
    implement_inner!(set_focus_continuous |focus: FocusContinuous| => ());
    implement_inner!(get_focus_absolute => FocusValue);
    implement_inner!(set_focus_absolute |focus: FocusValue| => ());
    implement_inner!(get_focus_relative => FocusValue);
    implement_inner!(set_focus_relative |focus: FocusValue| => ());

    implement_inner!(set_date_time |date_time: chrono::NaiveDateTime| => ());

    implement_inner!(get_additional_configuration => AdditionalConfiguration);
    implement_inner!(set_additional_configuration |configuration: AdditionalConfiguration| => ());
    implement_inner!(get_default_configuration => AdditionalConfiguration);
}
