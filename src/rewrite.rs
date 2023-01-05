use base64::encode;
use lol_html::{element, HtmlRewriter, Settings};

pub fn html(page: String, url: reqwest::Url, encoding: String) -> String {
    let origin = url.origin().ascii_serialization().to_string();
    let mut output = vec![];
    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![
                element!("a[href]", |el| {
                    let href;
                    if el.get_attribute("href").unwrap().starts_with("/") {
                        let new_url = format!("{}{}", origin, el.get_attribute("href").unwrap());
                        href = match encoding.as_str() {
                            "b64" => format!("/{}/{}", encoding, encode(new_url)),
                            _ => format!("/{}/{}", encoding, new_url),
                        };
                    } else {
                        href = match encoding.as_str() {
                            "b64" => format!(
                                "/{}/{}",
                                encoding,
                                encode(el.get_attribute("href").unwrap())
                            ),
                            _ => format!("/{}/{}", encoding, el.get_attribute("href").unwrap()),
                        };
                    }
                    el.set_attribute("href", &href)?;

                    Ok(())
                }),
                element!("form[action]", |el| {
                    let action;
                    if el.get_attribute("action").unwrap().starts_with("/") {
                        let new_url = format!("{}{}", origin, el.get_attribute("action").unwrap());
                        action = match encoding.as_str() {
                            "b64" => format!("/{}/{}", encoding, encode(new_url)),
                            _ => format!("/{}/{}", encoding, new_url),
                        };
                    } else {
                        action = match encoding.as_str() {
                            "b64" => format!(
                                "/{}/{}",
                                encoding,
                                encode(el.get_attribute("action").unwrap())
                            ),
                            _ => format!("/{}/{}", encoding, el.get_attribute("action").unwrap()),
                        };
                    }
                    el.set_attribute("action", &action)?;

                    Ok(())
                }),
                element!("img[src]", |el| {
                    if el.get_attribute("src").unwrap().starts_with("/") {
                        let src = format!("{}{}", origin, el.get_attribute("src").unwrap());
                        el.set_attribute("src", &src)?;
                    }

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
