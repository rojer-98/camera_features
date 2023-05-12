use digest::DigestAuth;
use pulsar_core::prelude::*;
use reqwest::Client;

use crate::IpCamerasError;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum RequestType {
    Reqwest,
    Curl,
    All,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Header {
    JSON,
}

impl Header {
    pub fn to_curl(&self) -> &str {
        use Header::*;

        match self {
            JSON => "Content-Type: application/json",
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Method {
    GET,
    PUT,
    POST,
    DELETE,
    PATCH,
    HEAD,
    CONNECT,
    OPTIONS,
    TRACE,
}

impl ToString for Method {
    fn to_string(&self) -> String {
        use Method::*;

        match self {
            GET => "GET".to_owned(),
            PUT => "PUT".to_owned(),
            POST => "POST".to_owned(),
            DELETE => "DELETE".to_owned(),
            PATCH => "PATCH".to_owned(),
            HEAD => "HEAD".to_owned(),
            CONNECT => "CONNECT".to_owned(),
            OPTIONS => "OPTIONS".to_owned(),
            TRACE => "TRACE".to_owned(),
        }
    }
}

pub async fn request(
    rt: RequestType,
    url: String,
    params: Option<String>,
    auth: (Option<String>, Option<String>),
    method: Method,
    headers: Option<Vec<Header>>,
) -> Result<String, IpCamerasError> {
    use RequestType::*;
    match rt {
        Reqwest => r_reqwest(url, params, auth, method, headers).await,
        Curl => r_curl(url, params, auth, method, headers).await,
        All => r_curl(
            url.clone(),
            params.clone(),
            auth.clone(),
            method,
            headers.clone(),
        )
        .await
        .and(r_reqwest(url, params, auth, method, headers).await),
    }
}

pub async fn r_curl(
    url: String,
    params: Option<String>,
    auth: (Option<String>, Option<String>),
    method: Method,
    headers: Option<Vec<Header>>,
) -> Result<String, IpCamerasError> {
    let mut cmd = tokio::process::Command::new("curl");

    cmd.arg(url).arg("-X").arg(method.to_string());

    if params.is_some() {
        cmd.arg("-d").arg(params.unwrap());
    }

    if headers.is_some() {
        let headers = headers.unwrap();

        for h in headers {
            cmd.arg("-H").arg(h.to_curl());
        }
    }

    // Wait for 5 second
    cmd.arg("--max-time").arg(5u32.to_string());

    if let (Some(username), Some(password)) = auth {
        let auth = format!("{}:{}", username, password);
        cmd.arg("--digest").arg("--user").arg(auth);
    }

    let output = cmd
        .output()
        .await
        .map_err(|source| IpCamerasError::Std { source })?
        .stdout;

    Ok(String::from_utf8(output).map_err(|source| IpCamerasError::Utf8 { source })?)
}

pub async fn r_reqwest(
    url: String,
    params: Option<String>,
    auth: (Option<String>, Option<String>),
    method: Method,
    headers: Option<Vec<Header>>,
) -> Result<String, IpCamerasError> {
    use Method::*;

    let client = Client::new();
    let params = params.unwrap_or_default();

    let (username, password) = auth;

    let rb = match method {
        GET => client.get(url),
        PUT => client.put(url).body(params),
        POST => client.post(url).body(params),
        DELETE => client.delete(url).body(params),
        HEAD => client.head(url),
        _ => {
            warn!("Not supported HTTP request {:?}", method);
            return Err(IpCamerasError::NotSet);
        }
    };

    let mut rb = if let Some(h_s) = headers {
        let mut rb_h = rb;
        for h in h_s {
            rb_h = match h {
                Header::JSON => rb_h.header(reqwest::header::CONTENT_TYPE, "application/json"),
            };
        }

        rb_h
    } else {
        rb
    };

    rb = match (username.as_ref(), password.as_ref()) {
        (Some(username), Some(password)) => rb.digest_auth(&username, &password).await?,
        _ => rb,
    };

    Ok(rb.send().await?.text().await?)
}
