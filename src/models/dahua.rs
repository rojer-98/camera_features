use std::io::ErrorKind;

use async_trait::*;
use domain::{stream::Resource, CameraId};
use onvif::FpsValue;
use pulsar_core::prelude::*;

use crate::{
    utils::{focus::*, handler::*, request::*, serde::dahua::*},
    IpCamerasError, DEFAULT_TIMEOUT,
};

const RETRIES: usize = 5;
const INTERVAL: u64 = 400;

#[derive(Debug)]
pub struct DahuaHttp {
    pub id: CameraId,
    pub host: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub timeout: u64,
}

impl From<Resource> for DahuaHttp {
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

impl Default for DahuaHttp {
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
impl ApiHandler for DahuaHttp {
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
    async fn set_fps(&self, fps: FpsValue) -> Result<(), IpCamerasError> {
        let fps = Some(fps as f64);
        self.set_config(Config {
            fps,
            ..Default::default()
        })
        .await
    }

    async fn get_fps(&self) -> Result<FpsValue, IpCamerasError> {
        let result = Self::parse_output(&self.get_config("Encode").await?);
        if result.is_some() {
            Ok(result.unwrap_or_default().into())
        } else {
            Err(IpCamerasError::Fps)
        }
    }

    async fn switch_spotlight(&self, enabled: bool) -> Result<(), IpCamerasError> {
        self.set_config(Config {
            external_spotlight: Some(enabled.into()),
            // spotlight: Some(brightness > 0),
            // brightness: Some(brightness)
            ..Default::default()
        })
        .await
    }

    async fn get_spotlight_state(&self) -> Result<bool, IpCamerasError> {
        let output = self.get_config("AlarmOut").await?;
        if output.contains("table.AlarmOut[0].Mode=0") {
            Ok(false)
        } else if output.contains("table.AlarmOut[0].Mode=1") {
            Ok(true)
        } else {
            Err(ErrorKind::InvalidInput.into())
        }
    }

    async fn set_date_time(&self, date_time: chrono::NaiveDateTime) -> Result<(), IpCamerasError> {
        self.get(
            "global",
            &[
                ("action", "setCurrentTime"),
                ("time", &date_time.format("%F%%20%T").to_string()),
            ],
        )
        .await?;

        Ok(())
    }

    async fn get_focus_capabilities(&self) -> Result<FocusCapabilities, IpCamerasError> {
        Ok(FocusCapabilities::new().absolute(0.0, 1.0, 0.001))
    }

    async fn get_focus_absolute(&self) -> Result<FocusValue, IpCamerasError> {
        use std::str::FromStr;
        use tokio::time::{sleep, Duration};

        for _ in 0..RETRIES {
            let output = self
                .get("devVideoInput", &[("action", "getFocusStatus")])
                .await?;
            let mut focus = None;
            let mut status = None;

            for line in output.lines() {
                let pv: Vec<&str> = line.split('=').collect();
                if pv.len() == 2 {
                    if pv[0] == "status.Focus" {
                        if let Ok(value) = FocusValue::from_str(pv[1]) {
                            focus = Some(value);
                        } else {
                            return Err(ErrorKind::InvalidData.into());
                        }
                    } else if pv[0] == "status.Status" && pv[1] == "Normal" {
                        status = Some(());
                    }
                }
            }

            if let (Some(focus), Some(())) = (focus, status) {
                return Ok(focus);
            }

            sleep(Duration::from_millis(INTERVAL)).await;
        }

        warn!("unable to get Normal focus status after {} tries", RETRIES);
        Err(ErrorKind::InvalidData.into())
    }

    async fn set_focus_absolute(&self, focus: FocusValue) -> Result<(), IpCamerasError> {
        self.get(
            "devVideoInput",
            &[
                ("action", "adjustFocus"),
                ("focus", &focus.to_string()),
                ("zoom", "0.0"),
            ],
        )
        .await?;

        Ok(())
    }
}

impl DahuaHttp {
    async fn get<S: AsRef<str>>(
        &self,
        cgi: S,
        params: &[(S, S)],
    ) -> Result<String, IpCamerasError> {
        Ok(self
            .request(
                format!(
                    "http://{}/cgi-bin/{}.cgi?{}",
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

    async fn get_config<S: AsRef<str>>(&self, key: S) -> Result<String, IpCamerasError> {
        // http://<ip>/cgi-bin/configManager.cgi?action=setConfig&<paramName>=<paramValue>[&<paramName>=<paramValue>...]
        Ok(self
            .request(
                format!(
                    "http://{}/cgi-bin/configManager.cgi?action=getConfig&name={}",
                    self.host(),
                    key.as_ref()
                ),
                None,
                Method::GET,
                None,
            )
            .await?)
    }

    async fn set_config(&self, config: Config) -> Result<(), IpCamerasError> {
        // http://<ip>/cgi-bin/configManager.cgi?action=setConfig&<paramName>=<paramValue>[&<paramName>=<paramValue>...]
        if self
            .request(
                format!(
                    "http://{}/cgi-bin/configManager.cgi?action=setConfig&{}",
                    self.host(),
                    serde_url_params::to_string(&config)?
                ),
                None,
                Method::GET,
                None,
            )
            .await?
            .contains("OK")
        {
            Ok(())
        } else {
            Err(ErrorKind::InvalidInput.into())
        }
    }

    fn parse_output(input: &str) -> Option<u32> {
        let inner = input.find("Encode[0].MainFormat[0].Video.FPS");
        if let Some(i) = inner {
            let output = match input.get(i..) {
                Some(s) => s,
                None => return None,
            };
            output
                .chars()
                .skip_while(|ch| ch != &'=')
                .skip_while(|ch| !ch.is_digit(10))
                .take_while(|ch| ch.is_digit(10))
                .fold(None, |acc, ch| {
                    ch.to_digit(10).map(|b| acc.unwrap_or(0) * 10 + b)
                })
        } else {
            None
        }
    }
}
