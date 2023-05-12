use async_trait::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_xml_rs::{from_str, to_string};

use std::{
    io::ErrorKind,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, Mutex,
    },
};

use common::CameraRole;
use domain::{stream::Resource, CameraId};
use onvif::FpsValue;

use pulsar_core::prelude::*;

use crate::{
    utils::{focus::*, handler::*, request::Method, serde::hik::*},
    AdditionalConfiguration, HikvisionConfiguration, IpCamerasError, DEFAULT_TIMEOUT,
};

#[derive(Debug, Clone)]
pub struct Focus {
    pub current_interval: usize,
    pub current_direction: bool,
}

impl Default for Focus {
    fn default() -> Self {
        Self {
            current_interval: 1,
            current_direction: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Projectors {
    pub projectors_lines: Vec<u8>,
}

impl Default for Projectors {
    fn default() -> Self {
        Self {
            projectors_lines: vec![5, 7],
        }
    }
}

#[derive(Debug, Clone)]
pub struct CameraS {
    pub firmware_verison: FirmwareVerison,
}

impl Default for CameraS {
    fn default() -> Self {
        Self {
            firmware_verison: FirmwareVerison::V502,
        }
    }
}

type ProjectorsSettings = Arc<Mutex<Projectors>>;
type FocusSettings = Arc<Mutex<Focus>>;
type CameraSettings = Arc<Mutex<CameraS>>;

#[derive(Debug)]
pub struct HikvisionHttp {
    pub id: CameraId,
    pub host: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub timeout: u64,
    pub camera_role: CameraRole,

    pub focus: FocusSettings,
    pub projectors: ProjectorsSettings,
    pub camera_version: CameraSettings,

    pub is_ptz: AtomicBool,
}

impl Default for HikvisionHttp {
    fn default() -> Self {
        Self {
            id: 0,
            host: None,
            username: None,
            password: None,
            timeout: DEFAULT_TIMEOUT,
            camera_role: CameraRole::View,

            focus: Arc::new(Mutex::new(Default::default())),
            projectors: Arc::new(Mutex::new(Default::default())),
            camera_version: Arc::new(Mutex::new(Default::default())),

            is_ptz: AtomicBool::new(false),
        }
    }
}

impl From<Resource> for HikvisionHttp {
    fn from(r: Resource) -> Self {
        let o = r.onvif.unwrap_or_default();

        Self {
            id: r.id,
            host: o.host,
            username: o.username,
            password: o.password,
            camera_role: r.role,

            ..Default::default()
        }
    }
}

#[async_trait]
impl ApiHandler for HikvisionHttp {
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
    async fn init(&self) -> Result<(), IpCamerasError> {
        let firmware_verison = self
            .retrieve_version_of_camera()
            .await
            .unwrap_or_default()
            .firmware_verison;

        self.camera_version.lock()?.firmware_verison = firmware_verison;
        trace!("Hikvision got firmware version");

        let is_ptz = self.check_is_ptz().await?;
        self.is_ptz.store(is_ptz, Relaxed);

        if !is_ptz {
            trace!("Not ptz");
            self.projectors.lock()?.projectors_lines = self.prepare_raw_projectors().await?;
            trace!("Hikvsion got projectors");
        }

        Ok(())
    }

    async fn get_spotlight_state(&self) -> Result<bool, IpCamerasError> {
        Ok(self.retrieve_spotlight_settings().await?.into())
    }

    async fn switch_spotlight(&self, enabled: bool) -> Result<(), IpCamerasError> {
        let some_lines = self.projectors.lock()?.projectors_lines.clone();

        let sync_signal_output_list = if enabled {
            SyncSignalOutputList::set_some(some_lines)
        } else {
            SyncSignalOutputList::unset_some(some_lines)
        };

        trace!("Current switch list is {:?}", sync_signal_output_list);

        Ok(self
            .send_spotlight_settings(enabled.into(), sync_signal_output_list)
            .await?)
    }

    async fn get_fps(&self) -> Result<FpsValue, IpCamerasError> {
        let video_settings = self.retrieve_video_settings().await?.video;
        let fps = video_settings.max_frame_rate / 100;
        Ok(fps)
    }

    async fn set_fps(&self, fps: FpsValue) -> Result<(), IpCamerasError> {
        let mut sc = self.retrieve_video_settings().await?;
        sc.video.max_frame_rate = fps * 100;

        self.send_video_settings(sc).await?;
        Ok(())
    }

    async fn set_focus_continuous(&self, fc: FocusContinuous) -> Result<(), IpCamerasError> {
        let (interval, direction) = (fc.interval, fc.direction);

        // 60 is default value from Hikvision web page
        let focus = match direction {
            Direction::Forward => 60.,
            _ => -60.,
        };
        trace!("Current set focus is {focus}");

        // Imitation of Hikvision web Page
        // At the beginning we send a focus value
        // After a zero
        self.send_focus_settings(focus.into()).await?;
        self.send_focus_settings(0f32.into()).await?;

        trace!("Focus update is done");

        let direction = match direction {
            Direction::Forward => true,
            _ => false,
        };

        self.focus.lock()?.current_interval = interval;
        self.focus.lock()?.current_direction = direction;

        Ok(())
    }

    async fn get_focus_capabilities(&self) -> Result<FocusCapabilities, IpCamerasError> {
        Ok(FocusCapabilities::new().continuous(1, 1))
    }

    async fn get_focus_continuous(&self) -> Result<FocusContinuous, IpCamerasError> {
        let interval = self.focus.lock()?.current_interval;
        let direction = match self.focus.lock()?.current_direction {
            true => Direction::Forward,
            _ => Direction::Backward,
        };

        Ok(FocusContinuous {
            direction,
            interval,
        })
    }

    async fn get_additional_configuration(
        &self,
    ) -> Result<AdditionalConfiguration, IpCamerasError> {
        let hikvision = Some(self.prepare_hikvision_configuration().await?);
        let id = self.id;

        Ok(AdditionalConfiguration {
            id,
            hikvision,
            default_settings: Some(false),

            ..Default::default()
        })
    }

    async fn get_default_configuration(&self) -> Result<AdditionalConfiguration, IpCamerasError> {
        let (ic, sc) = self.retrieve_common_default_settings().await?;

        Ok(AdditionalConfiguration {
            id: self.id,
            default_settings: Some(true),
            hikvision: Some(HikvisionConfiguration {
                image_channel: Some(ic),
                streaming_channel: Some(sc),

                ..Default::default()
            }),
            ..Default::default()
        })
    }

    async fn set_additional_configuration(
        &self,
        configuration: AdditionalConfiguration,
    ) -> Result<(), IpCamerasError> {
        match configuration.default_settings {
            Some(ds) => {
                if ds {
                    return self.send_common_default_settings().await;
                }
            }
            _ => (),
        }

        match configuration.hikvision {
            Some(configuration) => {
                if configuration.default_switch {
                    let mut new_projectors = Vec::new();

                    if configuration.external_projector {
                        new_projectors.push(5)
                    }

                    if configuration.internal_projector {
                        new_projectors.push(7)
                    }

                    self.projectors.lock()?.projectors_lines = new_projectors;
                }

                if let Some(ic) = configuration.image_channel {
                    self.send_image_channel(ic.clone()).await?;
                }

                if let Some(sc) = configuration.streaming_channel {
                    self.send_video_settings(sc).await?;
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }
}

impl HikvisionHttp {
    async fn send<S>(&self, url: String, settings: S) -> Result<(), IpCamerasError>
    where
        S: Serialize + Send + 'static + std::fmt::Debug,
    {
        let response: Response = from_str(
            &self
                .request(
                    url,
                    Some(format!(r#"{}"#, to_string(&settings)?)),
                    Method::PUT,
                    None,
                )
                .await?,
        )?;

        if response.is_ok() {
            Ok(())
        } else {
            let error_code = response.status_code;
            let err_msg = response.status_string;

            error!("Hikvision send reqwest error: {err_msg} with code {error_code}");

            Err(IpCamerasError::from(ErrorKind::InvalidData))
        }
    }

    async fn recieve<D>(&self, url: String) -> Result<D, IpCamerasError>
    where
        D: DeserializeOwned,
    {
        Ok(from_str(
            &self.request(url, None, Method::GET, None).await?,
        )?)
    }

    // FUNCTIONS TO PREPEARE RECIEVE|SEND
    async fn retrieve_spotlight_settings(&self) -> Result<SPSettings, IpCamerasError> {
        let host = self.host.clone().unwrap_or_default();

        let ss = match self.camera_role {
            CameraRole::View => return Ok(SPSettings::default()),
            _ => {
                if self.is_ptz.load(Relaxed) {
                    Ok(self
                        .recieve::<IrcutFilter>(format!(
                            "http://{host}/ISAPI/Image/channels/1/ircutFilter"
                        ))
                        .await?
                        .into())
                } else {
                    Ok(self
                        .recieve::<ImageIcrE>(format!("http://{host}/ISAPI/Image/channels/1/icr"))
                        .await?
                        .into())
                }
            }
        };

        trace!(
            "Projector hik spotlight settings: addr: {}, return: {:?}",
            self.host(),
            ss
        );

        ss
    }

    async fn get_raw_projectors_params(&self) -> Result<SyncSignalOutputList, IpCamerasError> {
        let host = self.host.clone().unwrap_or_default();

        let ssol = match self.camera_role {
            CameraRole::View => return Err(IpCamerasError::NotAvialiableApi),
            _ => {
                self.recieve::<SyncSignalOutputList>(format!(
                    "http://{host}/ISAPI/ITC/syncSignalOutput"
                ))
                .await?
            }
        };

        Ok(ssol)
    }

    async fn send_projectors_settings(
        &self,
        host: &str,
        ps: SyncSignalOutputList,
    ) -> Result<(), IpCamerasError> {
        let projector = self
            .send::<SyncSignalOutputList>(
                format!("http://{host}/ISAPI/ITC/syncSignalOutput"),
                ps.into(),
            )
            .await;

        trace!(
            "Switch day|night hik status: addr: {}, return: {:?}",
            self.host(),
            projector
        );

        projector
    }

    async fn send_icr_settings(&self, host: &str, ss: SPSettings) -> Result<(), IpCamerasError> {
        let day_and_night = self
            .send::<ImageIcrE>(
                format!("http://{host}/ISAPI/Image/channels/1/icr"),
                ss.into(),
            )
            .await;
        trace!(
            "Projector hik status: addr: {}, return: {:?}",
            self.host(),
            day_and_night
        );

        day_and_night
    }

    async fn send_ptz_icr_settings(
        &self,
        host: &str,
        ss: SPSettings,
    ) -> Result<(), IpCamerasError> {
        let day_and_night = self
            .send::<IrcutFilter>(
                format!("http://{host}/ISAPI/Image/channels/1/ircutFilter"),
                ss.into(),
            )
            .await;
        trace!(
            "Projector hik status: addr: {}, return: {:?}",
            self.host(),
            day_and_night
        );

        day_and_night
    }

    async fn send_spotlight_settings(
        &self,
        ss: SPSettings,
        ps: SyncSignalOutputList,
    ) -> Result<(), IpCamerasError> {
        let host = self.host();
        match self.camera_role {
            CameraRole::View => return Ok(()),
            _ => {
                if self.is_ptz.load(Relaxed) {
                    trace!("PTZ switch");
                    self.send_ptz_icr_settings(host, ss).await
                } else {
                    self.send_projectors_settings(host, ps)
                        .await
                        .and(self.send_icr_settings(host, ss).await)
                }
            }
        }
    }

    async fn retrieve_video_settings(&self) -> Result<StreamingChannel, IpCamerasError> {
        let host = self.host();
        self.recieve(format!("http://{host}/ISAPI/Streaming/channels/1"))
            .await
    }

    async fn send_video_settings(&self, sc: StreamingChannel) -> Result<(), IpCamerasError> {
        let host = self.host();
        self.send(format!("http://{host}/ISAPI/Streaming/channels/1"), sc)
            .await
    }

    async fn retrieve_version_of_camera(&self) -> Result<DeviceInfo, IpCamerasError> {
        let host = self.host();
        self.recieve(format!("http://{host}/ISAPI/System/deviceInfo"))
            .await
    }

    async fn retrieve_image_channel(&self) -> Result<ImageChannel, IpCamerasError> {
        let host = self.host();
        self.recieve(format!("http://{host}/ISAPI/Image/channels/1"))
            .await
    }

    // This spaghetti code needs coz hikvision can't process image_channel request.
    // Faggot
    async fn send_image_channel(&self, ic: ImageChannel) -> Result<(), IpCamerasError> {
        let common_req = Ok(())
            .and(self.send_image_channel_color(&ic).await)
            .and(self.send_image_channel_sharpness(&ic).await)
            .and(self.send_image_channel_gain(&ic).await)
            .and(self.send_image_channel_shutter(&ic).await);

        match self.camera_role {
            CameraRole::Register => common_req
                .and(self.send_image_channel_white_balance(&ic).await)
                .and(self.send_image_channel_noise_reduce_ext(&ic).await)
                .and(self.send_image_channel_gamma_correction(&ic).await)
                .and(self.send_image_channel_noise_reduce_2d(&ic).await)
                .and(self.send_image_channel_bright_enhance(&ic).await),
            CameraRole::View => common_req
                .and(self.send_image_channel_exposure(&ic).await)
                .and(self.send_image_channel_hlc(&ic).await)
                .and(self.send_image_channel_noise_reduce(&ic).await),
            _ => Ok(()),
        }
    }

    async fn send_image_channel_color(&self, ic: &ImageChannel) -> Result<(), IpCamerasError> {
        let host = self.host();
        let c = unwrap_some!(
            ic.color.clone(),
            return Err(IpCamerasError::NotAvialiableApi)
        );
        self.send(format!("http://{host}/ISAPI/Image/channels/1/color"), c)
            .await
    }

    async fn send_image_channel_sharpness(&self, ic: &ImageChannel) -> Result<(), IpCamerasError> {
        let host = self.host();
        let s = unwrap_some!(
            ic.sharpness.clone(),
            return Err(IpCamerasError::NotAvialiableApi)
        );

        self.send(format!("http://{host}/ISAPI/Image/channels/1/sharpness"), s)
            .await
    }

    async fn send_image_channel_white_balance(
        &self,
        ic: &ImageChannel,
    ) -> Result<(), IpCamerasError> {
        let host = self.host();
        let wb = unwrap_some!(
            ic.white_balance.clone(),
            return Err(IpCamerasError::NotAvialiableApi)
        );
        self.send(
            format!("http://{host}/ISAPI/Image/channels/1/whiteBalance"),
            wb,
        )
        .await
    }

    async fn send_image_channel_bright_enhance(
        &self,
        ic: &ImageChannel,
    ) -> Result<(), IpCamerasError> {
        let host = self.host();
        let be = unwrap_some!(
            ic.bright_enhance.clone(),
            return Err(IpCamerasError::NotAvialiableApi)
        );
        self.send(
            format!("http://{host}/ISAPI/Image/channels/1/brightEnhance"),
            be,
        )
        .await
    }

    async fn send_image_channel_shutter(&self, ic: &ImageChannel) -> Result<(), IpCamerasError> {
        let host = self.host();
        let s = unwrap_some!(
            ic.shutter.clone(),
            return Err(IpCamerasError::NotAvialiableApi)
        );
        self.send(format!("http://{host}/ISAPI/Image/channels/1/shutter"), s)
            .await
    }

    async fn send_image_channel_noise_reduce_2d(
        &self,
        ic: &ImageChannel,
    ) -> Result<(), IpCamerasError> {
        let host = self.host();
        let nrd = unwrap_some!(
            ic.noise_reduce_2d.clone(),
            return Err(IpCamerasError::NotAvialiableApi)
        );
        self.send(
            format!("http://{host}/ISAPI/Image/channels/1/NoiseReduce2D"),
            nrd,
        )
        .await
    }

    async fn send_image_channel_gain(&self, ic: &ImageChannel) -> Result<(), IpCamerasError> {
        let host = self.host();
        let g = unwrap_some!(
            ic.gain.clone(),
            return Err(IpCamerasError::NotAvialiableApi)
        );
        self.send(format!("http://{host}/ISAPI/Image/channels/1/gain"), g)
            .await
    }

    async fn send_image_channel_gamma_correction(
        &self,
        ic: &ImageChannel,
    ) -> Result<(), IpCamerasError> {
        let host = self.host();
        let gc = unwrap_some!(
            ic.gamma_correction.clone(),
            return Err(IpCamerasError::NotAvialiableApi)
        );
        self.send(
            format!("http://{host}/ISAPI/Image/channels/1/gammaCorrection"),
            gc,
        )
        .await
    }

    async fn send_image_channel_noise_reduce(
        &self,
        ic: &ImageChannel,
    ) -> Result<(), IpCamerasError> {
        let host = self.host();
        let nre = unwrap_some!(
            ic.noise_reduce.clone(),
            return Err(IpCamerasError::NotAvialiableApi)
        );
        self.send(
            format!("http://{host}/ISAPI/Image/channels/1/noiseReduce"),
            nre,
        )
        .await
    }

    async fn send_image_channel_hlc(&self, ic: &ImageChannel) -> Result<(), IpCamerasError> {
        let host = self.host();
        let nre = unwrap_some!(ic.hlc.clone(), return Err(IpCamerasError::NotAvialiableApi));
        self.send(format!("http://{host}/ISAPI/Image/channels/1/HLC"), nre)
            .await
    }

    async fn send_image_channel_exposure(&self, ic: &ImageChannel) -> Result<(), IpCamerasError> {
        let host = self.host();
        let nre = unwrap_some!(
            ic.exposure.clone(),
            return Err(IpCamerasError::NotAvialiableApi)
        );
        self.send(
            format!("http://{host}/ISAPI/Image/channels/1/exposure"),
            nre,
        )
        .await
    }

    async fn send_image_channel_noise_reduce_ext(
        &self,
        ic: &ImageChannel,
    ) -> Result<(), IpCamerasError> {
        let host = self.host();
        let nre = unwrap_some!(
            ic.noise_reduce_ext.clone(),
            return Err(IpCamerasError::NotAvialiableApi)
        );
        self.send(
            format!("http://{host}/ISAPI/Image/channels/1/noiseReduceExt"),
            nre,
        )
        .await
    }

    #[allow(dead_code)]
    async fn retrieve_time_settings(&self) -> Result<Time, IpCamerasError> {
        let host = self.host();
        self.recieve(format!("http://{host}/ISAPI/System/time"))
            .await
    }

    #[allow(dead_code)]
    async fn send_time_settings(&self, ts: (Time, NTPServer)) -> Result<(), IpCamerasError> {
        let (t, ntp) = ts;
        let host = self.host();

        self.send(format!("http://{host}/ISAPI/System/time"), t)
            .await
            .and(
                self.send(format!("http://{host}/ISAPI/System/time/ntpServers/1"), ntp)
                    .await,
            )
    }

    async fn retrieve_ptz_channel(&self) -> Result<PTZChannel, IpCamerasError> {
        let host = self.host();

        self.recieve(format!("http://{host}/ISAPI/PTZCtrl/channels/1"))
            .await
    }

    async fn send_focus_settings(&self, fd: FocusData) -> Result<(), IpCamerasError> {
        let host = self.host();
        match self.camera_role {
            CameraRole::View => {
                self.send(
                    format!("http://{host}/ISAPI/System/Video/inputs/channels/1/focus"),
                    fd,
                )
                .await
            }
            _ => Err(IpCamerasError::NotAvialiableApi),
        }
    }

    #[allow(dead_code)]
    async fn default_time_settings(&self) -> Result<(Time, NTPServer), IpCamerasError> {
        let mut time = self.retrieve_time_settings().await?;
        time.time_mode = dublicates::TimeMode::NTP;

        let ntp_server = NTPServer {
            id: 1,
            addresing_format_type: AddresingFormatType::IPADDRESS,
            ip_address: Some("172.16.16.10".to_owned()),
            synchronize_interval: Some(60),
            port_no: Some(123),
            host_name: None,
            ip6_address: None,
        };

        Ok((time, ntp_server))
    }

    async fn default_general_settings(&self) -> Result<ImageChannel, IpCamerasError> {
        let mut ic = self.retrieve_image_channel().await?;

        //Setting default params
        match self.camera_role {
            CameraRole::Register => {
                ic.color.as_mut().map(|color| {
                    color.saturation_level = 50;
                    color.night_mode = Some(true);
                    color.brightness_level = 75;
                    color.contrast_level = 18;
                });
                ic.sharpness.as_mut().map(|sharpness| {
                    sharpness.sharpness_level = 60;
                });
                ic.white_balance.as_mut().map(|w_b| {
                    w_b.white_balance_style = WhiteBalanceStyle::AUTO1;
                });
                ic.bright_enhance.as_mut().map(|b_e| {
                    b_e.bright_enhance_level = 70;
                });
                ic.plate_bright.as_mut().map(|p_b| {
                    p_b.plate_bright_enabled = Some(true);
                    p_b.plate_bright_sensitivity = Some(5);
                });
                ic.gamma_correction.as_mut().map(|g_c| {
                    g_c.gamma_correction_enabled = true;
                    g_c.gamma_correction_level = 30;
                });
                ic.shutter.as_mut().map(|shutter| {
                    shutter.shutter_level = 2000.to_string();
                });
                ic.gain.as_mut().map(|gain| {
                    gain.gain_level = 30;
                });
                ic.noise_reduce_2d.as_mut().map(|nr2d| {
                    nr2d.noise_reduce_2d_enable = true;
                    nr2d.noise_reduce_2d_level = 55;
                });
                ic.noise_reduce_ext.as_mut().map(|nre| {
                    nre.mode = NoiseReduceMode::GENERAL;
                    nre.general_mode.general_level = 100;
                });
            }
            CameraRole::View => {
                ic.color.as_mut().map(|color| {
                    color.brightness_level = 62;
                    color.saturation_level = 50;
                    color.contrast_level = 44;
                    color.gray_scale = Some(GrayScale {
                        gray_scale_mode: GrayScaleMode::OUTDOOR,
                    });
                });
                ic.sharpness.as_mut().map(|sharpness| {
                    sharpness.sharpness_level = 50;
                });
                ic.exposure.as_mut().map(|exposure| {
                    exposure.exposure_type = ExposureType::AUTO;
                    exposure.auto_iris_level = Some(100);
                    exposure.overexpose_suppress = Some(OverexposeSuppress {
                        enabled: false,
                        ..Default::default()
                    });
                });
                ic.shutter.as_mut().map(|shutter| {
                    shutter.shutter_level = "1/500".to_string();
                });
                ic.gain.as_mut().map(|gain| {
                    gain.gain_level = 65;
                });
                ic.hlc.as_mut().map(|hlc| {
                    hlc.enabled = true;
                    hlc.hlc_level = 50;
                });
                ic.noise_reduce.as_mut().map(|n_r| {
                    n_r.mode = NoiseReduceMode::GENERAL;
                    n_r.general_mode = Some(GeneralMode { general_level: 51 });
                });
            }
            _ => (),
        }

        Ok(ic)
    }

    async fn default_video_settings(&self) -> Result<StreamingChannel, IpCamerasError> {
        let mut sc = self.retrieve_video_settings().await?;

        //Setting default params
        match self.camera_role {
            CameraRole::View => {
                sc.video.max_frame_rate = 1000;
                sc.video.video_resolution_width = 2592;
                sc.video.video_resolution_height = 1944;
                sc.video.video_quality_control_type = Some("cbr".to_string());
                sc.video.constant_bit_rate = Some(8192);
                sc.video.gov_length = Some(10);
                sc.video.h264_profile = Some(H264Profile::Baseline);
                sc.video.svc = Some(SVC {
                    enabled: Some(false),
                    svc_mode: None,
                });
                sc.video.smoothing = Some(1);
                sc.video.smart_codec = Some(SmartCodec { enabled: false });
            }
            CameraRole::Register => {
                sc.video.max_frame_rate = 1000;
                sc.video.video_resolution_width = 4096;
                sc.video.video_resolution_height = 2160;
                sc.video.svc = Some(SVC {
                    enabled: None,
                    svc_mode: Some(SVCMode::CLOSE),
                });
                sc.video.gov_length = Some(10);
            }
            _ => (),
        }
        Ok(sc)
    }

    async fn retrieve_common_settings(
        &self,
    ) -> Result<(ImageChannel, StreamingChannel), IpCamerasError> {
        Ok((
            self.retrieve_image_channel().await?,
            self.retrieve_video_settings().await?,
        ))
    }

    async fn retrieve_common_default_settings(
        &self,
    ) -> Result<(ImageChannel, StreamingChannel), IpCamerasError> {
        Ok((
            self.default_general_settings().await?,
            self.default_video_settings().await?,
        ))
    }

    async fn send_common_default_settings(&self) -> Result<(), IpCamerasError> {
        Ok(self
            .send_video_settings(self.default_video_settings().await?)
            .await
            .and(
                self.send_image_channel(self.default_general_settings().await?)
                    .await,
            )?)
    }

    async fn prepare_raw_projectors(&self) -> Result<Vec<u8>, IpCamerasError> {
        let ssol = self.get_raw_projectors_params().await?;
        let mut projectors = Vec::new();

        //Get 5 line
        if let Some(external) = ssol.sync_signal_output_list.get(4) {
            trace!("External state is {:?}", external);
            projectors.push(5)
        };

        //Get 7 line
        if let Some(internal) = ssol.sync_signal_output_list.get(6) {
            trace!("Internal state is {:?}", internal);
            projectors.push(7);
        };

        let current_version = self.camera_version.lock()?.firmware_verison;

        trace!("Current version of hikvision: {:?}", current_version);
        let default_switch = match current_version {
            FirmwareVerison::V514 => false,
            _ => true,
        };

        //Get 7 line
        if !default_switch {
            projectors.push(1)
        }

        Ok(projectors)
    }

    async fn prepare_hikvision_configuration(
        &self,
    ) -> Result<HikvisionConfiguration, IpCamerasError> {
        let projectors = self.projectors.lock()?.projectors_lines.clone();

        //Get 7 line
        let internal_projector = projectors.contains(&7);
        //Get 5 line
        let external_projector = projectors.contains(&5);
        let default_switch = !projectors.contains(&1);

        let (image_channel, streaming_channel) =
            if let Ok((ic, sc)) = self.retrieve_common_settings().await {
                (Some(ic), Some(sc))
            } else {
                error!("Cannot get image channel and streaming channel on hikvision");
                (None, None)
            };

        Ok(HikvisionConfiguration {
            internal_projector,
            external_projector,
            default_switch,

            image_channel,
            streaming_channel,

            ..Default::default()
        })
    }

    async fn check_is_ptz(&self) -> Result<bool, IpCamerasError> {
        Ok(self.retrieve_ptz_channel().await.is_ok())
    }
}
