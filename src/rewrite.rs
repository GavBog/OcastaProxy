use base64::encode;
use lol_html::{element, html_content::ContentType, text, HtmlRewriter, Settings};
use regex::Regex;

fn get_url(el: String, origin: String, encoding: String) -> String {
    let mut attribute = el;

    if attribute.starts_with("data:")
        || attribute.starts_with("about:")
        || attribute.starts_with("javascript:")
        || attribute.starts_with("blob:")
        || attribute.starts_with("mailto:")
    {
        return attribute;
    }

    if attribute.starts_with("./") {
        attribute = attribute[2..].to_string();
    }

    if attribute.starts_with(format!("/{}/", encoding).as_str()) {
        return attribute;
    }

    if attribute.starts_with("//") {
        attribute = format!("https:{}", attribute);
    }

    let valid_protocol = attribute.starts_with("http://") || attribute.starts_with("https://");

    if !origin.ends_with("/")
        && !attribute.starts_with("/")
        && !attribute.starts_with("http:")
        && !attribute.starts_with("https:")
    {
        attribute = format!("/{}", attribute);
    }

    attribute = if valid_protocol {
        attribute
    } else {
        format!("{}{}", origin, attribute)
    };

    attribute = match encoding.as_str() {
        "b64" => encode(attribute),
        _ => attribute,
    };

    attribute = format!("/{}/{}", encoding, attribute);

    return attribute;
}

fn rewritecss(text: String, encoding: String, origin: String) -> String {
    let mut text = text;

    // replace css url with proxy url
    let re = Regex::new(r"url\((.*?)\)").unwrap();
    text = re
        .replace_all(&text, |caps: &regex::Captures| {
            let url = caps.get(1).unwrap().as_str();
            let url = get_url(url.to_string(), origin.clone(), encoding.clone());
            format!("url({})", url)
        })
        .to_string();

    return text;
}

fn rewritejs(url: reqwest::Url, text: String) -> String {
    let mut text = text.as_str().to_string();

    if url
        .to_string()
        .starts_with("https://www.googletagmanager.com/gtm.js")
    {
        text = text.replace("t.location", "t.$Ocasta.location");
    }

    // replace window.location and document.location with proxy location
    let re = Regex::new(r"(,| |=|\()(window.location|document.location)(,| |=|\)|\.)").unwrap();
    text = re
        .replace_all(&text, |caps: &regex::Captures| {
            let mut text = caps.get(0).unwrap().as_str().to_string();
            text = text.replace(".location", ".$Ocasta.location");
            text
        })
        .to_string();

    return text;
}

fn html(page: String, url: reqwest::Url, encoding: String, origin: String) -> String {
    let mut output = vec![];
    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![
                element!("head", |el| {
                    el.prepend(
                        format!("<script>{}</script>", include_str!("../static/client.js"))
                            .as_str(),
                        ContentType::Html,
                    );

                    Ok(())
                }),
                // remove attributes that interfere with proxy
                element!("[http-equiv]", |el| {
                    let attribute = el.get_attribute("http-equiv").unwrap_or_default();
                    if attribute.contains("content-security-policy") {
                        el.remove();
                    }
                    Ok(())
                }),
                element!("[integrity]", |el| {
                    el.remove_attribute("integrity");
                    Ok(())
                }),
                element!("[nonce]", |el| {
                    el.remove_attribute("nonce");
                    Ok(())
                }),
                // Test
                element!("[data-jsarwt]", |el| {
                    el.remove_attribute("data-jsarwt");
                    Ok(())
                }),
                // End Test
                // URLs
                element!("[src]", |el| {
                    let attribute = el.get_attribute("src").unwrap_or_default();
                    let attribute = get_url(attribute, origin.clone(), encoding.clone());

                    el.set_attribute("src", attribute.as_str()).unwrap();
                    Ok(())
                }),
                element!("[href]", |el| {
                    let attribute = el.get_attribute("href").unwrap_or_default();
                    let attribute = get_url(attribute, origin.clone(), encoding.clone());

                    el.set_attribute("href", attribute.as_str()).unwrap();
                    Ok(())
                }),
                element!("[action]", |el| {
                    let attribute = el.get_attribute("action").unwrap_or_default();
                    let attribute = get_url(attribute, origin.clone(), encoding.clone());

                    el.set_attribute("action", attribute.as_str()).unwrap();
                    Ok(())
                }),
                element!("[srcset]", |el| {
                    let attribute = el.get_attribute("srcset").unwrap_or_default();
                    let mut new_attribute = String::new();

                    for url in attribute.split(",") {
                        let url = url.trim();
                        let url = get_url(url.to_string(), origin.clone(), encoding.clone());
                        new_attribute.push_str(&format!("{}, ", url));
                    }

                    el.set_attribute("srcset", new_attribute.as_str()).unwrap();
                    Ok(())
                }),
                // CSS
                text!("style", |t| {
                    let text = t.as_str().to_string();
                    let text = rewritecss(text, encoding.clone(), origin.clone());

                    t.replace(text.as_str(), ContentType::Html);
                    Ok(())
                }),
                element!("[style]", |el| {
                    let attribute = el.get_attribute("style").unwrap_or_default();
                    let attribute =
                        rewritecss(attribute.to_string(), encoding.clone(), origin.clone());

                    el.set_attribute("style", attribute.as_str()).unwrap();
                    Ok(())
                }),
                // Javascript
                element!("[onclick]", |el| {
                    let attribute = el.get_attribute("onclick").unwrap_or_default();
                    let attribute = rewritejs(url.clone(), attribute.to_string());

                    el.set_attribute("onclick", attribute.as_str()).unwrap();
                    Ok(())
                }),
                text!("script", |t| {
                    let text = rewritejs(url.clone(), t.as_str().to_string());
                    t.replace(text.as_str(), ContentType::Html);

                    Ok(())
                }),
            ],
            ..Settings::default()
        },
        |c: &[u8]| output.extend_from_slice(c),
    );

    rewriter.write(page.as_bytes()).unwrap();
    rewriter.end().unwrap();

    let page = String::from_utf8(output).unwrap();
    return page;
}

pub fn page(
    page: String,
    url: reqwest::Url,
    encoding: String,
    content_type: String,
    origin: String,
) -> String {
    if content_type.starts_with("text/html") {
        return html(page, url, encoding, origin);
    } else if content_type.starts_with("text/css") {
        return rewritecss(page, encoding, origin);
    } else if content_type.starts_with("text/javascript")
        || content_type.starts_with("application/javascript")
    {
        return rewritejs(url, page);
    }

    return page;
}
