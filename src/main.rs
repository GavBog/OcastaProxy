use axum::{
    body::{Body, Bytes},
    extract,
    http::{HeaderMap, HeaderValue, Response, StatusCode},
    response::Html,
    routing::{get, post},
    Router,
};
use base64::{
    alphabet,
    engine::{self, general_purpose},
    Engine as _,
};
use ocastaproxy::{errors, rewrite, websocket};
use serde::Deserialize;
use std::{collections::HashMap, net::SocketAddr};

#[derive(Deserialize)]
struct FormData {
    url: String,
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn gateway(extract::Path(path): extract::Path<String>, body: Bytes) -> Response<Body> {
    let mut url = if let Ok(data) = serde_urlencoded::from_bytes::<FormData>(&body) {
        data.url
    } else {
        return errors::error_response(StatusCode::BAD_REQUEST);
    };
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

    let header = if let Ok(header) = HeaderValue::from_str(&url) {
        header
    } else {
        return errors::error_response(StatusCode::BAD_REQUEST);
    };

    let mut headers = HeaderMap::new();
    headers.insert("location", header);

    let mut res = Response::default();
    *res.status_mut() = StatusCode::SEE_OTHER;
    *res.headers_mut() = headers;

    res
}

fn get_url(url: String, encoding: String, query: HashMap<String, String>) -> String {
    let mut url = match encoding.as_str() {
        "b64" => {
            const CUSTOM_ENGINE: engine::GeneralPurpose =
                engine::GeneralPurpose::new(&alphabet::URL_SAFE, general_purpose::NO_PAD);

            if let Ok(url) = CUSTOM_ENGINE.decode(url) {
                if let Ok(url) = String::from_utf8(url) {
                    if let Ok(url) = reqwest::Url::parse(&url) {
                        url
                    } else {
                        return "".to_string();
                    }
                } else {
                    return "".to_string();
                }
            } else {
                return "".to_string();
            }
        }
        _ => {
            if let Ok(url) = reqwest::Url::parse(&url) {
                url
            } else {
                return "".to_string();
            }
        }
    };

    let query = query
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

    url.to_string()
}

fn get_headers(headers: HeaderMap, origin: String) -> HeaderMap {
    let mut new_headers = HeaderMap::new();
    for (key, value) in headers.iter() {
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
                if let Ok(header_value) = HeaderValue::from_str(&origin) {
                    new_headers.insert(key.clone(), header_value);
                }
            }
            "referer" => {
                if let Ok(header_value) = HeaderValue::from_str(&origin) {
                    new_headers.insert(key.clone(), header_value);
                }
            }
            _ => {
                new_headers.insert(key.clone(), value.clone());
            }
        }
    }

    new_headers
}

async fn proxy(
    extract::Path((encoding, url)): extract::Path<(String, String)>,
    extract::Query(query): extract::Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response<Body> {
    let url = if let Ok(url) = reqwest::Url::parse(&get_url(url, encoding.clone(), query)) {
        url
    } else {
        return errors::error_response(StatusCode::BAD_REQUEST);
    };

    let origin = url.origin().ascii_serialization();
    let headers = get_headers(headers, origin.clone());
    let client = reqwest::Client::new();
    let response = if let Ok(response) = client.get(url.clone()).headers(headers).send().await {
        response
    } else {
        return errors::error_response(StatusCode::BAD_REQUEST);
    };

    let status = response.status();
    let mut response_headers = response.headers().clone();
    response_headers.remove("content-length");
    response_headers.remove("content-security-policy");
    response_headers.remove("content-security-policy-report-only");
    response_headers.remove("strict-transport-security");
    response_headers.remove("x-content-type-options");
    response_headers.remove("x-frame-options");
    let content_type = if let Some(content_type) = response_headers.get("content-type") {
        content_type
    } else {
        return errors::error_response(StatusCode::BAD_REQUEST);
    };

    if content_type.to_str().unwrap_or("").starts_with("image/") {
        let mut res = Response::default();
        *res.status_mut() = status;
        *res.headers_mut() = response_headers;
        *res.body_mut() = response.bytes().await.unwrap_or_default().into();
        return res;
    }

    let page = if let Ok(page) = response.text().await {
        page
    } else {
        return errors::error_response(StatusCode::BAD_REQUEST);
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

async fn post_proxy(
    extract::Path((encoding, url)): extract::Path<(String, String)>,
    extract::Query(query): extract::Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let url = if let Ok(url) = reqwest::Url::parse(&get_url(url, encoding.clone(), query)) {
        url
    } else {
        return errors::error_response(StatusCode::BAD_REQUEST);
    };

    let origin = url.origin().ascii_serialization();
    let headers = get_headers(headers, origin.clone());
    let client = reqwest::Client::new();
    let response = if let Ok(response) = client
        .post(url.clone())
        .headers(headers)
        .body(body)
        .send()
        .await
    {
        response
    } else {
        return errors::error_response(StatusCode::BAD_REQUEST);
    };

    let status = response.status();
    let mut response_headers = response.headers().clone();
    response_headers.remove("content-length");
    response_headers.remove("content-security-policy");
    response_headers.remove("content-security-policy-report-only");
    response_headers.remove("strict-transport-security");
    response_headers.remove("x-content-type-options");
    response_headers.remove("x-frame-options");

    let page = if let Ok(page) = response.text().await {
        page
    } else {
        return errors::error_response(StatusCode::BAD_REQUEST);
    };

    let mut res = Response::default();
    *res.status_mut() = status;
    *res.headers_mut() = response_headers;
    *res.body_mut() = page.into();

    res
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/:encoding/gateway", post(gateway))
        .route("/:encoding/*url", get(proxy))
        .route("/:encoding/*url", post(post_proxy))
        .route("/ws/:encoding/*url", get(websocket::proxy));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
