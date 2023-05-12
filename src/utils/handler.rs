use crate::{
    utils::{focus::*, request::*},
    AdditionalConfiguration, IpCamerasError,
};

use onvif::FpsValue;

use async_trait::*;

#[async_trait]
pub trait ApiHandler {
    //AUTH GETTERS
    fn auth(&self) -> (&str, &str);
    fn host(&self) -> &str {
        "127.0.0.1"
    }

    //INIT
    async fn init(&self) -> Result<(), IpCamerasError> {
        Ok(())
    }

    //HTTP REQUEST TO CAMERA
    async fn request(
        &self,
        url: String,
        params: Option<String>,
        method: Method,
        headers: Option<Vec<Header>>,
    ) -> Result<String, IpCamerasError> {
        let (user, password) = self.auth();

        request(
            RequestType::Reqwest,
            url,
            params,
            (Some(user.to_string()), Some(password.to_string())),
            method,
            headers,
        )
        .await
    }

    //FOCUS FUNCTIONS
    async fn get_focus_continuous(&self) -> Result<FocusContinuous, IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }
    async fn set_focus_continuous(&self, _: FocusContinuous) -> Result<(), IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }
    async fn get_focus_capabilities(&self) -> Result<FocusCapabilities, IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }
    async fn set_focus_relative(&self, _: FocusValue) -> Result<(), IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }
    async fn get_focus_relative(&self) -> Result<FocusValue, IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }
    async fn get_focus_absolute(&self) -> Result<FocusValue, IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }
    async fn set_focus_absolute(&self, _: FocusValue) -> Result<(), IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }

    //DATE AND TIME FUNCTIONS
    async fn set_date_time(&self, _: chrono::NaiveDateTime) -> Result<(), IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }

    //FPS FUNCTIONS
    async fn get_fps(&self) -> Result<FpsValue, IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }
    async fn set_fps(&self, _: FpsValue) -> Result<(), IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }

    //SWITCH AND GET SPOTIGHT FUNCTIONS
    async fn get_spotlight_state(&self) -> Result<bool, IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }
    async fn switch_spotlight(&self, _: bool) -> Result<(), IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }

    //SET AND GET ADDITIONAL CONFIGURATION
    async fn get_additional_configuration(
        &self,
    ) -> Result<AdditionalConfiguration, IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }
    async fn set_additional_configuration(
        &self,
        _: AdditionalConfiguration,
    ) -> Result<(), IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }
    async fn get_default_configuration(&self) -> Result<AdditionalConfiguration, IpCamerasError> {
        Err(IpCamerasError::NotAvialiableApi)
    }
}
