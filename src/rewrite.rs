use base64::encode;
use lol_html::{element, HtmlRewriter, Settings};

fn get_url(el: String, origin: String, encoding: String) -> String {
    let mut attribute = el;

    if attribute.starts_with("data:")
        || attribute.starts_with("about:")
        || attribute.starts_with("javascript:")
        || attribute.starts_with("blob:")
    {
        return attribute;
    } else if attribute.starts_with("./") {
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

pub fn html(page: String, url: reqwest::Url, encoding: String) -> String {
    let mut output = vec![];
    let origin = url.origin().ascii_serialization();
    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![
                element!("base[href]", |el| {
                    // Temporary fix for base tag!
                    el.remove();
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
                element!("[src], [href], [action]", |el| {
                    let mut attribute = el.get_attribute("src").unwrap_or_default();
                    if attribute.is_empty() {
                        attribute = el.get_attribute("href").unwrap_or_default();
                    }
                    if attribute.is_empty() {
                        attribute = el.get_attribute("action").unwrap_or_default();
                    }

                    let attribute = get_url(attribute, origin.clone(), encoding.clone());

                    el.set_attribute("src", attribute.as_str()).unwrap();
                    el.set_attribute("href", attribute.as_str()).unwrap();
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
