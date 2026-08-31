//! Paths and credentials for Google Sheets write access.
//!
//! Prefers a service account JSON; otherwise OAuth desktop flow with
//! `credentials.json` + cached `token.json` (same layout as the Python tool).

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const SCOPES: &[&str] = &["https://www.googleapis.com/auth/spreadsheets",
    "https://www.googleapis.com/auth/drive.readonly"];

const CREDENTIALS_NAMES: &[&str] = &[
    "credentials.json",
    "client_secret.json",
    "oauth_credentials.json",
];
const SERVICE_ACCOUNT_NAMES: &[&str] = &["service_account.json", "service-account.json"];

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("{0}")]
    Message(String),
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP: {0}")]
    Http(String),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl AuthError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

/// Resolve the data directory for credentials, tokens, audits, and column config.
///
/// Order:
/// 1. `AWARDS_ROOT`
/// 2. cwd if it already holds project/auth files
/// 3. `~/.config/awards-tui` (created if needed) for installed binaries
/// 4. cargo workspace root when developing from a checkout
pub fn project_root() -> PathBuf {
    if let Ok(root) = std::env::var("AWARDS_ROOT") {
        return PathBuf::from(root);
    }
    if let Ok(cwd) = std::env::current_dir() {
        if looks_like_project_dir(&cwd) {
            return cwd;
        }
    }
    if let Some(config) = dirs::config_dir() {
        let dir = config.join("awards-tui");
        if looks_like_project_dir(&dir) || dir.is_dir() {
            let _ = std::fs::create_dir_all(&dir);
            return dir;
        }
        let _ = std::fs::create_dir_all(&dir);
        return dir;
    }
    let cargo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    if looks_like_project_dir(&cargo_root) {
        return cargo_root;
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn looks_like_project_dir(dir: &Path) -> bool {
    [
        "award_columns.json",
        "credentials.json",
        "client_secret.json",
        "oauth_credentials.json",
        "service_account.json",
        "service-account.json",
        "token.json",
        "awards-tui.toml",
    ]
    .iter()
    .any(|name| dir.join(name).is_file())
}

fn find_existing(root: &Path, names: &[&str]) -> Option<PathBuf> {
    names.iter().map(|n| root.join(n)).find(|p| p.is_file())
}

pub fn credentials_path() -> Option<PathBuf> {
    find_existing(&project_root(), CREDENTIALS_NAMES)
}

pub fn service_account_path() -> Option<PathBuf> {
    find_existing(&project_root(), SERVICE_ACCOUNT_NAMES)
}

pub fn token_path() -> PathBuf {
    project_root().join("token.json")
}

pub fn auth_status() -> &'static str {
    if service_account_path().is_some() {
        return "service_account";
    }
    let path = token_path();
    if path.is_file() {
        match load_authorized_user(&path) {
            Ok(tok) if tok.usable() => "oauth_token",
            _ => "oauth_needs_login",
        }
    } else if credentials_path().is_some() {
        "oauth_needs_login"
    } else {
        "missing"
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AuthorizedUser {
    token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    token_uri: Option<String>,
    client_id: String,
    client_secret: String,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default)]
    expiry: Option<String>,
    #[serde(default)]
    account: Option<String>,
    #[serde(default)]
    universe_domain: Option<String>,
}

impl AuthorizedUser {
    fn usable(&self) -> bool {
        if !self.token.is_empty() && !self.expired() {
            return true;
        }
        self.refresh_token.as_ref().is_some_and(|r| !r.is_empty())
    }

    fn expired(&self) -> bool {
        let Some(expiry) = &self.expiry else {
            return false;
        };
        // google-auth writes "2026-08-14T12:00:00Z" or with fractional seconds
        let parsed = DateTime::parse_from_rfc3339(expiry)
            .or_else(|_| DateTime::parse_from_str(expiry, "%Y-%m-%dT%H:%M:%S%.fZ"))
            .or_else(|_| DateTime::parse_from_str(expiry, "%Y-%m-%dT%H:%M:%SZ"));
        match parsed {
            Ok(dt) => Utc::now() >= dt.with_timezone(&Utc) - Duration::seconds(60),
            Err(_) => false,
        }
    }
}

fn load_authorized_user(path: &Path) -> Result<AuthorizedUser, AuthError> {
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

fn save_authorized_user(path: &Path, tok: &AuthorizedUser) -> Result<(), AuthError> {
    let text = serde_json::to_string_pretty(tok)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    #[cfg(unix)]
    {
        use std::fs::OpenOptions;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(text.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, text)?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct InstalledClientFile {
    installed: Option<InstalledClient>,
    web: Option<InstalledClient>,
}

#[derive(Debug, Deserialize)]
struct InstalledClient {
    client_id: String,
    client_secret: String,
    #[serde(default)]
    token_uri: Option<String>,
    #[serde(default)]
    auth_uri: Option<String>,
}

fn load_oauth_client(path: &Path) -> Result<InstalledClient, AuthError> {
    let text = std::fs::read_to_string(path)?;
    let file: InstalledClientFile = serde_json::from_str(&text)?;
    file.installed
        .or(file.web)
        .ok_or_else(|| AuthError::msg("credentials.json missing installed/web client"))
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
    #[serde(default)]
    scope: Option<String>,
}

fn http_form_post(url: &str, form: &[(&str, &str)]) -> Result<TokenResponse, AuthError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| AuthError::Http(e.to_string()))?;
    let resp = client
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(form)
        .send()
        .map_err(|e| AuthError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(AuthError::Http(format!("token endpoint {status}: {body}")));
    }
    resp.json().map_err(|e| AuthError::Http(e.to_string()))
}

fn apply_token_response(tok: &mut AuthorizedUser, resp: TokenResponse) {
    tok.token = resp.access_token;
    if let Some(refresh) = resp.refresh_token.filter(|s| !s.is_empty()) {
        tok.refresh_token = Some(refresh);
    }
    if let Some(secs) = resp.expires_in {
        let expiry = Utc::now() + Duration::seconds(secs);
        tok.expiry = Some(expiry.format("%Y-%m-%dT%H:%M:%SZ").to_string());
    }
    if let Some(scope) = resp.scope {
        tok.scopes = scope.split_whitespace().map(str::to_string).collect();
    }
    if tok.scopes.is_empty() {
        tok.scopes = SCOPES.iter().map(|s| (*s).to_string()).collect();
    }
    if tok.universe_domain.is_none() {
        tok.universe_domain = Some("googleapis.com".into());
    }
}

fn refresh_authorized_user(tok: &mut AuthorizedUser) -> Result<(), AuthError> {
    let refresh = tok
        .refresh_token
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AuthError::msg("Token refresh failed. Run: awards-tui --login"))?;
    let token_uri = tok
        .token_uri
        .clone()
        .unwrap_or_else(|| "https://oauth2.googleapis.com/token".into());
    let resp = http_form_post(
        &token_uri,
        &[
            ("client_id", &tok.client_id),
            ("client_secret", &tok.client_secret),
            ("refresh_token", refresh),
            ("grant_type", "refresh_token"),
        ],
    )
    .map_err(|e| {
        AuthError::msg(format!(
            "Token refresh failed. Run: awards-tui --login ({e})"
        ))
    })?;
    apply_token_response(tok, resp);
    save_authorized_user(&token_path(), tok)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ServiceAccountFile {
    client_email: String,
    private_key: String,
    #[serde(default)]
    token_uri: Option<String>,
}

#[derive(Debug, Serialize)]
struct SaClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

fn access_token_from_service_account(path: &Path) -> Result<String, AuthError> {
    warn_if_insecure_secret_file(path);
    let text = std::fs::read_to_string(path)?;
    let sa: ServiceAccountFile = serde_json::from_str(&text)?;
    let now = Utc::now().timestamp();
    let claims = SaClaims {
        iss: sa.client_email.clone(),
        scope: SCOPES.join(" "),
        aud: sa
            .token_uri
            .clone()
            .unwrap_or_else(|| "https://oauth2.googleapis.com/token".into()),
        iat: now,
        exp: now + 3600,
    };
    let key = EncodingKey::from_rsa_pem(sa.private_key.as_bytes())
        .map_err(|e| AuthError::msg(format!("service account key: {e}")))?;
    let mut header = Header::new(Algorithm::RS256);
    header.typ = Some("JWT".into());
    let assertion =
        encode(&header, &claims, &key).map_err(|e| AuthError::msg(format!("JWT sign: {e}")))?;
    let token_uri = sa
        .token_uri
        .unwrap_or_else(|| "https://oauth2.googleapis.com/token".into());
    let resp = http_form_post(
        &token_uri,
        &[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &assertion),
        ],
    )?;
    Ok(resp.access_token)
}

/// Warn when a secret JSON file is group/world-readable (Unix only).
fn warn_if_insecure_secret_file(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let mode = meta.permissions().mode() & 0o777;
            if secret_mode_too_open(mode) {
                eprintln!(
                    "warning: {} is readable by group/others (mode {mode:03o}); chmod 600 recommended",
                    path.display()
                );
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

#[cfg(unix)]
fn secret_mode_too_open(mode: u32) -> bool {
    mode & 0o077 != 0
}

/// Return a bearer access token for Sheets write access.
pub fn get_access_token(interactive: bool) -> Result<String, AuthError> {
    if let Some(sa) = service_account_path() {
        return access_token_from_service_account(&sa);
    }

    let path = token_path();
    if path.is_file() {
        let mut tok = load_authorized_user(&path)?;
        if !tok.expired() && !tok.token.is_empty() {
            return Ok(tok.token);
        }
        if tok.refresh_token.as_ref().is_some_and(|r| !r.is_empty()) {
            refresh_authorized_user(&mut tok)?;
            return Ok(tok.token);
        }
    }

    let Some(oauth_path) = credentials_path() else {
        return Err(AuthError::msg(
            "No credentials found. Place OAuth client JSON as credentials.json \
             (or service_account.json shared on the sheet), then run: awards-tui --login",
        ));
    };

    if !interactive {
        return Err(AuthError::msg("Not logged in. Run: awards-tui --login"));
    }

    interactive_login(&oauth_path)
}

fn interactive_login(oauth_path: &Path) -> Result<String, AuthError> {
    let client = load_oauth_client(oauth_path)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    let redirect = format!("http://127.0.0.1:{port}/");
    let auth_uri = client
        .auth_uri
        .clone()
        .unwrap_or_else(|| "https://accounts.google.com/o/oauth2/auth".into());
    let scope = SCOPES.join(" ");
    let state = oauth_state();
    let auth_url = format!(
        "{auth_uri}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&access_type=offline&prompt=consent",
        urlencoding::encode(&client.client_id),
        urlencoding::encode(&redirect),
        urlencoding::encode(&scope),
        urlencoding::encode(&state),
    );

    if open::that(&auth_url).is_err() {
        eprintln!("Open this URL in a browser:\n{auth_url}");
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
    let (mut stream, _) = loop {
        match listener.accept() {
            Ok(conn) => break conn,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    return Err(AuthError::msg(
                        "OAuth timed out after 5 minutes waiting for the browser callback",
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(err) => return Err(err.into()),
        }
    };
    stream.set_nonblocking(false)?;
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let first_line = req.lines().next().unwrap_or("");
    if let Some(error) = extract_query_param(first_line, "error") {
        let desc = extract_query_param(first_line, "error_description").unwrap_or_default();
        return Err(AuthError::msg(format!(
            "OAuth provider returned error={error} {desc}"
        )));
    }
    let returned_state = extract_query_param(first_line, "state").unwrap_or_default();
    if returned_state != state {
        return Err(AuthError::msg(
            "OAuth callback state mismatch (possible CSRF). Try --login again.",
        ));
    }
    let code = extract_query_param(first_line, "code")
        .ok_or_else(|| AuthError::msg("OAuth callback missing code"))?;
    let body = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<html><body><h2>Logged in</h2><p>You can close this tab and return to the terminal.</p></body></html>";
    let _ = stream.write_all(body);

    let token_uri = client
        .token_uri
        .clone()
        .unwrap_or_else(|| "https://oauth2.googleapis.com/token".into());
    let resp = http_form_post(
        &token_uri,
        &[
            ("code", &code),
            ("client_id", &client.client_id),
            ("client_secret", &client.client_secret),
            ("redirect_uri", &redirect),
            ("grant_type", "authorization_code"),
        ],
    )?;

    let mut tok = AuthorizedUser {
        token: String::new(),
        refresh_token: None,
        token_uri: Some(token_uri),
        client_id: client.client_id,
        client_secret: client.client_secret,
        scopes: SCOPES.iter().map(|s| (*s).to_string()).collect(),
        expiry: None,
        account: None,
        universe_domain: Some("googleapis.com".into()),
    };
    apply_token_response(&mut tok, resp);
    save_authorized_user(&token_path(), &tok)?;
    Ok(tok.token)
}

fn oauth_state() -> String {
    let mut bytes = [0u8; 16];
    if getrandom::getrandom(&mut bytes).is_ok() {
        return bytes.iter().map(|b| format!("{b:02x}")).collect();
    }
    // Extremely unlikely; keep login usable if the OS RNG is unavailable.
    format!(
        "{:x}-{:x}",
        Utc::now().timestamp_nanos_opt().unwrap_or(0),
        std::process::id()
    )
}

fn extract_query_param(request_line: &str, key: &str) -> Option<String> {
    // GET /?code=...&scope=... HTTP/1.1
    let path = request_line.split_whitespace().nth(1)?;
    let query = path.split_once('?')?.1;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=')?;
        if k == key {
            return Some(urlencoding::decode(v).ok()?.into_owned());
        }
    }
    None
}

/// Force interactive OAuth login and cache token.json, or validate a service account.
/// Returns a short user-facing success message.
pub fn login() -> Result<String, AuthError> {
    if let Some(sa) = service_account_path() {
        warn_if_insecure_secret_file(&sa);
        let text = std::fs::read_to_string(&sa)?;
        let v: serde_json::Value = serde_json::from_str(&text)?;
        let email = v
            .get("client_email")
            .and_then(|x| x.as_str())
            .unwrap_or("service_account");
        let _ = get_access_token(false)?;
        return Ok(format!(
            "Service account ready ({email}). No token.json needed."
        ));
    }
    let path = token_path();
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    let Some(oauth_path) = credentials_path() else {
        return Err(AuthError::msg(
            "No credentials found. Place OAuth client JSON as credentials.json",
        ));
    };
    let _ = interactive_login(&oauth_path)?;
    let hint = if let Ok(tok) = load_authorized_user(&token_path()) {
        tok.account
            .filter(|s| !s.is_empty())
            .unwrap_or(tok.client_id)
    } else {
        auth_status().to_string()
    };
    Ok(format!(
        "Logged in ({hint}). token.json saved at {}.",
        token_path().display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_code_from_request_line() {
        let line = "GET /?code=abc%2Fdef&scope=x HTTP/1.1";
        assert_eq!(
            extract_query_param(line, "code").as_deref(),
            Some("abc/def")
        );
    }

    #[test]
    fn extract_state_and_error() {
        let line = "GET /?error=access_denied&state=abc HTTP/1.1";
        assert_eq!(
            extract_query_param(line, "error").as_deref(),
            Some("access_denied")
        );
        assert_eq!(extract_query_param(line, "state").as_deref(), Some("abc"));
    }

    #[test]
    fn awards_root_env_wins() {
        let prev = std::env::var_os("AWARDS_ROOT");
        std::env::set_var("AWARDS_ROOT", "/tmp/awards-tui-root-test");
        assert_eq!(project_root(), PathBuf::from("/tmp/awards-tui-root-test"));
        match prev {
            Some(v) => std::env::set_var("AWARDS_ROOT", v),
            None => std::env::remove_var("AWARDS_ROOT"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn secret_mode_flags_group_or_world() {
        assert!(!secret_mode_too_open(0o600));
        assert!(!secret_mode_too_open(0o400));
        assert!(secret_mode_too_open(0o640));
        assert!(secret_mode_too_open(0o644));
        assert!(secret_mode_too_open(0o606));
    }
}
