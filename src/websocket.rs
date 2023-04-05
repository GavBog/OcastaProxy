use actix_web::{get, web, HttpRequest, HttpResponse};
use actix_ws::Message as ActixMessage;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{http, Message as TungsteniteMessage, Result},
};

#[get("/ws/main/{url:.*}")]
pub async fn proxy(
    req: HttpRequest,
    stream: web::Payload,
    url: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let mut url = reqwest::Url::parse(url.as_str()).unwrap();
    let query = req
        .query_string()
        .split('&')
        .filter(|s| !s.starts_with("origin="))
        .collect::<Vec<&str>>()
        .join("&");

    url.set_query(Some(query.as_str()));

    let origin = req
        .query_string()
        .split('&')
        .find(|s| s.starts_with("origin="))
        .map(|s| s.split('=').nth(1).unwrap_or(""))
        .unwrap_or("");

    let mut headers = reqwest::header::HeaderMap::new();
    for (k, v) in req.headers() {
        if k == "origin" {
            headers.insert(k, origin.parse().unwrap());
            continue;
        }
        if k == "host" {
            headers.insert(k, url.host_str().unwrap_or("").parse().unwrap());
            continue;
        }

        headers.insert(k, v.clone());
    }

    let mut server = http::Request::builder().uri(url.as_str()).body(()).unwrap();
    *server.headers_mut() = headers;

    let (mut response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let (mut socket, server_response) = connect_async(server).await.unwrap();
    *response.headers_mut() = server_response.headers().clone().into();

    actix_rt::spawn(async move {
        let mut alive = true;
        loop {
            tokio::select! {
                Some(Ok(msg)) = msg_stream.next() => {
                    match msg {
                        ActixMessage::Text(text) => {
                            let _ = socket.send(TungsteniteMessage::Text(text.to_string())).await;
                        }
                        ActixMessage::Binary(bin) => {
                            let _ = socket.send(TungsteniteMessage::Binary(bin.to_vec())).await;
                        }
                        ActixMessage::Ping(msg) => {
                            let _ = socket.send(TungsteniteMessage::Ping(msg.to_vec())).await;
                        }
                        ActixMessage::Pong(msg) => {
                            if msg.to_vec() == b"OcastaHeartbeat" {
                                alive = true;
                                continue;
                            }
                            let _ = socket.send(TungsteniteMessage::Pong(msg.to_vec())).await;
                        }
                        ActixMessage::Close(_) => {
                            let _ = socket.send(TungsteniteMessage::Close(None)).await;
                            let _ = socket.close(None).await;
                            let _ = session.close(None).await;
                            break;
                        }
                        _ => {}
                    }
                }
                Some(msg) = socket.next() => {
                    match msg {
                        Ok(msg) => {
                            match msg {
                                TungsteniteMessage::Text(text) => {
                                    let _ = session.text(text).await;
                                }
                                TungsteniteMessage::Binary(bin) => {
                                    let _ = session.binary(bin).await;
                                }
                                TungsteniteMessage::Ping(msg) => {
                                    let _ = session.ping(&msg).await;
                                }
                                TungsteniteMessage::Pong(msg) => {
                                    let _ = session.pong(&msg).await;
                                }
                                TungsteniteMessage::Close(_) => {
                                    let _ = socket.send(TungsteniteMessage::Close(None)).await;
                                    let _ = socket.close(None).await;
                                    let _ = session.close(None).await;
                                    break;
                                }
                                _ => {}
                            }
                        }
                        Err(_) => {
                            let _ = socket.send(TungsteniteMessage::Close(None)).await;
                            let _ = socket.close(None).await;
                            let _ = session.close(None).await;
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(100)) => {
                    let _ = session.ping(b"OcastaHeartbeat").await;
                    if !alive {
                        let _ = socket.send(TungsteniteMessage::Close(None)).await;
                        let _ = socket.close(None).await;
                        let _ = session.close(None).await;
                        break;
                    }
                    alive = false;
                }
                else => {
                    let _ = socket.send(TungsteniteMessage::Close(None)).await;
                    let _ = socket.close(None).await;
                    let _ = session.close(None).await;
                    break;
                }
            }
        }
    });

    Ok(response)
}
