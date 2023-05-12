use std::{
    io::ErrorKind,
    sync::atomic::{AtomicBool, Ordering::Relaxed},
};

use async_trait::*;
use regex::Regex;

use domain::{stream::Resource, CameraId};
use onvif::{ok_or_explain, FpsValue, OnvifConnection, OnvifError, OnvifParams};
use pulsar_core::prelude::*;

use crate::{
    utils::{handler::*, request::*},
    IpCamerasError, DEFAULT_TIMEOUT,
};

#[derive(Debug)]
pub struct StilsoftHttp {
    pub id: CameraId,
    pub host: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub timeout: u64,
    pub language: u32,

    pub spotlight_state: AtomicBool,
}

impl Default for StilsoftHttp {
    fn default() -> Self {
        Self {
            id: 0,
            host: None,
            username: None,
            password: None,
            timeout: DEFAULT_TIMEOUT,
            language: 3,

            spotlight_state: AtomicBool::new(false),
        }
    }
}

impl From<Resource> for StilsoftHttp {
    fn from(r: Resource) -> Self {
        let o = r.onvif.unwrap_or_default();
        Self {
            id: r.id,
            host: o.host,
            username: o.username,
            password: o.password,

            ..Default::default()
        }
    }
}

#[async_trait]
impl ApiHandler for StilsoftHttp {
    //AUTH GETTERS
    fn auth(&self) -> (&str, &str) {
        match (self.username.as_ref(), self.password.as_ref()) {
            (Some(u), Some(p)) => (u.as_str(), p.as_str()),
            (Some(u), None) => (u.as_str(), "Admin777"),
            (None, Some(p)) => ("admin", p.as_str()),
            (None, None) => ("admin", "Admin777"),
        }
    }

    fn host(&self) -> &str {
        if self.host.is_some() {
            self.host.as_ref().unwrap().as_str()
        } else {
            warn!("Empty host. Take localhost.");
            "127.0.0.1"
        }
    }

    //EXTERNAL API
    async fn get_fps(&self) -> Result<FpsValue, IpCamerasError> {
        Ok(ok_or_explain!(self.init_onvif().await?.get_fps().await))
    }

    async fn set_fps(&self, fps: FpsValue) -> Result<(), IpCamerasError> {
        Ok(ok_or_explain!(self.init_onvif().await?.set_fps(fps).await))
    }

    async fn get_spotlight_state(&self) -> Result<bool, IpCamerasError> {
        Ok(self.spotlight_state.load(Relaxed))
    }

    async fn switch_spotlight(&self, enabled: bool) -> Result<(), IpCamerasError> {
        let web_id = self.get_id_from_camera().await?;
        let host = self.host();
        let value = (enabled as i32) + 1;

        if self
            .request(
                format!("http://{host}/ajax/image_profile?id={web_id}&value={value}"),
                None,
                Method::GET,
                None,
            )
            .await?
            .contains("Success")
        {
            self.spotlight_state.store(enabled, Relaxed);
            Ok(())
        } else {
            Err(IpCamerasError::Spotlight)
        }
    }
}

impl StilsoftHttp {
    async fn get_id_from_camera(&self) -> Result<String, IpCamerasError> {
        let host = self.host();
        let (user, password) = self.auth();
        let language = self.language;

        let url = format!("http://{}/goform/setLoginParam", host);
        let params = format!("user={user}&password={password}&language={language}",);

        let response = self.request(url, Some(params), Method::POST, None).await?;
        let re = Regex::new(r"(YWRtaW46YWRtaW4|YWRtaW46YWRtaW43Nzc)")?;

        let caps = re.captures(&response).ok_or(IpCamerasError::Std {
            source: std::io::ErrorKind::InvalidData.into(),
        })?;

        if caps.len() > 0 {
            let catch = caps.get(0).map_or("", |m| m.as_str());
            Ok(catch.to_string())
        } else {
            Err(IpCamerasError::from(ErrorKind::InvalidInput))
        }
    }

    async fn init_onvif(&self) -> Result<OnvifConnection, OnvifError> {
        let onvif_params = OnvifParams {
            host: self.host.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            dummy: false,
            post_process_status: None,
        };

        let onvif_connection = ok_or_explain!(OnvifConnection::new(onvif_params).await);

        Ok(onvif_connection)
    }
}
