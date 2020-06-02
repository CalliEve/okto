use std::str::FromStr;

pub fn format_links(links: &Vec<String>) -> Option<String> {
    let mut res = String::new();

    for str_link in links {
        if let Ok(link) = url::Url::from_str(&str_link) {
            if let Some(domain) = link.domain() {
                res.push_str(&format!("[{}]({})", domain, &str_link));
            }
        }
    }

    if res.is_empty() {
        None
    } else {
        Some(res)
    }
}
