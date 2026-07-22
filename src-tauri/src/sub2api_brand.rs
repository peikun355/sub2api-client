use url::Url;

pub const PRODUCT_NAME: &str = "Sub2API Client";
pub const PROVIDER_ID: &str = "sub2api";
pub const DEFAULT_MODEL: &str = "gpt-5.6-terra";
pub const DEFAULT_REASONING_EFFORT: &str = "high";
const PRODUCTION_ORIGIN: &str = "https://ai.tavonilo.com";

pub fn trusted_site_origin() -> &'static str {
    #[cfg(debug_assertions)]
    {
        option_env!("SUB2API_DEV_ORIGIN").unwrap_or(PRODUCTION_ORIGIN)
    }
    #[cfg(not(debug_assertions))]
    {
        PRODUCTION_ORIGIN
    }
}

pub fn api_url(path: &str) -> String {
    format!("{}{}", trusted_site_origin().trim_end_matches('/'), path)
}

pub fn validate_service_base_url(value: &str) -> Result<String, String> {
    let parsed = Url::parse(value.trim()).map_err(|_| "The service returned an invalid API URL")?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("The service returned an invalid API URL".into());
    }
    let origin = parsed.origin().ascii_serialization();
    let trusted = Url::parse(trusted_site_origin())
        .map_err(|_| "The compiled service origin is invalid")?
        .origin()
        .ascii_serialization();

    #[cfg(debug_assertions)]
    let allowed = origin == trusted
        && matches!(
            parsed.host_str(),
            Some("localhost" | "127.0.0.1" | "::1") | Some("ai.tavonilo.com")
        );
    #[cfg(not(debug_assertions))]
    let allowed = origin == trusted
        && parsed.scheme() == "https"
        && parsed.host_str() == Some("ai.tavonilo.com");

    if !allowed {
        return Err("The service origin is not trusted by this build".into());
    }
    Ok(value.trim().trim_end_matches('/').to_string())
}

pub fn models_url(base_url: &str) -> String {
    if base_url.trim_end_matches('/').ends_with("/v1") {
        format!("{}/models", base_url.trim_end_matches('/'))
    } else {
        format!("{}/v1/models", base_url.trim_end_matches('/'))
    }
}

pub fn parse_setup_ticket(url: &str) -> Result<String, String> {
    let parsed = Url::parse(url).map_err(|_| "Invalid Sub2API setup link")?;
    if parsed.scheme() != "sub2api" || parsed.host_str() != Some("v1") || parsed.path() != "/setup"
    {
        return Err("Invalid Sub2API setup link".into());
    }
    let pairs: Vec<_> = parsed.query_pairs().collect();
    if pairs.len() != 1 || pairs[0].0 != "ticket" {
        return Err("Invalid Sub2API setup link".into());
    }
    let ticket = pairs[0].1.as_ref();
    if ticket.len() != 43
        || !ticket
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Invalid Sub2API setup link".into());
    }
    Ok(ticket.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_link_contains_only_one_high_entropy_ticket() {
        let ticket = "a".repeat(43);
        assert_eq!(
            parse_setup_ticket(&format!("sub2api://v1/setup?ticket={ticket}")).unwrap(),
            ticket
        );
        assert!(parse_setup_ticket("sub2api://v1/setup?ticket=short").is_err());
        assert!(parse_setup_ticket(&format!(
            "sub2api://v1/setup?ticket={}&api_key=secret",
            "a".repeat(43)
        ))
        .is_err());
    }

    #[test]
    fn production_service_url_is_the_only_non_local_origin() {
        assert!(validate_service_base_url("https://ai.tavonilo.com").is_ok());
        assert!(validate_service_base_url("https://evil.example").is_err());
    }
}
