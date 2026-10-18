//! A `User-Agent` value that names the app, its platform and rok-ui.

/// A `User-Agent` for the app's requests: `notes/1.2.0 (linux; x86_64) rok-ui/0.9.0`. Servers
/// log it, and it tells them which build sent a request. Pass it to the client:
///
/// ```
/// use rok_ui::http::{describe_user_agent, HttpClient};
///
/// let client = HttpClient::builder()
///     .header(
///         "User-Agent",
///         describe_user_agent(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
///     )
///     .build();
/// # let _ = client;
/// ```
///
/// Characters a product name cannot hold (spaces, separators) become `-`; an empty `version`
/// is left out.
#[must_use]
pub fn describe_user_agent(app: &str, version: &str) -> String {
    let token = |text: &str| -> String {
        text.chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(character) {
                    character
                } else {
                    '-'
                }
            })
            .collect()
    };
    let product = if version.is_empty() {
        token(app)
    } else {
        format!("{}/{}", token(app), token(version))
    };
    format!(
        "{product} ({}; {}) rok-ui/{}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        env!("CARGO_PKG_VERSION")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agents_name_the_app_platform_and_rok_ui() {
        let platform = format!("({}; {})", std::env::consts::OS, std::env::consts::ARCH);
        let rok_ui = format!("rok-ui/{}", env!("CARGO_PKG_VERSION"));
        assert_eq!(
            describe_user_agent("notes", "1.2.0"),
            format!("notes/1.2.0 {platform} {rok_ui}")
        );
        // Product names are tokens: no spaces or separators.
        assert_eq!(
            describe_user_agent("My Notes (beta)", ""),
            format!("My-Notes--beta- {platform} {rok_ui}")
        );
    }
}
