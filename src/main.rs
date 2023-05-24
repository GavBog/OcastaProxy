use axum::{
    body::Body,
    extract,
    http::{Request, Response, StatusCode},
    response::Html,
    routing::get,
    Router,
};
use base64::{
    alphabet,
    engine::{self, general_purpose},
    Engine as _,
};
use ocastaproxy::{rewrite, websocket};
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize)]
struct FormData {
    url: String,
}

#[derive(Deserialize)]
struct ProxyData {
    #[serde(flatten)]
    query: std::collections::HashMap<String, String>,
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn gateway(url: extract::Query<FormData>, path: extract::Path<String>) -> Response<Body> {
    let mut url = url.url.clone();
    if !url.starts_with("http") {
        url = format!("https://{}", url);
    }
    let encoding = path.as_str();
    url = match encoding {
        "b64" => {
            const CUSTOM_ENGINE: engine::GeneralPurpose =
                engine::GeneralPurpose::new(&alphabet::URL_SAFE, general_purpose::NO_PAD);
            CUSTOM_ENGINE.encode(url)
        }
        _ => url,
    };
    url = format!("/{}/{}", encoding, url);

    let mut res = Response::default();
    let header = match reqwest::header::HeaderValue::from_str(&url) {
        Ok(header) => header,
        Err(_) => {
            return bad_request_response();
        }
    };

    *res.status_mut() = StatusCode::PERMANENT_REDIRECT;
    res.headers_mut().insert("location", header);
    res
}

async fn proxy(
    extract::Path((encoding, url)): extract::Path<(String, String)>,
    query: extract::Query<ProxyData>,
    req: Request<Body>,
) -> Response<Body> {
    let mut url = match encoding.as_str() {
        "b64" => {
            const CUSTOM_ENGINE: engine::GeneralPurpose =
                engine::GeneralPurpose::new(&alphabet::URL_SAFE, general_purpose::NO_PAD);

            match CUSTOM_ENGINE.decode(url) {
                Ok(url) => match String::from_utf8(url) {
                    Ok(url) => match reqwest::Url::parse(&url) {
                        Ok(url) => url,
                        Err(_) => {
                            return bad_request_response();
                        }
                    },
                    Err(_) => {
                        return bad_request_response();
                    }
                },
                Err(_) => {
                    return bad_request_response();
                }
            }
        }
        _ => match reqwest::Url::parse(&url) {
            Ok(url) => url,
            Err(_) => {
                return bad_request_response();
            }
        },
    };

    let query = query
        .query
        .iter()
        .map(|(key, value)| {
            if value.is_empty() {
                key.clone()
            } else {
                format!("{}={}", key, value)
            }
        })
        .collect::<Vec<String>>()
        .join("&");

    if !query.is_empty() {
        url.set_query(Some(&query));
    }

    // Headers
    let mut headers = reqwest::header::HeaderMap::new();
    let origin = url.origin().ascii_serialization();
    for (key, value) in req.headers().iter() {
        match key.as_str() {
            "host"
            | "accept-encoding"
            | "forwarded"
            | "x-forwarded-for"
            | "x-forwarded-host"
            | "x-forwarded-proto"
            | "x-real-ip"
            | "x-envoy-external-address" => {}
            "origin" => {
                match reqwest::header::HeaderValue::from_str(&origin) {
                    Ok(header_value) => headers.insert(key.clone(), header_value),
                    Err(_) => {
                        return internal_server_error_response();
                    }
                };
            }
            "referer" => {
                match reqwest::header::HeaderValue::from_str(url.as_str()) {
                    Ok(header_value) => headers.insert(key.clone(), header_value),
                    Err(_) => {
                        return internal_server_error_response();
                    }
                };
            }
            _ => {
                headers.insert(key.clone(), value.clone());
            }
        }
    }

    // Download
    let client = reqwest::Client::new();
    let response = match client.get(url.clone()).headers(headers).send().await {
        Ok(res) => res,
        Err(_) => {
            return internal_server_error_response();
        }
    };

    let status = response.status();
    let mut response_headers = response.headers().clone();
    response_headers.remove("content-length");
    response_headers.remove("content-security-policy");
    response_headers.remove("content-security-policy-report-only");
    response_headers.remove("strict-transport-security");
    response_headers.remove("x-content-type-options");
    response_headers.remove("x-frame-options");
    let content_type = match response_headers.get("content-type") {
        Some(content_type) => content_type,
        None => {
            return internal_server_error_response();
        }
    };

    if content_type.to_str().unwrap_or("").starts_with("image/") {
        let mut res = Response::default();
        *res.status_mut() = status;
        *res.headers_mut() = response_headers;
        *res.body_mut() = response.bytes().await.unwrap_or_default().into();
        return res;
    }

    let page = match response.text().await {
        Ok(page) => page,
        Err(_) => {
            return internal_server_error_response();
        }
    };

    // Rewrite
    let new_page = rewrite::page(
        page,
        url,
        encoding,
        content_type.to_str().unwrap_or("").to_string(),
        origin,
    );

    let mut res = Response::default();
    *res.status_mut() = status;
    *res.headers_mut() = response_headers;
    *res.body_mut() = new_page.into();
    res
}

fn bad_request_response() -> Response<Body> {
    let mut res = Response::default();
    *res.status_mut() = StatusCode::BAD_REQUEST;
    res
}

fn internal_server_error_response() -> Response<Body> {
    let mut res = Response::default();
    *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    res
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/:encoding/gateway", get(gateway))
        .route("/:encoding/*url", get(proxy))
        .route("/ws/:encoding/*url", get(websocket::proxy));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
