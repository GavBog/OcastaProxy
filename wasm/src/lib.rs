use base64::{engine::general_purpose::STANDARD as b64, Engine};
use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn rewrite() {
    let window = web_sys::window().expect("no global `window` exists");
    let href = window.location().href().expect("failed to get pathname");

    // get encoding and url
    let mut parts = href.split('/');
    let encoding = parts.nth(3).expect("failed to get encoding");
    let url = web_sys::Url::new(parts.collect::<Vec<&str>>().join("/").as_str())
        .expect("failed to parse url");

    let object: Object = Object::new();
    // set $Ocasta.location
    let _ = Reflect::set(&object, &"location".into(), &url.clone().into());
    // set $Ocasta.encoding
    let _ = Reflect::set(&object, &"encoding".into(), &encoding.into());
    // set $Ocasta.url
    let _ = Reflect::set(&object, &"url".into(), &Object::new());

    // set window.$Ocasta
    let _ = Reflect::set(&window, &"$Ocasta".into(), &object);

    // set navigator.sendBeacon
    let navigator = Reflect::get(&window, &"navigator".into()).expect("failed to get navigator");
    let navigator =
        Reflect::get(&navigator, &"sendBeacon".into()).expect("failed to get sendBeacon");
    let send_beacon_proxy = js_sys::Proxy::new(&navigator, &js_sys::Object::new());
    let _ = Reflect::set(&send_beacon_proxy, &"apply".into(), &js_sys::Function::new_no_args("a[0] = $Ocasta.url.encode(a[0], $Ocasta.location.origin, window.$Ocasta.encoding); return Reflect.apply(t, g, a);").into());
    let _ = Reflect::set(&window, &"navigator".into(), &send_beacon_proxy.into());

    // set window.WebSocket
    let web_socket = Reflect::get(&window, &"WebSocket".into()).expect("failed to get WebSocket");
    let web_socket_proxy = js_sys::Proxy::new(&web_socket, &js_sys::Object::new());
    let _ = Reflect::set(&web_socket_proxy, &"apply".into(), &js_sys::Function::new_no_args("a[0] = /ws/ + $Ocasta.url.encode(a[0], $Ocasta.location.origin, window.$Ocasta.encoding); return Reflect.apply(t, g, a);").into());
    let _ = Reflect::set(&window, &"WebSocket".into(), &web_socket_proxy.into());
}

#[wasm_bindgen]
pub fn url_rewrite(url: String, origin: String, encoding: String) -> String {
    let mut url = url;

    if url.starts_with("data:")
        || url.starts_with("about:")
        || url.starts_with("javascript:")
        || url.starts_with("blob:")
        || url.starts_with("mailto:")
    {
        return url;
    }

    if url.starts_with("./") {
        url = url[2..].to_string();
    }
    if url.starts_with("../") {
        url = url[3..].to_string();
    }

    if url.starts_with(format!("/{}/", encoding).as_str()) {
        return url;
    }

    if url.starts_with("//") {
        url = format!("https:{}", url);
    }

    let valid_protocol = url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("ws://")
        || url.starts_with("wss://");

    if !origin.ends_with("/")
        && !url.starts_with("/")
        && !url.starts_with("http:")
        && !url.starts_with("https:")
    {
        url = format!("/{}", url);
    }

    url = if valid_protocol {
        url
    } else {
        format!("{}{}", origin, url)
    };

    url = encode(url, encoding.clone());

    url = format!("/{}/{}", encoding, url);

    url
}

#[wasm_bindgen]
pub fn encode(text: String, encoding: String) -> String {
    let text = match encoding.as_str() {
        "b64" => b64.encode(text.as_bytes()),
        _ => text,
    };

    text
}

#[wasm_bindgen]
pub fn decode(text: String, encoding: String) -> String {
    let text = match encoding.as_str() {
        "b64" => {
            String::from_utf8(b64.decode(text.as_bytes()).unwrap_or_default()).unwrap_or_default()
        }
        _ => text,
    };

    text
}
