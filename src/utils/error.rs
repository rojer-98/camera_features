use domain::stream::StreamError;
use domain::CameraId;
use onvif::OnvifError;

use thiserror::*;

#[derive(Error, Debug)]
pub enum IpCamerasError {
    #[error(transparent)]
    StreamError {
        #[from]
        source: StreamError,
    },
    #[error(transparent)]
    Std {
        #[from]
        source: std::io::Error,
    },
    #[error(transparent)]
    Utf8 {
        #[from]
        source: std::string::FromUtf8Error,
    },
    #[error("Error with std mutex or rwlock")]
    Sync,
    #[error("Digest auth error happened: {source}")]
    Digest {
        #[from]
        source: digest::DigestError,
    },
    #[error("Reqwest error happened: {source}")]
    Reqwest {
        #[from]
        source: reqwest::Error,
    },
    #[error("Serde json error happened: {source}")]
    SerdeJson {
        #[from]
        source: serde_json::Error,
    },
    #[error("Regex error happened: {source}")]
    Regex {
        #[from]
        source: regex::Error,
    },
    #[error("Serde url error happened: {source}")]
    SerdeUrl {
        #[from]
        source: serde_url_params::Error,
    },
    #[error("Serde xml error happened: {source}")]
    SerdeXml {
        #[from]
        source: serde_xml_rs::Error,
    },
    #[error("ONVIF error: {source}")]
    Onvif {
        #[from]
        source: OnvifError,
    },
    #[error("pulsar router slot error: {source}")]
    Slot {
        #[from]
        source: pulsar_core::router::SlotError,
    },
    #[error("no ONVIF connection available for block: {0}")]
    NoOnvifConnection(CameraId),
    #[error("no ONVIF parameters supplied")]
    NoOnvifParams,
    #[error("no ONVIF video source available")]
    NoOnvifVideoSource,
    #[error("params to connection is not set")]
    NotSet,
    #[error("api is not supported")]
    NotAvialiableApi,
    #[error("error with setting|getting spotlight to camera")]
    Spotlight,
    #[error("error with setting|getting fps to camera")]
    Fps,
}

impl From<IpCamerasError> for StreamError {
    fn from(error: IpCamerasError) -> Self {
        match error {
            IpCamerasError::NotAvialiableApi => StreamError::not_implemented(0),
            _ => StreamError::device(error.to_string())
        }
    }
}

impl From<std::io::ErrorKind> for IpCamerasError {
    fn from(error: std::io::ErrorKind) -> Self {
        IpCamerasError::Std {
            source: error.into(),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for IpCamerasError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        Self::Sync
    }
}
