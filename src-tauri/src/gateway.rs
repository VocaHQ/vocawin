//! Opt-in local VocaGateway via Docker Desktop Compose.
//! Pair helper for phones and other clients. Not used for VocaWin dictation.

use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::UdpSocket,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

/// Single pin for the desktop-embed image and docs. Not `latest`.
pub const GATEWAY_RELEASE_TAG: &str = "v0.1.0";
pub const GATEWAY_IMAGE: &str = "ghcr.io/vocahq/vocagateway:v0.1.0";
pub const COMPOSE_PROJECT: &str = "vocagateway";
pub const COMPOSE_SERVICE: &str = "gateway";
pub const PUBLISH_HOST: &str = "0.0.0.0";
pub const PUBLISH_PORT: u16 = 8765;
pub const LOCAL_BASE_URL: &str = "http://127.0.0.1:8765";
pub const WEBUI_URL: &str = "http://127.0.0.1:8765/";
pub const DOCKER_DESKTOP_INSTALL_URL: &str =
    "https://docs.docker.com/desktop/setup/install/windows-install/";
pub const GATEWAY_SITE_URL: &str = "https://vocagateway.vocahq.com";

const COMPOSE_YAML: &str = include_str!("../gateway/compose.yaml");
const TOKEN_HEX_LEN: usize = 32;
const DOCKER_BIN: &str = "docker";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GatewayPhase {
    DockerMissing,
    Stopped,
    Starting,
    LiveNotReady,
    Ready,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayStatus {
    pub phase: GatewayPhase,
    pub phase_label: String,
    pub docker_available: bool,
    pub docker_detail: String,
    pub public_url: String,
    pub suggested_public_url: String,
    pub pairable: bool,
    pub live: bool,
    pub ready: bool,
    pub message: String,
    pub webui_url: String,
    pub docker_install_url: String,
    pub gateway_site_url: String,
    pub image: String,
    pub release_tag: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayPairing {
    pub url: String,
    pub qr_svg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PairingResponse {
    payload: String,
    url: Option<String>,
}

/// Reject hosts a phone on another device cannot reach.
pub fn is_loopback_public_url(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    let without_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed);
    let authority = without_scheme.split('/').next().unwrap_or("").trim();
    let host = if let Some(rest) = authority.strip_prefix('[') {
        rest.split(']').next().unwrap_or("").to_ascii_lowercase()
    } else {
        authority
            .split(':')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase()
    };
    matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1" | "0.0.0.0")
        || host.starts_with("127.")
}

pub fn validate_public_url(raw: &str) -> Result<String, String> {
    let url = raw.trim().to_string();
    if url.is_empty() {
        return Err("Set a phone-reachable public URL first (LAN or Tailscale).".into());
    }
    // Defend .env writes: reject CR/LF and other characters that can inject keys
    // or break Compose env parsing when the value is written unquoted.
    if url.chars().any(|c| {
        c.is_control()
            || c.is_whitespace()
            || matches!(c, '#' | '"' | '\'' | '`' | '$' | '=' | '@' | '\\' | ';')
    }) {
        return Err("Public URL contains invalid characters.".into());
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("Public URL must start with http:// or https://.".into());
    }
    if is_loopback_public_url(&url) {
        return Err(
            "Do not use 127.0.0.1 or localhost in the public URL. A phone cannot reach that."
                .into(),
        );
    }
    Ok(url.trim_end_matches('/').to_string())
}

pub fn generate_token_hex() -> Result<String, String> {
    let mut bytes = [0u8; TOKEN_HEX_LEN / 2];
    fill_random(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn fill_random(buf: &mut [u8]) -> Result<(), String> {
    getrandom::getrandom(buf).map_err(|error| format!("Could not generate gateway token: {error}"))
}

pub fn map_phase(docker_available: bool, container_up: bool, live: bool, ready: bool) -> GatewayPhase {
    if !docker_available {
        return GatewayPhase::DockerMissing;
    }
    if ready {
        return GatewayPhase::Ready;
    }
    if live {
        return GatewayPhase::LiveNotReady;
    }
    if container_up {
        return GatewayPhase::Starting;
    }
    GatewayPhase::Stopped
}

pub fn phase_label(phase: GatewayPhase) -> &'static str {
    match phase {
        GatewayPhase::DockerMissing => "Docker missing",
        GatewayPhase::Stopped => "Stopped",
        GatewayPhase::Starting => "Starting",
        GatewayPhase::LiveNotReady => "Live (not ready)",
        GatewayPhase::Ready => "Ready",
    }
}

pub fn suggest_lan_public_url() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    if ip.is_loopback() || ip.is_unspecified() {
        return None;
    }
    let candidate = format!("http://{ip}:{PUBLISH_PORT}");
    if is_loopback_public_url(&candidate) {
        return None;
    }
    Some(candidate)
}

pub fn gateway_dir(app_data: &Path) -> PathBuf {
    app_data.join("gateway")
}

pub fn ensure_gateway_files(dir: &Path, public_url: &str) -> Result<String, String> {
    fs::create_dir_all(dir).map_err(|error| format!("Could not create gateway directory: {error}"))?;
    let compose_path = dir.join("compose.yaml");
    fs::write(&compose_path, COMPOSE_YAML.replace(
        "ghcr.io/vocahq/vocagateway:v0.1.0",
        GATEWAY_IMAGE,
    ))
    .map_err(|error| format!("Could not write compose.yaml: {error}"))?;

    let env_path = dir.join(".env");
    let token = read_or_create_token(&env_path)?;
    write_env_file(&env_path, &token, public_url)?;
    Ok(token)
}

fn read_or_create_token(env_path: &Path) -> Result<String, String> {
    if env_path.exists() {
        if let Some(existing) = parse_env_value(&fs::read_to_string(env_path).unwrap_or_default(), "VOCAGATEWAY_TOKEN")
        {
            if existing.len() >= TOKEN_HEX_LEN {
                return Ok(existing);
            }
        }
    }
    generate_token_hex()
}

fn write_env_file(path: &Path, token: &str, public_url: &str) -> Result<(), String> {
    let mut body = String::new();
    body.push_str(&format!("VOCAGATEWAY_TOKEN={token}\n"));
    body.push_str(&format!("VOCAGATEWAY_PUBLISH_HOST={PUBLISH_HOST}\n"));
    body.push_str(&format!("VOCAGATEWAY_PUBLISH_PORT={PUBLISH_PORT}\n"));
    body.push_str(&format!("VOCAGATEWAY_PORT={PUBLISH_PORT}\n"));
    body.push_str(&format!("VOCAGATEWAY_IMAGE={GATEWAY_IMAGE}\n"));
    if !public_url.trim().is_empty() {
        body.push_str(&format!("VOCAGATEWAY_PUBLIC_URL={}\n", public_url.trim()));
    }
    write_restricted(path, &body)
}

fn write_restricted(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| format!("Could not write {}: {error}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, perms)
            .map_err(|error| format!("Could not restrict {}: {error}", path.display()))?;
    }
    Ok(())
}

pub fn parse_env_value(contents: &str, key: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((name, value)) = line.split_once('=') {
            if name.trim() == key {
                return Some(value.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

pub fn read_token(dir: &Path) -> Option<String> {
    let contents = fs::read_to_string(dir.join(".env")).ok()?;
    parse_env_value(&contents, "VOCAGATEWAY_TOKEN").filter(|t| t.len() >= TOKEN_HEX_LEN)
}

fn docker_command(args: &[&str]) -> Command {
    let mut command = Command::new(DOCKER_BIN);
    command.args(args);
    command
}

pub fn detect_docker() -> (bool, String) {
    let version = match docker_command(&["version"]).output() {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .find(|line| line.to_ascii_lowercase().contains("server"))
            .unwrap_or("Docker engine reachable")
            .trim()
            .to_string(),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return (
                false,
                if stderr.trim().is_empty() {
                    "Docker is installed but the engine is not running. Start Docker Desktop.".into()
                } else {
                    format!("Docker engine not ready: {}", stderr.trim())
                },
            );
        }
        Err(_) => {
            return (
                false,
                "Docker Desktop was not found. Install it (WSL2 required), then try again.".into(),
            );
        }
    };

    match docker_command(&["compose", "version"]).output() {
        Ok(output) if output.status.success() => {
            let compose = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (
                true,
                if compose.is_empty() {
                    version
                } else {
                    format!("{version}; {compose}")
                },
            )
        }
        _ => (
            false,
            "Docker is present but Compose is missing. Update Docker Desktop.".into(),
        ),
    }
}

fn compose_file_args(dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    let compose = dir.join("compose.yaml");
    let env = dir.join(".env");
    if !compose.exists() {
        return Err("Gateway compose.yaml is missing. Click Start to recreate it.".into());
    }
    if !env.exists() {
        return Err("Gateway .env is missing. Click Start to recreate it.".into());
    }
    Ok((compose, env))
}

fn run_compose(dir: &Path, subcommand: &[&str]) -> Result<String, String> {
    let (compose, env) = compose_file_args(dir)?;
    let compose_str = compose.to_string_lossy();
    let env_str = env.to_string_lossy();
    let mut args = vec![
        "compose",
        "-p",
        COMPOSE_PROJECT,
        "-f",
        compose_str.as_ref(),
        "--env-file",
        env_str.as_ref(),
    ];
    args.extend_from_slice(subcommand);
    let output = docker_command(&args)
        .current_dir(dir)
        .output()
        .map_err(|error| format!("Could not run docker compose: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        Err(format!("docker compose failed: {detail}"))
    }
}

pub fn container_running(dir: &Path) -> bool {
    let (compose, env) = match compose_file_args(dir) {
        Ok(paths) => paths,
        Err(_) => return false,
    };
    let compose_str = compose.to_string_lossy();
    let env_str = env.to_string_lossy();
    let output = docker_command(&[
        "compose",
        "-p",
        COMPOSE_PROJECT,
        "-f",
        compose_str.as_ref(),
        "--env-file",
        env_str.as_ref(),
        "ps",
        "-q",
        COMPOSE_SERVICE,
    ])
    .current_dir(dir)
    .output();
    match output {
        Ok(output) if output.status.success() => {
            !String::from_utf8_lossy(&output.stdout).trim().is_empty()
        }
        _ => false,
    }
}

async fn probe_status(path: &str) -> (bool, u16) {
    let url = format!("{LOCAL_BASE_URL}{path}");
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(client) => client,
        Err(_) => return (false, 0),
    };
    match client.get(&url).send().await {
        Ok(response) => {
            let code = response.status().as_u16();
            (response.status().is_success(), code)
        }
        Err(_) => (false, 0),
    }
}

pub async fn probe_live_ready() -> (bool, bool) {
    let (live_ok, _) = probe_status("/health/live").await;
    let (ready_ok, ready_code) = probe_status("/health/ready").await;
    // 503 on /health/ready still means the process answered (pairable-before-ready).
    let live = live_ok || ready_ok || ready_code == 503;
    (live, ready_ok)
}

pub async fn status(dir: &Path, public_url: &str) -> GatewayStatus {
    let (docker_available, docker_detail) = detect_docker();
    let suggested = suggest_lan_public_url().unwrap_or_default();
    let public = public_url.trim().to_string();
    let container_up = docker_available && container_running(dir);
    let (live, ready) = if docker_available {
        probe_live_ready().await
    } else {
        (false, false)
    };
    let phase = map_phase(docker_available, container_up, live, ready);
    let token_ok = read_token(dir).is_some();
    let url_ok = !public.is_empty() && !is_loopback_public_url(&public);
    let pairable = live && url_ok && token_ok;
    let message = match phase {
        GatewayPhase::DockerMissing => {
            "Install Docker Desktop for Windows (WSL2 required), then return here.".into()
        }
        GatewayPhase::Stopped => {
            if public.is_empty() {
                "Set a non-loopback public URL, then Start.".into()
            } else if is_loopback_public_url(&public) {
                "Replace localhost/127.0.0.1 with a LAN or Tailscale URL.".into()
            } else {
                "Gateway is stopped.".into()
            }
        }
        GatewayPhase::Starting => "Container is up. Waiting for /health/live...".into(),
        GatewayPhase::LiveNotReady => {
            "Gateway is live. Model may still be downloading. Pairing can work before Ready.".into()
        }
        GatewayPhase::Ready => "Gateway is ready for dictation clients.".into(),
    };
    GatewayStatus {
        phase,
        phase_label: phase_label(phase).into(),
        docker_available,
        docker_detail,
        public_url: public,
        suggested_public_url: suggested,
        pairable,
        live,
        ready,
        message,
        webui_url: WEBUI_URL.into(),
        docker_install_url: DOCKER_DESKTOP_INSTALL_URL.into(),
        gateway_site_url: GATEWAY_SITE_URL.into(),
        image: GATEWAY_IMAGE.into(),
        release_tag: GATEWAY_RELEASE_TAG.into(),
    }
}

pub fn start(dir: &Path, public_url: &str) -> Result<(), String> {
    let (docker_ok, detail) = detect_docker();
    if !docker_ok {
        return Err(detail);
    }
    let url = validate_public_url(public_url)?;
    ensure_gateway_files(dir, &url)?;
    // Prefer a registry pull of the pinned tag. If GHCR has no public image yet,
    // operators can shallow-clone tag GATEWAY_RELEASE_TAG into this directory and
    // run `docker compose … up -d --build` (see docs/gateway-embed.md).
    let _ = run_compose(dir, &["pull", COMPOSE_SERVICE]);
    run_compose(dir, &["up", "-d", COMPOSE_SERVICE]).map(|_| ())
}

pub fn stop(dir: &Path) -> Result<(), String> {
    let (docker_ok, detail) = detect_docker();
    if !docker_ok {
        return Err(detail);
    }
    if !dir.join("compose.yaml").exists() {
        return Ok(());
    }
    // Never pass --volumes; model data in the named volume must survive Stop.
    run_compose(dir, &["down"]).map(|_| ())
}

pub fn set_public_url(dir: &Path, public_url: &str) -> Result<String, String> {
    let url = if public_url.trim().is_empty() {
        String::new()
    } else {
        validate_public_url(public_url)?
    };
    if dir.exists() || !url.is_empty() {
        let _ = ensure_gateway_files(dir, &url);
    }
    Ok(url)
}

pub async fn pairing(dir: &Path, public_url: &str) -> Result<GatewayPairing, String> {
    let url = validate_public_url(public_url)?;
    let token = read_token(dir).ok_or_else(|| {
        "Gateway token is missing. Start Gateway once to create the .env file.".to_string()
    })?;
    let (live, _) = probe_live_ready().await;
    if !live {
        return Err("Gateway is not live yet. Wait until status is Live or Ready.".into());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|error| format!("Could not build HTTP client: {error}"))?;
    let endpoint = format!(
        "{LOCAL_BASE_URL}/v1/admin/pairing?url={}",
        urlencoding_minimal(&url)
    );
    let response = client
        .get(&endpoint)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|error| format!("Pairing request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Pairing returned HTTP {}", response.status()));
    }
    let body = response
        .text()
        .await
        .map_err(|error| format!("Pairing body unreadable: {error}"))?;
    let parsed: PairingResponse = serde_json::from_str(&body)
        .map_err(|error| format!("Pairing JSON invalid: {error}"))?;
    if let Ok(decoded) = decode_pairing_payload_url(&parsed.payload) {
        if is_loopback_public_url(&decoded) {
            return Err(
                "Pairing payload still has a loopback URL. Check VOCAGATEWAY_PUBLIC_URL.".into(),
            );
        }
    }
    let display_url = parsed.url.unwrap_or(url);
    if is_loopback_public_url(&display_url) {
        return Err(
            "Pairing response returned a loopback URL. Check VOCAGATEWAY_PUBLIC_URL.".into(),
        );
    }
    let qr_svg = fetch_qr_svg(&client, &token, &display_url).await;
    Ok(GatewayPairing {
        url: display_url,
        qr_svg,
    })
}

fn urlencoding_minimal(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn decode_pairing_payload_url(payload: &str) -> Result<String, String> {
    // Payload is base64url JSON {"v":1,"url":"...","token":"..."}. Avoid a base64
    // crate: only need to reject loopback when we can decode; otherwise skip.
    let normalized = payload.replace('-', "+").replace('_', "/");
    let padded = match normalized.len() % 4 {
        0 => normalized,
        2 => format!("{normalized}=="),
        3 => format!("{normalized}="),
        _ => return Err("bad padding".into()),
    };
    let bytes = decode_base64(&padded)?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    json.get("url")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| "missing url".into())
}

fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    // Minimal standard base64 decoder for pairing payload checks.
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut i = 0;
    while i + 4 <= bytes.len() {
        let a = if bytes[i] == b'=' {
            0
        } else {
            val(bytes[i]).ok_or("bad base64")?
        };
        let b = if bytes[i + 1] == b'=' {
            0
        } else {
            val(bytes[i + 1]).ok_or("bad base64")?
        };
        let c = if bytes[i + 2] == b'=' {
            0
        } else {
            val(bytes[i + 2]).ok_or("bad base64")?
        };
        let d = if bytes[i + 3] == b'=' {
            0
        } else {
            val(bytes[i + 3]).ok_or("bad base64")?
        };
        out.push((a << 2) | (b >> 4));
        if bytes[i + 2] != b'=' {
            out.push((b << 4) | (c >> 2));
        }
        if bytes[i + 3] != b'=' {
            out.push((c << 6) | d);
        }
        i += 4;
    }
    Ok(out)
}

async fn fetch_qr_svg(client: &reqwest::Client, token: &str, public_url: &str) -> Option<String> {
    let endpoint = format!(
        "{LOCAL_BASE_URL}/v1/admin/pairing/qr.svg?url={}",
        urlencoding_minimal(public_url)
    );
    let response = client
        .get(&endpoint)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let text = response.text().await.ok()?;
    if text.contains("<svg") {
        Some(text)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_loopback_hosts() {
        assert!(is_loopback_public_url("http://127.0.0.1:8765"));
        assert!(is_loopback_public_url("http://localhost:8765"));
        assert!(is_loopback_public_url("https://127.0.0.1/"));
        assert!(is_loopback_public_url("http://127.1.2.3:8765"));
        assert!(is_loopback_public_url("http://[::1]:8765"));
        assert!(!is_loopback_public_url("http://192.168.1.20:8765"));
        assert!(!is_loopback_public_url("http://100.64.1.2:8765"));
        assert!(!is_loopback_public_url(""));
    }

    #[test]
    fn validate_public_url_requires_scheme_and_non_loopback() {
        assert!(validate_public_url("192.168.1.20:8765").is_err());
        assert!(validate_public_url("http://127.0.0.1:8765").is_err());
        assert!(validate_public_url("http://192.168.1.20:8765\nVOCAGATEWAY_TOKEN=ab").is_err());
        assert!(validate_public_url("http://192.168.1.20:8765#frag").is_err());
        assert!(validate_public_url("http://192.168.1.20@127.0.0.1:8765").is_err());
        assert_eq!(
            validate_public_url("http://192.168.1.20:8765/").unwrap(),
            "http://192.168.1.20:8765"
        );
    }

    #[test]
    fn token_is_at_least_32_hex() {
        let token = generate_token_hex().expect("entropy");
        assert!(token.len() >= 32);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn phase_mapping_covers_contract_states() {
        assert_eq!(
            map_phase(false, false, false, false),
            GatewayPhase::DockerMissing
        );
        assert_eq!(map_phase(true, false, false, false), GatewayPhase::Stopped);
        assert_eq!(
            map_phase(true, true, false, false),
            GatewayPhase::Starting
        );
        assert_eq!(
            map_phase(true, true, true, false),
            GatewayPhase::LiveNotReady
        );
        assert_eq!(map_phase(true, true, true, true), GatewayPhase::Ready);
        assert_eq!(map_phase(true, false, true, true), GatewayPhase::Ready);
    }

    #[test]
    fn parse_env_reads_token() {
        let sample = "VOCAGATEWAY_PUBLISH_PORT=8765\nVOCAGATEWAY_TOKEN=abcd\n";
        assert_eq!(
            parse_env_value(sample, "VOCAGATEWAY_TOKEN").as_deref(),
            Some("abcd")
        );
    }

    #[test]
    fn compose_yaml_pins_release_image() {
        assert!(COMPOSE_YAML.contains(GATEWAY_IMAGE) || COMPOSE_YAML.contains("v0.1.0"));
        assert!(!COMPOSE_YAML.contains(":latest"));
        assert!(COMPOSE_YAML.contains("service") || COMPOSE_YAML.contains("gateway:"));
    }
}
