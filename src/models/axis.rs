use async_trait::*;

use std::io::ErrorKind;

use serde::{de::DeserializeOwned, Serialize};

use domain::{stream::Resource, CameraId};
use pulsar_core::prelude::*;

use crate::{
    utils::{focus::*, handler::*, request::*, serde::axis::*},
    IpCamerasError, DEFAULT_TIMEOUT,
};

use onvif::FpsValue;

#[derive(Debug)]
pub struct AxisHttp {
    pub id: CameraId,
    pub host: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub timeout: u64,
}

impl From<Resource> for AxisHttp {
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

impl Default for AxisHttp {
    fn default() -> Self {
        Self {
            id: 0,
            host: None,
            username: None,
            password: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

#[async_trait]
impl ApiHandler for AxisHttp {
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

    // EXTERNAL API
    async fn switch_spotlight(&self, enabled: bool) -> Result<(), IpCamerasError> {
        let port = Port {
            port: "2",
            normal_state: enabled.into(),
            state: enabled.into(),
        };

        self.axis_request::<SwitchData, _>(RequestParams::SetPorts { ports: [port] })
            .await?;

        Ok(())
    }

    async fn get_spotlight_state(&self) -> Result<bool, IpCamerasError> {
        let response: ProjectorsData = self
            .axis_request::<_, [Port; 0]>(RequestParams::GetPorts)
            .await?;

        let port = response
            .items
            .into_iter()
            .find(|item| item.port == "2")
            .ok_or(ErrorKind::NotFound)?;

        Ok(port.state == PortState::Closed && port.normal_state == PortState::Closed)
    }

    async fn get_fps(&self) -> Result<FpsValue, IpCamerasError> {
        let fps_val = Self::parse_int(
            &self
                .get(
                    "param",
                    &[("action", "list"), ("group", "Image.I0.Stream.FPS")],
                )
                .await?,
        );
        if fps_val.is_some() {
            Ok(fps_val.unwrap_or_default().into())
        } else {
            Err(ErrorKind::InvalidData.into())
        }
    }

    async fn set_fps(&self, fps: FpsValue) -> Result<(), IpCamerasError> {
        if self
            .get(
                "param",
                &[
                    ("action", "update"),
                    ("Image.I0.Stream.FPS", &fps.to_string()),
                ],
            )
            .await?
            .starts_with("OK")
        {
            Ok(())
        } else {
            Err(ErrorKind::InvalidData.into())
        }
    }

    async fn set_date_time(&self, date_time: chrono::NaiveDateTime) -> Result<(), IpCamerasError> {
        self.generic_request(
            "time",
            SetTimeZoneRequest {
                time_zone: "UTC".into(),
            },
        )
        .await?;

        self.generic_request("time", SetDateTimeRequest { date_time })
            .await?;

        Ok(())
    }

    async fn get_focus_capabilities(&self) -> Result<FocusCapabilities, IpCamerasError> {
        Ok(FocusCapabilities::new().absolute(0.0, 1.0, 0.001))
    }

    async fn get_focus_absolute(&self) -> Result<FocusValue, IpCamerasError> {
        use std::str::FromStr;
        use xml::{
            attribute::OwnedAttribute,
            reader::{EventReader, XmlEvent},
        };

        let req = self
            .get("opticssetup", &[("monitor", "poll"), ("source", "1")])
            .await?;
        let parser = EventReader::new(req.as_bytes());

        for elem in parser.into_iter().flatten() {
            if let XmlEvent::StartElement {
                name, attributes, ..
            } = elem
            {
                if name.local_name == "opticsSetupState" {
                    for OwnedAttribute { name, value } in &attributes {
                        if name.local_name == "focusPosition" {
                            if let Ok(focus) = f32::from_str(value) {
                                return Ok(focus);
                            }
                        }
                    }
                }
            }
        }
        Err(ErrorKind::InvalidData.into())
    }

    async fn set_focus_absolute(&self, focus: FocusValue) -> Result<(), IpCamerasError> {
        if self
            .get(
                "opticssetup",
                &[("afocus", &focus.to_string()), ("source", "1")],
            )
            .await?
            .starts_with("ok")
        {
            Ok(())
        } else {
            Err(ErrorKind::InvalidData.into())
        }
    }
}

impl AxisHttp {
    async fn axis_request<
        D: DeserializeOwned + std::fmt::Debug,
        P: AsRef<[Port]> + Serialize + Send + Sync + 'static,
    >(
        &self,
        params: RequestParams<P>,
    ) -> Result<D, IpCamerasError> {
        let result: Response<D> = serde_json::from_str(
            self.request(
                format!("http://{}/axis-cgi/io/portmanagement.cgi", self.host()),
                Some(serde_json::to_string(&ApiRequest::from(params))?),
                Method::POST,
                Some(vec![Header::JSON]),
            )
            .await?
            .as_ref(),
        )?;

        Ok(result.data)
    }

    async fn generic_request<
        S: std::fmt::Display,
        P: Serialize + Send + 'static,
        R: DeserializeOwned + std::fmt::Debug,
    >(
        &self,
        cgi: S,
        payload: impl Into<GenericApiRequest<P>> + Send + 'static,
    ) -> Result<R, IpCamerasError> {
        let result: Response<R> = serde_json::from_str(
            self.request(
                format!("http://{}/axis-cgi/{}.cgi", self.host(), cgi),
                Some(serde_json::to_string(&payload.into())?),
                Method::GET,
                Some(vec![Header::JSON]),
            )
            .await?
            .as_ref(),
        )?;

        Ok(result.data)
    }

    async fn get<S: AsRef<str>>(
        &self,
        cgi: S,
        params: &[(S, S)],
    ) -> Result<String, IpCamerasError> {
        Ok(self
            .request(
                format!(
                    "http://{}/axis-cgi/{}.cgi?{}",
                    self.host(),
                    cgi.as_ref(),
                    params
                        .iter()
                        .map(|(p, v)| format!("{}={}", p.as_ref(), v.as_ref()))
                        .reduce(|a, i| format!("{}&{}", a, i))
                        .unwrap_or_else(|| "".to_string())
                ),
                None,
                Method::GET,
                None,
            )
            .await?)
    }

    fn parse_int(input: &str) -> Option<u32> {
        input
            .chars()
            .skip_while(|ch| ch != &'=')
            .skip_while(|ch| !ch.is_digit(10))
            .take_while(|ch| ch.is_digit(10))
            .fold(None, |acc, ch| {
                ch.to_digit(10).map(|b| acc.unwrap_or(0) * 10 + b)
            })
    }
}
