use base64::{engine::general_purpose::STANDARD as b64, Engine};
use js_sys::{Array, Function, Object, Proxy, Reflect};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::Url;

// Client side code takes heavy inspiration from Corrosion only rewritten in Rust
// https://github.com/titaniumnetwork-dev/Corrosion/tree/main/lib/browser

#[wasm_bindgen]
pub fn rewrite() {
    let window = web_sys::window().expect("no global `window` exists");
    let href = window.location().href().unwrap_or_default();

    // get encoding and url
    let mut parts = href.split('/');
    let encoding = parts.nth(3).unwrap_or_default();
    let url =
        Url::new(parts.collect::<Vec<&str>>().join("/").as_str()).expect("failed to parse url");

    let object = Object::new();
    // set $Ocasta.location
    let _ = Reflect::set(&object, &"location".into(), &url.clone().into());
    // set $Ocasta.encoding
    let _ = Reflect::set(&object, &"encoding".into(), &encoding.into());
    // set $Ocasta.url
    let _ = Reflect::set(&object, &"url".into(), &Object::new());

    // set window.$Ocasta
    let _ = Reflect::set(&window, &"$Ocasta".into(), &object);

    // set url handlers
    let object_clone = object.clone();
    let url_handler_wrap = Closure::wrap(Box::new(
        move |target: JsValue, _that: JsValue, args: JsValue| {
            let args_array = Array::from(&args);

            if args_array.length() > 0 {
                let first_arg = args_array.get(0);
                let wrapped_url = url_wrap(
                    first_arg.as_string().unwrap_or_default(),
                    Reflect::get(
                        &Reflect::get(&object_clone, &"location".into()).unwrap_or_default(),
                        &"origin".into(),
                    )
                    .unwrap_or_default()
                    .as_string()
                    .unwrap_or_default(),
                    Reflect::get(&object_clone, &"encoding".into())
                        .unwrap_or_default()
                        .as_string()
                        .unwrap_or_default(),
                );

                args_array.set(0, JsValue::from(wrapped_url));
            }

            let target_function = target.dyn_into::<Function>().unwrap();
            let new_instance =
                Reflect::construct(&target_function, &args_array).unwrap_or_default();
            JsValue::from(new_instance)
        },
    )
        as Box<dyn FnMut(JsValue, JsValue, JsValue) -> JsValue>);

    // set window.Request
    let request = Reflect::get(&window, &"Request".into()).unwrap_or_default();
    if !request.is_undefined() {
        // ctx.window.Request = new Proxy(ctx.window.Request, {
        //     construct(target, args) {
        //         if (args[0]) args[0] = ctx.url.wrap(args[0], { ...ctx.meta, flags: ['xhr'], })
        //         return Reflect.construct(target, args);
        //     },
        // });
        let handler = Object::new();
        let _ = Reflect::set(
            &handler,
            &"construct".into(),
            &url_handler_wrap.as_ref().into(),
        );
        let proxy = Proxy::new(&request, &handler);
        let _ = Reflect::set(&window, &"Request".into(), &proxy.into());
    }
}

#[wasm_bindgen]
pub fn url_wrap(url: String, origin: String, encoding: String) -> String {
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

    if !origin.ends_with("/") && !url.starts_with("/") && !valid_protocol {
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
pub fn url_unwrap(url: String, encoding: String) -> String {
    let mut url = url;

    url = url
        .strip_prefix(format!("/{}/", encoding).as_str())
        .unwrap_or(&url)
        .to_string();
    url = decode(url, encoding.clone());

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
