use js_sys::{Array, Function, Object, Proxy, Reflect};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{Request, Url};

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

    // set window.Request
    let request = Reflect::get(&window, &"Request".into()).unwrap_or_default();
    if !request.is_undefined() {
        // const requestURL = Object.getOwnPropertyDescriptor(ctx.window.Request.prototype, 'url');
        let request_prototype = Reflect::get(&request, &"prototype".into()).unwrap_or_default();
        let request_url =
            Object::get_own_property_descriptor(&request_prototype.clone().into(), &"url".into());

        // ctx.window.Request = new Proxy(ctx.window.Request, {
        //     construct(target, args) {
        //         if (args[0]) args[0] = ctx.url.wrap(args[0], { ...ctx.meta, flags: ['xhr'], })
        //         return Reflect.construct(target, args);
        //     },
        // });
        let object_clone = object.clone();
        let closure = Closure::wrap(Box::new(
            move |target: JsValue, _that: JsValue, args: JsValue| {
                let args = Array::from(&args);
                let url = args.get(0).as_string().unwrap_or_default();
                let wrapped_url = url_wrap(
                    url,
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
                args.set(0, JsValue::from(wrapped_url));
                let target_function = target.dyn_into::<Function>().unwrap_or_default();
                let new_instance = Reflect::construct(&target_function, &args).unwrap_or_default();
                JsValue::from(new_instance)
            },
        )
            as Box<dyn FnMut(JsValue, JsValue, JsValue) -> JsValue>);
        let handler = Object::new();
        let _ = Reflect::set(&handler, &"construct".into(), &closure.as_ref().into());
        let proxy = Proxy::new(&request, &handler);
        let _ = Reflect::set(&window, &"Request".into(), &proxy.into());
        closure.forget();

        // Object.defineProperty(ctx.window.Request.prototype, 'url', {
        //     get: new Proxy(requestURL.get, {
        //         apply: (target, that, args) => {
        //             var url = Reflect.apply(target, that, args);
        //             return url ? ctx.url.unwrap(url, ctx.meta) : url;
        //         },
        //     }),
        // });
        let object_clone = object.clone();
        let closure = Closure::wrap(Box::new(
            move |target: JsValue, that: JsValue, args: JsValue| {
                let target = target.dyn_into::<Function>().unwrap_or_default();
                let args = Array::from(&args);
                let url = Reflect::apply(&target, &that, &args).unwrap_or_default();
                let unwrapped_url = url_unwrap(
                    url.as_string().unwrap_or_default(),
                    Reflect::get(&object_clone, &"encoding".into())
                        .unwrap_or_default()
                        .as_string()
                        .unwrap_or_default(),
                );

                JsValue::from(unwrapped_url)
            },
        )
            as Box<dyn FnMut(JsValue, JsValue, JsValue) -> JsValue>);
        let getter_handler = Object::new();
        let _ = Reflect::set(&getter_handler, &"apply".into(), &closure.as_ref().into());
        let request_url_get = Reflect::get(&request_url, &"get".into()).unwrap_or_default();
        let getter_proxy = Proxy::new(&request_url_get, &getter_handler);
        let url_descriptor = Object::new();
        let _ = Reflect::set(&url_descriptor, &"get".into(), &getter_proxy.into());
        let _ = Reflect::define_property(
            &request_prototype.into(),
            &"url".into(),
            &url_descriptor.into(),
        );
        closure.forget();
    }

    // set window.Response
    let response = Reflect::get(&window, &"Response".into()).unwrap_or_default();
    if !response.is_undefined() {
        // const responseURL = Object.getOwnPropertyDescriptor(ctx.window.Response.prototype, 'url');
        let response_prototype = Reflect::get(&response, &"prototype".into()).unwrap_or_default();
        let response_url =
            Object::get_own_property_descriptor(&response_prototype.clone().into(), &"url".into());

        // Object.defineProperty(ctx.window.Response.prototype, 'url', {
        //     get: new Proxy(responseURL.get, {
        //         apply: (target, that, args) => {
        //             var url = Reflect.apply(target, that, args);
        //             return url ? ctx.url.unwrap(url, ctx.meta) : url;
        //         },
        //     }),
        // });
        let object_clone = object.clone();
        let closure = Closure::wrap(Box::new(
            move |target: JsValue, that: JsValue, args: JsValue| {
                let target = target.dyn_into::<Function>().unwrap_or_default();
                let args = Array::from(&args);
                let url = Reflect::apply(&target, &that, &args).unwrap_or_default();
                let unwrapped_url = url_unwrap(
                    url.as_string().unwrap_or_default(),
                    Reflect::get(&object_clone, &"encoding".into())
                        .unwrap_or_default()
                        .as_string()
                        .unwrap_or_default(),
                );

                JsValue::from(unwrapped_url)
            },
        )
            as Box<dyn FnMut(JsValue, JsValue, JsValue) -> JsValue>);
        let getter_handler = Object::new();
        let _ = Reflect::set(&getter_handler, &"apply".into(), &closure.as_ref().into());
        let response_url_get = Reflect::get(&response_url, &"get".into()).unwrap_or_default();
        let getter_proxy = Proxy::new(&response_url_get, &getter_handler);
        let url_descriptor = Object::new();
        let _ = Reflect::set(&url_descriptor, &"get".into(), &getter_proxy.into());
        let _ = Reflect::define_property(
            &response_prototype.into(),
            &"url".into(),
            &url_descriptor.into(),
        );
        closure.forget();
    }

    // set window.open
    let open = Reflect::get(&window, &"open".into()).unwrap_or_default();
    if !open.is_undefined() {
        // ctx.window.open = new Proxy(ctx.window.open, {
        //     apply: (target, that, args) => {
        //         if (args[0]) args[0] = ctx.url.wrap(args[0], ctx.meta);
        //         return Reflect.apply(target, that, args)
        //     },
        // });
        let object_clone = object.clone();
        let closure = Closure::wrap(Box::new(
            move |target: JsValue, that: JsValue, args: JsValue| {
                let target = target.dyn_into::<Function>().unwrap_or_default();
                let args = Array::from(&args);
                let url = Reflect::apply(&target, &that, &args).unwrap_or_default();
                let wrapped_url = url_wrap(
                    url.as_string().unwrap_or_default(),
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

                JsValue::from(wrapped_url)
            },
        )
            as Box<dyn FnMut(JsValue, JsValue, JsValue) -> JsValue>);
        let handler = Object::new();
        let _ = Reflect::set(&handler, &"apply".into(), &closure.as_ref().into());
        let open_proxy = Proxy::new(&open, &handler);
        let _ = Reflect::set(&window, &"open".into(), &open_proxy.into());
        closure.forget();
    }

    // set window.fetch
    let fetch = Reflect::get(&window, &"fetch".into()).unwrap_or_default();
    if !fetch.is_undefined() {
        // ctx.window.fetch = new Proxy(ctx.window.fetch, {
        //     apply: (target, that, args) => {
        //         if (args[0] instanceof ctx.window.Request) return Reflect.apply(target, that, args);
        //         if (args[0]) args[0] = ctx.url.wrap(args[0], { ...ctx.meta, flags: ['xhr'], });
        //         return Reflect.apply(target, that, args);
        //     },
        // });
        let object_clone = object.clone();
        let closure = Closure::wrap(Box::new(
            move |target: JsValue, that: JsValue, args: JsValue| {
                let args = Array::from(&args);
                let target = target.dyn_into::<Function>().unwrap_or_default();
                let result;
                if args.get(0).is_instance_of::<Request>() {
                    result = Reflect::apply(&target, &that, &args).unwrap_or_default();
                } else {
                    let url = Reflect::apply(&target, &that, &args).unwrap_or_default();
                    let wrapped_url = url_wrap(
                        url.as_string().unwrap_or_default(),
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

                    result = JsValue::from(wrapped_url);
                }

                result
            },
        )
            as Box<dyn FnMut(JsValue, JsValue, JsValue) -> JsValue>);
        let handler = Object::new();
        let _ = Reflect::set(&handler, &"apply".into(), &closure.as_ref().into());
        let fetch_proxy = Proxy::new(&fetch, &handler);
        let _ = Reflect::set(&window, &"fetch".into(), &fetch_proxy.into());
        closure.forget();
    }

    // set window.Navigator
    let navigator = Reflect::get(&window, &"Navigator".into()).unwrap_or_default();
    let navigator_prototype = Reflect::get_prototype_of(&navigator).unwrap_or_default();
    let send_beacon = Reflect::get(&navigator_prototype, &"sendBeacon".into()).unwrap_or_default();
    if !navigator.is_undefined() && !send_beacon.is_undefined() {
        // ctx.window.Navigator.prototype.sendBeacon = new Proxy(ctx.window.Navigator.prototype.sendBeacon, {
        //     apply: (target, that, args) => {
        //         if (args[0]) ctx.url.wrap(args[0], { ...ctx.meta, flags: ['xhr'], });
        //         return Reflect.apply(target, that, args);
        //     },
        // });
        let object_clone = object.clone();
        let closure = Closure::wrap(Box::new(
            move |target: JsValue, that: JsValue, args: JsValue| {
                let args = Array::from(&args);
                let target = target.dyn_into::<Function>().unwrap_or_default();
                let url = Reflect::apply(&target, &that, &args).unwrap_or_default();
                let wrapped_url = url_wrap(
                    url.as_string().unwrap_or_default(),
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

                JsValue::from(wrapped_url)
            },
        )
            as Box<dyn FnMut(JsValue, JsValue, JsValue) -> JsValue>);
        let handler = Object::new();
        let _ = Reflect::set(&handler, &"apply".into(), &closure.as_ref().into());
        let send_beacon_proxy = Proxy::new(&send_beacon, &handler);
        let _ = Reflect::set(
            &navigator_prototype,
            &"sendBeacon".into(),
            &send_beacon_proxy.into(),
        );
        closure.forget();
    }

    // set window.XMLHttpRequest
    let xml_http_request = Reflect::get(&window, &"XMLHttpRequest".into()).unwrap_or_default();
    if !xml_http_request.is_undefined() {
        let xml_http_request_prototype =
            Reflect::get(&xml_http_request, &"prototype".into()).unwrap_or_default();
        // const responseURL = Object.getOwnPropertyDescriptor(ctx.window.XMLHttpRequest.prototype, 'responseURL');
        let response_url =
            Reflect::get(&xml_http_request_prototype, &"responseURL".into()).unwrap_or_default();

        // ctx.window.XMLHttpRequest.prototype.open = new Proxy(ctx.window.XMLHttpRequest.prototype.open, {
        //     apply: (target, that, args) => {
        //         if (args[1]) args[1] = ctx.url.wrap(args[1], { ...ctx.meta, flags: ['xhr'], });
        //         return Reflect.apply(target, that, args);
        //     },
        // });
        let open = Reflect::get(&xml_http_request_prototype, &"open".into()).unwrap_or_default();
        let object_clone = object.clone();
        let closure = Closure::wrap(Box::new(
            move |target: JsValue, that: JsValue, args: JsValue| {
                let args = Array::from(&args);
                let url = args.get(1).as_string().unwrap_or_default();
                let wrapped_url = url_wrap(
                    url,
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
                let _ = Reflect::set(&args, &1.into(), &wrapped_url.into());
                let target = target.dyn_into::<Function>().unwrap_or_default();
                let result = Reflect::apply(&target, &that, &args).unwrap_or_default();
                result
            },
        )
            as Box<dyn FnMut(JsValue, JsValue, JsValue) -> JsValue>);
        let handler = Object::new();
        let _ = Reflect::set(&handler, &"apply".into(), &closure.as_ref().into());
        let open_proxy = Proxy::new(&open, &handler);
        let _ = Reflect::set(
            &xml_http_request_prototype,
            &"open".into(),
            &open_proxy.into(),
        );
        closure.forget();

        // Object.defineProperty(ctx.window.XMLHttpRequest.prototype, 'responseURL', {
        //     get: new Proxy(responseURL.get, {
        //         apply: (target, that, args) => {
        //             const url = Reflect.apply(target, that, args);
        //             return url ? ctx.url.unwrap(url, ctx.meta) : url;
        //         },
        //     }),
        // });
        let response_url_get = Reflect::get_own_property_descriptor(
            &xml_http_request_prototype.into(),
            &"responseURL".into(),
        )
        .unwrap_or_default();
        web_sys::console::log_2(&"response_url_get".into(), &response_url_get);
        let object_clone = object.clone();
        let closure = Closure::wrap(Box::new(
            move |target: JsValue, that: JsValue, args: JsValue| {
                let args = Array::from(&args);
                let target = target.dyn_into::<Function>().unwrap_or_default();
                let url = Reflect::apply(&target, &that, &args).unwrap_or_default();
                let unwrapped_url = url_unwrap(
                    url.as_string().unwrap_or_default(),
                    Reflect::get(&object_clone, &"encoding".into())
                        .unwrap_or_default()
                        .as_string()
                        .unwrap_or_default(),
                );

                JsValue::from(unwrapped_url)
            },
        )
            as Box<dyn FnMut(JsValue, JsValue, JsValue) -> JsValue>);
        let handler = Object::new();
        let _ = Reflect::set(&handler, &"apply".into(), &closure.as_ref().into());
        let response_url_get_proxy = Proxy::new(&response_url_get, &handler);
        let _ = Reflect::set(&response_url, &"get".into(), &response_url_get_proxy.into());
        closure.forget();
    }

    // set window.postMessage
    let post_message = Reflect::get(&window, &"postMessage".into()).unwrap_or_default();
    if !post_message.is_undefined() {
        // ctx.window.postMessage = new Proxy(ctx.window.postMessage, {
        //     apply: (target, that, args) => {
        //         if (!ctx.serviceWorker && args[1]) args[1] = ctx.meta.origin;
        //         return Reflect.apply(target, that, args);
        //     },
        // });
        let object_clone = object.clone();
        let closure = Closure::wrap(Box::new(
            move |target: JsValue, that: JsValue, args: JsValue| {
                let args = Array::from(&args);
                let origin = Reflect::get(
                    &Reflect::get(&object_clone, &"location".into()).unwrap_or_default(),
                    &"origin".into(),
                )
                .unwrap_or_default()
                .as_string()
                .unwrap_or_default();
                let _ = Reflect::set(&args, &1.into(), &origin.into());
                let target = target.dyn_into::<Function>().unwrap_or_default();
                let result = Reflect::apply(&target, &that, &args).unwrap_or_default();
                result
            },
        )
            as Box<dyn FnMut(JsValue, JsValue, JsValue) -> JsValue>);
        let handler = Object::new();
        let _ = Reflect::set(&handler, &"apply".into(), &closure.as_ref().into());
        let post_message_proxy = Proxy::new(&post_message, &handler);
        let _ = Reflect::set(&window, &"postMessage".into(), &post_message_proxy.into());
        closure.forget();
    }

    // set window.WebSocket
    let websocket = Reflect::get(&window, &"WebSocket".into()).unwrap_or_default();
    if !websocket.is_undefined() {
        // ctx.window.WebSocket = new Proxy(ctx.window.WebSocket, {
        //     construct: (target, args) => {
        //         if (args[0]) args[0] = ctx.url.wrap(args[0].toString().replace('ws', 'http'), ctx.meta).replace('http', 'ws') + '?origin=' + ctx.location.origin;
        //         return Reflect.construct(target, args);
        //     },
        // });
        let object_clone = object.clone();
        let closure = Closure::wrap(Box::new(
            move |target: JsValue, args: JsValue, _new_target: JsValue| {
                let args = Array::from(&args);
                let url = Reflect::get(&args, &0.into())
                    .unwrap_or_default()
                    .as_string()
                    .unwrap_or_default();
                let origin = Reflect::get(
                    &Reflect::get(&object_clone, &"location".into()).unwrap_or_default(),
                    &"origin".into(),
                )
                .unwrap_or_default()
                .as_string()
                .unwrap_or_default();
                let wrapped_url = format!(
                    "/ws{}?origin={}",
                    url_wrap(
                        url.replace("ws", "http"),
                        origin.clone(),
                        Reflect::get(&object_clone, &"encoding".into())
                            .unwrap_or_default()
                            .as_string()
                            .unwrap_or_default(),
                    )
                    .replace("http", "ws"),
                    origin
                );
                let _ = Reflect::set(&args, &0.into(), &wrapped_url.into());
                let target = target.dyn_into::<Function>().unwrap_or_default();
                let result = Reflect::construct(&target, &args).unwrap_or_default();
                result
            },
        )
            as Box<dyn FnMut(JsValue, JsValue, JsValue) -> JsValue>);
        let handler = Object::new();
        let _ = Reflect::set(&handler, &"construct".into(), &closure.as_ref().into());
        let websocket_proxy = Proxy::new(&websocket, &handler);
        let _ = Reflect::set(&window, &"WebSocket".into(), &websocket_proxy.into());
        closure.forget();
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
        "b64" => web_sys::window()
            .expect("no global `window` exists")
            .btoa(text.as_str())
            .unwrap_or_default(),
        _ => text,
    };

    text
}

#[wasm_bindgen]
pub fn decode(text: String, encoding: String) -> String {
    let text = match encoding.as_str() {
        "b64" => web_sys::window()
            .expect("no global `window` exists")
            .atob(text.as_str())
            .unwrap_or_default(),
        _ => text,
    };

    text
}
