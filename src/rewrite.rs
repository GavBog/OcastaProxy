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
                // Todo: replace all src, href, action, srcset, onclick, etc. regardless of the tag. ("[href]" instead of "a[href]")
                element!("a[href]", |el| {
                    let mut href = el.get_attribute("href").unwrap();
                    href = get_url(href, origin.clone(), encoding.clone());

                    el.set_attribute("href", &href)?;
                    Ok(())
                }),
                element!("link[href]", |el| {
                    let mut href = el.get_attribute("href").unwrap();
                    href = format!("{}{}", origin, href);

                    el.set_attribute("href", &href)?;
                    Ok(())
                }),
                element!("form[action]", |el| {
                    let mut href = el.get_attribute("action").unwrap();
                    href = get_url(href, origin.clone(), encoding.clone());

                    el.set_attribute("action", &href)?;
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
