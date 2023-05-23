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
    return Html(include_str!("../static/index.html"));
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

    return Response::builder()
        .status(StatusCode::FOUND)
        .header("location", format!("/{}/{}", encoding, url))
        .body(Body::empty())
        .unwrap();
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
                            return Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::empty())
                                .unwrap();
                        }
                    },
                    Err(_) => {
                        return Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::empty())
                            .unwrap();
                    }
                },
                Err(_) => {
                    return Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::empty())
                        .unwrap();
                }
            }
        }
        _ => match reqwest::Url::parse(&url) {
            Ok(url) => url,
            Err(_) => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::empty())
                    .unwrap();
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
                        return Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::empty())
                            .unwrap();
                    }
                };
            }
            "referer" => {
                match reqwest::header::HeaderValue::from_str(url.as_str()) {
                    Ok(header_value) => headers.insert(key.clone(), header_value),
                    Err(_) => {
                        return Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::empty())
                            .unwrap();
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
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap();
        }
    };
    let mut response_headers = response.headers().clone();
    response_headers.remove("content-length");
    let content_type = match response_headers.get("content-type") {
        Some(content_type) => content_type,
        None => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap();
        }
    };

    if content_type.to_str().unwrap_or("").starts_with("image/") {
        return Response::builder()
            .header("content-type", content_type)
            .body(response.bytes().await.unwrap_or_default().into())
            .unwrap();
    }

    let page = match response.text().await {
        Ok(page) => page,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap();
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

    Response::builder()
        .header("content-type", content_type)
        .body(Body::from(new_page))
        .unwrap()
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
