//! Servidores de acceso rápido: lista propia (`servers.json`), ping con el
//! protocolo Server List Ping de Java Edition e importación de `servers.dat`.
//!
//! El ping no usa autenticación ni cuentas: es el mismo intercambio que hace
//! la pantalla de multijugador (handshake, petición de estado, ping/pong).
//! Si la dirección no lleva puerto se consulta el SRV `_minecraft._tcp.<host>`
//! y, si no existe, se usa el puerto 25565.
//!
//! No se implementa el ping heredado de 1.6 (`0xFE 0x01`): lo hablan solo
//! servidores anteriores a 1.7.

use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex as StdMutex};
use std::time::{Duration, Instant};

use hickory_resolver::TokioResolver;
use hickory_resolver::proto::rr::RData;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::timeout_at;

use crate::fsutil::{read_json, write_json};
use crate::launcher::Core;
use crate::{Error, Result};

pub const SERVERS_SCHEMA: u32 = 1;
/// Cuánto vale un estado en la caché de pings.
pub const PING_CACHE_TTL: Duration = Duration::from_secs(30);
/// Tamaño máximo de la respuesta de estado.
pub const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_PORT: u16 = 25565;
const PING_CONCURRENCY: usize = 8;
const SRV_TIMEOUT: Duration = Duration::from_secs(4);
/// Espera máxima por el pong una vez recibido el estado.
const PONG_GRACE: Duration = Duration::from_secs(2);
/// Tamaño máximo de `servers.dat` que se acepta.
const MAX_SERVERS_DAT: u64 = 8 * 1024 * 1024;
const FAVICON_PREFIX: &str = "data:image/png;base64,";

// ---------------------------------------------------------------------------
// Lista de servidores
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Server {
    pub id: String,
    pub name: String,
    /// `host` o `host:puerto` tal como lo escribió el usuario.
    pub address: String,
    /// Instancia con la que se entra a este servidor.
    #[serde(default)]
    pub instance_id: Option<String>,
    /// Cuenta con la que se entra; `None` = la activa.
    #[serde(default)]
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServersFile {
    schema: u32,
    servers: Vec<Server>,
}

impl Default for ServersFile {
    fn default() -> Self {
        Self {
            schema: SERVERS_SCHEMA,
            servers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NewServer {
    /// Vacío = la dirección.
    pub name: String,
    pub address: String,
    pub instance_id: Option<String>,
    pub account_id: Option<String>,
}

/// Cambios sobre un servidor; `None` = no tocar. En los `Option<Option<_>>`,
/// `Some(None)` borra el valor.
#[derive(Debug, Clone, Default)]
pub struct ServerPatch {
    pub name: Option<String>,
    pub address: Option<String>,
    pub instance_id: Option<Option<String>>,
    pub account_id: Option<Option<String>>,
}

/// Serializa las lecturas-modificaciones de `servers.json`.
static SERVERS_LOCK: Mutex<()> = Mutex::const_new(());

fn not_found(id: &str) -> Error {
    Error::InvalidInput(format!("no existe el servidor {id}"))
}

impl Core {
    pub fn servers_file(&self) -> PathBuf {
        self.paths().root().join("servers.json")
    }

    async fn load_servers(&self) -> Result<ServersFile> {
        Ok(read_json(&self.servers_file()).await?.unwrap_or_default())
    }

    async fn save_servers(&self, file: &ServersFile) -> Result<()> {
        write_json(&self.servers_file(), file).await
    }

    pub async fn list_servers(&self) -> Result<Vec<Server>> {
        Ok(self.load_servers().await?.servers)
    }

    pub async fn add_server(&self, new: NewServer) -> Result<Server> {
        let address = new.address.trim().to_owned();
        split_address(&address)?;
        let name = new.name.trim();
        let server = Server {
            id: uuid::Uuid::new_v4().to_string(),
            name: if name.is_empty() { &address } else { name }.to_owned(),
            address,
            instance_id: new.instance_id.filter(|s| !s.is_empty()),
            account_id: new.account_id.filter(|s| !s.is_empty()),
        };
        let _guard = SERVERS_LOCK.lock().await;
        let mut file = self.load_servers().await?;
        file.servers.push(server.clone());
        self.save_servers(&file).await?;
        Ok(server)
    }

    pub async fn update_server(&self, id: &str, patch: ServerPatch) -> Result<Server> {
        if let Some(address) = &patch.address {
            split_address(address.trim())?;
        }
        let _guard = SERVERS_LOCK.lock().await;
        let mut file = self.load_servers().await?;
        let server = file
            .servers
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| not_found(id))?;
        if let Some(address) = patch.address {
            server.address = address.trim().to_owned();
        }
        if let Some(name) = patch.name {
            let name = name.trim();
            server.name = if name.is_empty() {
                server.address.clone()
            } else {
                name.to_owned()
            };
        }
        if let Some(instance) = patch.instance_id {
            server.instance_id = instance.filter(|s| !s.is_empty());
        }
        if let Some(account) = patch.account_id {
            server.account_id = account.filter(|s| !s.is_empty());
        }
        let updated = server.clone();
        self.save_servers(&file).await?;
        Ok(updated)
    }

    /// Quita un servidor de la lista y lo devuelve.
    pub async fn remove_server(&self, id: &str) -> Result<Server> {
        let _guard = SERVERS_LOCK.lock().await;
        let mut file = self.load_servers().await?;
        let pos = file
            .servers
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| not_found(id))?;
        let removed = file.servers.remove(pos);
        self.save_servers(&file).await?;
        Ok(removed)
    }

    /// Mueve un servidor a la posición `new_index` (se ajusta al final de la lista).
    pub async fn move_server(&self, id: &str, new_index: usize) -> Result<Vec<Server>> {
        let _guard = SERVERS_LOCK.lock().await;
        let mut file = self.load_servers().await?;
        let pos = file
            .servers
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| not_found(id))?;
        let server = file.servers.remove(pos);
        let index = new_index.min(file.servers.len());
        file.servers.insert(index, server);
        self.save_servers(&file).await?;
        Ok(file.servers)
    }

    /// Añade a la lista los servidores de `<juego>/servers.dat` de la instancia
    /// que no estén ya (por dirección). Los ocultos se ignoran. Devuelve
    /// cuántos se añadieron.
    pub async fn import_servers_dat(&self, instance_id: &str) -> Result<usize> {
        let instance = self.get_instance(instance_id).await?;
        let path = self.instance_game_dir(&instance.id)?.join("servers.dat");
        let found = read_servers_dat(&path).await?;
        if found.is_empty() {
            return Ok(0);
        }
        let _guard = SERVERS_LOCK.lock().await;
        let mut file = self.load_servers().await?;
        let mut added = 0;
        for (name, address) in found {
            let key = address.to_lowercase();
            if file
                .servers
                .iter()
                .any(|s| s.address.trim().to_lowercase() == key)
            {
                continue;
            }
            file.servers.push(Server {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                address,
                instance_id: Some(instance.id.clone()),
                account_id: None,
            });
            added += 1;
        }
        if added > 0 {
            self.save_servers(&file).await?;
        }
        Ok(added)
    }
}

// ---------------------------------------------------------------------------
// servers.dat
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
struct DatFile {
    #[serde(default)]
    servers: Vec<DatServer>,
}

#[derive(Debug, Default, Deserialize)]
struct DatServer {
    #[serde(default)]
    name: String,
    #[serde(default)]
    ip: String,
    #[serde(default)]
    hidden: bool,
}

/// `(nombre, dirección)` de los servidores visibles del archivo. Un archivo
/// que no existe da una lista vacía.
async fn read_servers_dat(path: &std::path::Path) -> Result<Vec<(String, String)>> {
    let bytes = match tokio::fs::metadata(path).await {
        Ok(m) if m.len() > MAX_SERVERS_DAT => {
            return Err(Error::InvalidInput(format!(
                "{} es demasiado grande",
                path.display()
            )));
        }
        Ok(_) => tokio::fs::read(path)
            .await
            .map_err(|e| Error::io(path, e))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(path, e)),
    };
    parse_servers_dat(&bytes).map_err(|e| {
        Error::InvalidInput(format!(
            "{} no es un servers.dat válido: {e}",
            path.display()
        ))
    })
}

fn parse_servers_dat(bytes: &[u8]) -> std::result::Result<Vec<(String, String)>, String> {
    // Lo normal es NBT sin comprimir; por si acaso, se admite gzip.
    let unzipped;
    let bytes = if bytes.starts_with(&[0x1f, 0x8b]) {
        use std::io::Read;
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(bytes)
            .take(MAX_SERVERS_DAT)
            .read_to_end(&mut out)
            .map_err(|e| e.to_string())?;
        unzipped = out;
        &unzipped[..]
    } else {
        bytes
    };
    let file: DatFile = fastnbt::from_bytes(bytes).map_err(|e| e.to_string())?;
    Ok(file
        .servers
        .into_iter()
        .filter(|s| !s.hidden)
        .filter_map(|s| {
            let ip = s.ip.trim().to_owned();
            split_address(&ip).ok()?;
            let name = s.name.trim();
            let name = if name.is_empty() {
                ip.clone()
            } else {
                name.to_owned()
            };
            Some((name, ip))
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Direcciones
// ---------------------------------------------------------------------------

/// `host[:puerto]` -> `(host, puerto)`. El puerto es `None` si no se escribió.
/// IPv6 va entre corchetes cuando lleva puerto (`[::1]:25565`).
pub fn split_address(address: &str) -> Result<(String, Option<u16>)> {
    let s = address.trim();
    let bad = || Error::InvalidInput(format!("dirección de servidor no válida: {s:?}"));
    if s.is_empty() || s.contains(char::is_whitespace) || s.contains('/') {
        return Err(bad());
    }
    let parse_port = |p: &str| p.parse::<u16>().ok().filter(|p| *p != 0).ok_or_else(bad);
    let (host, port) = if let Some(rest) = s.strip_prefix('[') {
        let (host, tail) = rest.split_once(']').ok_or_else(bad)?;
        match tail {
            "" => (host, None),
            t => (
                host,
                Some(parse_port(t.strip_prefix(':').ok_or_else(bad)?)?),
            ),
        }
    } else {
        match s.rsplit_once(':') {
            Some((host, port)) if !host.contains(':') => (host, Some(parse_port(port)?)),
            _ => (s, None),
        }
    };
    if host.is_empty() {
        return Err(bad());
    }
    Ok((host.to_owned(), port))
}

fn display_endpoint(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

// ---------------------------------------------------------------------------
// Protocolo: VarInt y paquetes
// ---------------------------------------------------------------------------

/// Codifica un VarInt (hasta 5 bytes; los negativos ocupan 5).
pub fn encode_varint(value: i32) -> Vec<u8> {
    let mut v = value as u32;
    let mut out = Vec::with_capacity(5);
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return out;
        }
        out.push(byte | 0x80);
    }
}

/// Decodifica un VarInt del principio de `buf`: `(valor, bytes usados)`.
/// `None` si está incompleto o pasa de 5 bytes.
pub fn decode_varint(buf: &[u8]) -> Option<(i32, usize)> {
    let mut value: u32 = 0;
    for (i, byte) in buf.iter().take(5).enumerate() {
        value |= u32::from(byte & 0x7f) << (7 * i);
        if byte & 0x80 == 0 {
            // El quinto byte solo puede aportar 4 bits.
            if i == 4 && byte & 0xf0 != 0 {
                return None;
            }
            return Some((value as i32, i + 1));
        }
    }
    None
}

async fn read_varint<R: AsyncRead + Unpin>(r: &mut R) -> std::io::Result<i32> {
    let mut buf = [0u8; 5];
    for i in 0..5 {
        buf[i] = r.read_u8().await?;
        if buf[i] & 0x80 == 0 {
            return decode_varint(&buf[..=i])
                .map(|(v, _)| v)
                .ok_or_else(invalid_varint);
        }
    }
    Err(invalid_varint())
}

fn invalid_varint() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, "VarInt no válido")
}

/// Antepone la longitud al contenido (id + datos).
fn frame(body: &[u8]) -> Vec<u8> {
    let mut out = encode_varint(body.len() as i32);
    out.extend_from_slice(body);
    out
}

fn handshake_packet(protocol: i32, host: &str, port: u16) -> Vec<u8> {
    let mut body = vec![0x00];
    body.extend(encode_varint(protocol));
    body.extend(encode_varint(host.len() as i32));
    body.extend_from_slice(host.as_bytes());
    body.extend_from_slice(&port.to_be_bytes());
    body.extend(encode_varint(1)); // siguiente estado: status
    frame(&body)
}

/// Lee un paquete: `(id, datos)`. Rechaza longitudes fuera de `1..=max`.
async fn read_packet<R: AsyncRead + Unpin>(
    r: &mut R,
    max: usize,
) -> std::result::Result<(i32, Vec<u8>), String> {
    let len = read_varint(r).await.map_err(|e| format!("lectura: {e}"))?;
    if len <= 0 {
        return Err("longitud de paquete no válida".into());
    }
    if len as usize > max {
        return Err(format!(
            "respuesta demasiado grande ({len} bytes; máximo {max})"
        ));
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf)
        .await
        .map_err(|e| format!("lectura: {e}"))?;
    let (id, used) = decode_varint(&buf).ok_or("id de paquete no válido")?;
    buf.drain(..used);
    Ok((id, buf))
}

// ---------------------------------------------------------------------------
// Estado del servidor
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ServerStatus {
    /// Dirección tal como se pidió.
    pub address: String,
    pub online: bool,
    pub latency_ms: u64,
    pub version_name: String,
    pub protocol: i32,
    pub players_online: u32,
    pub players_max: u32,
    pub sample: Vec<String>,
    pub motd_plain: String,
    pub favicon_png_base64: Option<String>,
    /// `host:puerto` al que se conectó (tras resolver el SRV).
    pub resolved: String,
    /// Motivo si no está en línea.
    pub error: Option<String>,
}

impl ServerStatus {
    fn failed(address: &str, resolved: String, error: String) -> Self {
        Self {
            address: address.to_owned(),
            online: false,
            latency_ms: 0,
            version_name: String::new(),
            protocol: 0,
            players_online: 0,
            players_max: 0,
            sample: Vec::new(),
            motd_plain: String::new(),
            favicon_png_base64: None,
            resolved,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PingOptions {
    pub connect_timeout: Duration,
    /// Tiempo total para el intercambio una vez conectado.
    pub io_timeout: Duration,
    pub max_response: usize,
    /// Versión de protocolo del handshake; -1 = "no importa".
    pub protocol: i32,
}

impl Default for PingOptions {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            io_timeout: Duration::from_secs(5),
            max_response: MAX_RESPONSE_BYTES,
            protocol: -1,
        }
    }
}

type CacheMap = HashMap<String, (Instant, ServerStatus)>;
static PING_CACHE: LazyLock<StdMutex<CacheMap>> = LazyLock::new(Default::default);

fn cache_key(address: &str) -> String {
    address.trim().to_lowercase()
}

fn cache_get(key: &str, ttl: Duration) -> Option<ServerStatus> {
    let cache = PING_CACHE.lock().ok()?;
    let (at, status) = cache.get(key)?;
    (at.elapsed() < ttl).then(|| status.clone())
}

fn cache_put(key: String, status: &ServerStatus) {
    if let Ok(mut cache) = PING_CACHE.lock() {
        cache.retain(|_, (at, _)| at.elapsed() < PING_CACHE_TTL);
        cache.insert(key, (Instant::now(), status.clone()));
    }
}

/// Vacía la caché de pings.
pub fn clear_ping_cache() {
    if let Ok(mut cache) = PING_CACHE.lock() {
        cache.clear();
    }
}

/// Consulta el estado de un servidor, con caché de 30 s. Un servidor caído da
/// `online: false` con el motivo en `error`; solo una dirección mal escrita es
/// un error.
pub async fn ping_server(address: &str) -> Result<ServerStatus> {
    split_address(address)?;
    let key = cache_key(address);
    if let Some(hit) = cache_get(&key, PING_CACHE_TTL) {
        return Ok(hit);
    }
    let status = ping_server_with(address, &PingOptions::default()).await?;
    cache_put(key, &status);
    Ok(status)
}

/// Como [`ping_server`], sin caché y con opciones.
pub async fn ping_server_with(address: &str, opts: &PingOptions) -> Result<ServerStatus> {
    let (host, port) = split_address(address)?;
    let (target_host, target_port) = resolve_endpoint(&host, port).await;
    let resolved = display_endpoint(&target_host, target_port);
    let started = Instant::now();
    let connect = tokio::time::timeout(
        opts.connect_timeout,
        TcpStream::connect((target_host.as_str(), target_port)),
    )
    .await;
    let mut stream = match connect {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            return Ok(ServerStatus::failed(
                address,
                resolved,
                format!("no se pudo conectar: {e}"),
            ));
        }
        Err(_) => {
            return Ok(ServerStatus::failed(
                address,
                resolved,
                format!(
                    "tiempo de conexión agotado ({} s)",
                    opts.connect_timeout.as_secs_f32()
                ),
            ));
        }
    };
    let _ = stream.set_nodelay(true);
    let connect_ms = started.elapsed().as_millis() as u64;

    // El handshake lleva el host que escribió el usuario y el puerto real.
    match exchange(&mut stream, &host, target_port, opts).await {
        Ok((json, latency)) => {
            let mut status = status_from_json(&json, address, resolved);
            status.latency_ms = latency.map_or(connect_ms, |d| d.as_millis() as u64);
            Ok(status)
        }
        Err(e) => Ok(ServerStatus::failed(address, resolved, e)),
    }
}

/// Handshake, petición de estado y ping. Devuelve el JSON y la latencia del
/// ping (`None` si el servidor no contestó al ping).
async fn exchange(
    stream: &mut TcpStream,
    host: &str,
    port: u16,
    opts: &PingOptions,
) -> std::result::Result<(Value, Option<Duration>), String> {
    let deadline = tokio::time::Instant::now() + opts.io_timeout;
    let timed_out = || format!("sin respuesta en {} s", opts.io_timeout.as_secs_f32());

    let mut request = handshake_packet(opts.protocol, host, port);
    request.extend(frame(&[0x00])); // status request
    stream
        .write_all(&request)
        .await
        .map_err(|e| format!("escritura: {e}"))?;

    let (id, data) = timeout_at(deadline, read_packet(stream, opts.max_response))
        .await
        .map_err(|_| timed_out())??;
    if id != 0x00 {
        return Err(format!("paquete inesperado 0x{id:02x}"));
    }
    let json = parse_status_payload(&data)?;

    // Ping/pong: si falla, el estado ya está y se sigue sin esta latencia.
    let token = fastrand::i64(..);
    let mut ping = vec![0x01];
    ping.extend_from_slice(&token.to_be_bytes());
    let pong_deadline = deadline.min(tokio::time::Instant::now() + PONG_GRACE);
    let sent = Instant::now();
    let pong = async {
        stream.write_all(&frame(&ping)).await.ok()?;
        let (id, data) = read_packet(stream, 64).await.ok()?;
        (id == 0x01 && data.len() == 8).then(|| sent.elapsed())
    };
    let latency = timeout_at(pong_deadline, pong).await.ok().flatten();
    Ok((json, latency))
}

/// Datos de la respuesta de estado: un string con longitud VarInt.
fn parse_status_payload(data: &[u8]) -> std::result::Result<Value, String> {
    let (len, used) = decode_varint(data).ok_or("respuesta de estado no válida")?;
    let body = data
        .get(used..)
        .filter(|b| len >= 0 && b.len() >= len as usize)
        .ok_or("respuesta de estado truncada")?;
    let text = std::str::from_utf8(&body[..len as usize]).map_err(|_| "respuesta no es UTF-8")?;
    let json: Value = serde_json::from_str(text).map_err(|e| format!("JSON no válido: {e}"))?;
    if !json.is_object() {
        return Err("la respuesta de estado no es un objeto".into());
    }
    Ok(json)
}

fn status_from_json(json: &Value, address: &str, resolved: String) -> ServerStatus {
    let count = |v: Option<&Value>| {
        v.and_then(Value::as_u64)
            .map_or(0, |n| u32::try_from(n).unwrap_or(u32::MAX))
    };
    let players = json.get("players");
    let sample = players
        .and_then(|p| p.get("sample"))
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|p| p.get("name").and_then(Value::as_str))
                .map(strip_legacy_codes)
                .collect()
        })
        .unwrap_or_default();
    let version = json.get("version");
    ServerStatus {
        address: address.to_owned(),
        online: true,
        latency_ms: 0,
        version_name: version
            .and_then(|v| v.get("name"))
            .and_then(Value::as_str)
            .map(strip_legacy_codes)
            .unwrap_or_default(),
        protocol: version
            .and_then(|v| v.get("protocol"))
            .and_then(Value::as_i64)
            .and_then(|p| i32::try_from(p).ok())
            .unwrap_or(0),
        players_online: count(players.and_then(|p| p.get("online"))),
        players_max: count(players.and_then(|p| p.get("max"))),
        sample,
        motd_plain: json
            .get("description")
            .map(chat_to_plain)
            .unwrap_or_default(),
        favicon_png_base64: json
            .get("favicon")
            .and_then(Value::as_str)
            .and_then(parse_favicon),
        resolved,
        error: None,
    }
}

/// `data:image/png;base64,XXXX` -> `XXXX` (sin saltos de línea).
fn parse_favicon(s: &str) -> Option<String> {
    let b64: String = s
        .strip_prefix(FAVICON_PREFIX)?
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    (!b64.is_empty()
        && b64
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=')))
    .then_some(b64)
}

// ---------------------------------------------------------------------------
// Texto de chat
// ---------------------------------------------------------------------------

/// Quita los códigos de formato `§x` (y `§x§r§r§g§g§b§b`).
pub fn strip_legacy_codes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

/// Componente de chat (cadena, objeto con `text`/`extra`, lista) -> texto plano.
pub fn chat_to_plain(component: &Value) -> String {
    let mut raw = String::new();
    push_chat(component, &mut raw, 0);
    strip_legacy_codes(&raw)
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

fn push_chat(v: &Value, out: &mut String, depth: u32) {
    if depth > 32 {
        return;
    }
    match v {
        Value::String(s) => out.push_str(s),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::Bool(b) => out.push_str(&b.to_string()),
        Value::Array(list) => list.iter().for_each(|c| push_chat(c, out, depth + 1)),
        Value::Object(map) => {
            match (map.get("text"), map.get("translate")) {
                (Some(text), _) => push_chat(text, out, depth + 1),
                (None, Some(Value::String(key))) => out.push_str(key),
                _ => {}
            }
            if let Some(extra) = map.get("extra") {
                push_chat(extra, out, depth + 1);
            }
        }
        Value::Null => {}
    }
}

// ---------------------------------------------------------------------------
// Resolución (SRV)
// ---------------------------------------------------------------------------

/// Sin puerto: SRV `_minecraft._tcp.<host>`; si no hay, `host:25565`.
async fn resolve_endpoint(host: &str, port: Option<u16>) -> (String, u16) {
    if let Some(port) = port {
        return (host.to_owned(), port);
    }
    if host.parse::<IpAddr>().is_err()
        && let Ok(Some(found)) = tokio::time::timeout(SRV_TIMEOUT, lookup_srv(host)).await
    {
        return found;
    }
    (host.to_owned(), DEFAULT_PORT)
}

async fn lookup_srv(host: &str) -> Option<(String, u16)> {
    let resolver = TokioResolver::builder_tokio().ok()?.build().ok()?;
    srv_with(&resolver, host).await
}

async fn srv_with(resolver: &TokioResolver, host: &str) -> Option<(String, u16)> {
    // El punto final evita que se aplique la lista de dominios de búsqueda.
    let lookup = resolver
        .srv_lookup(format!("_minecraft._tcp.{}.", host.trim_end_matches('.')))
        .await
        .ok()?;
    let records = lookup.answers().iter().filter_map(|r| match &r.data {
        RData::SRV(srv) => Some(SrvRecord {
            priority: srv.priority,
            weight: srv.weight,
            port: srv.port,
            target: srv.target.to_utf8(),
        }),
        _ => None,
    });
    pick_srv(records.collect())
}

#[derive(Debug, Clone, PartialEq)]
struct SrvRecord {
    priority: u16,
    weight: u16,
    port: u16,
    target: String,
}

/// Menor prioridad primero y, a igualdad, mayor peso. Descarta el destino `.`
/// (servicio no disponible, RFC 2782) y el puerto 0.
fn pick_srv(mut records: Vec<SrvRecord>) -> Option<(String, u16)> {
    records.retain(|r| r.port != 0 && !r.target.trim_end_matches('.').is_empty());
    records.sort_by(|a, b| a.priority.cmp(&b.priority).then(b.weight.cmp(&a.weight)));
    let best = records.into_iter().next()?;
    Some((best.target.trim_end_matches('.').to_owned(), best.port))
}

// ---------------------------------------------------------------------------
// Varios servidores
// ---------------------------------------------------------------------------

/// Consulta varias direcciones a la vez (hasta 8 en paralelo). El resultado
/// conserva el orden de entrada; una dirección mal escrita da `online: false`.
pub async fn ping_servers(addresses: &[String]) -> Vec<ServerStatus> {
    let gate = Arc::new(Semaphore::new(PING_CONCURRENCY));
    let mut tasks = tokio::task::JoinSet::new();
    for (index, address) in addresses.iter().cloned().enumerate() {
        let gate = gate.clone();
        tasks.spawn(async move {
            let _permit = gate.acquire_owned().await;
            let status = ping_server(&address)
                .await
                .unwrap_or_else(|e| ServerStatus::failed(&address, String::new(), e.to_string()));
            (index, status)
        });
    }
    let mut out: Vec<Option<ServerStatus>> = vec![None; addresses.len()];
    while let Some(joined) = tasks.join_next().await {
        if let Ok((index, status)) = joined {
            out[index] = Some(status);
        }
    }
    out.into_iter()
        .zip(addresses)
        .map(|(status, address)| {
            status.unwrap_or_else(|| {
                ServerStatus::failed(address, String::new(), "tarea interrumpida".into())
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use serde_json::json;
    use tokio::net::TcpListener;

    use super::*;
    use crate::Options;
    use crate::instance::{GameVersion, LoaderKind, NewInstance};
    use crate::secrets::MemoryStore;

    // -- VarInt -------------------------------------------------------------

    #[test]
    fn varint_vectores_conocidos() {
        let cases: &[(i32, &[u8])] = &[
            (0, &[0x00]),
            (1, &[0x01]),
            (127, &[0x7f]),
            (128, &[0x80, 0x01]),
            (255, &[0xff, 0x01]),
            (25565, &[0xdd, 0xc7, 0x01]),
            (2_097_151, &[0xff, 0xff, 0x7f]),
            (i32::MAX, &[0xff, 0xff, 0xff, 0xff, 0x07]),
            (-1, &[0xff, 0xff, 0xff, 0xff, 0x0f]),
            (i32::MIN, &[0x80, 0x80, 0x80, 0x80, 0x08]),
        ];
        for (value, bytes) in cases {
            assert_eq!(&encode_varint(*value), bytes, "codificar {value}");
            assert_eq!(decode_varint(bytes), Some((*value, bytes.len())), "{value}");
        }
    }

    #[test]
    fn varint_invalidos() {
        assert_eq!(decode_varint(&[]), None);
        assert_eq!(decode_varint(&[0x80]), None); // incompleto
        assert_eq!(decode_varint(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x01]), None); // 6 bytes
        assert_eq!(decode_varint(&[0xff, 0xff, 0xff, 0xff, 0x7f]), None); // desborda 32 bits
        // Con datos detrás solo consume lo suyo.
        assert_eq!(decode_varint(&[0x05, 0xaa, 0xbb]), Some((5, 1)));
    }

    #[tokio::test]
    async fn varint_asincrono() {
        for v in [0, 1, 300, -1, i32::MAX, i32::MIN] {
            let bytes = encode_varint(v);
            assert_eq!(read_varint(&mut &bytes[..]).await.unwrap(), v);
        }
        let too_long = [0x80u8; 6];
        assert!(read_varint(&mut &too_long[..]).await.is_err());
        assert!(read_varint(&mut &[0x80u8][..]).await.is_err()); // EOF
    }

    // -- Texto --------------------------------------------------------------

    #[test]
    fn chat_a_texto_plano() {
        assert_eq!(chat_to_plain(&json!("§aHola §lmundo§r")), "Hola mundo");
        assert_eq!(
            chat_to_plain(&json!({"text": "A", "color": "red", "extra": [
                {"text": "B", "bold": true},
                "C",
                {"text": "D", "extra": [{"text": "E"}]}
            ]})),
            "ABCDE"
        );
        assert_eq!(
            chat_to_plain(&json!([{"text": "uno "}, {"text": "dos"}])),
            "uno dos"
        );
        // Colores hexadecimales heredados y líneas con relleno.
        assert_eq!(
            chat_to_plain(&json!({"text": "    §x§f§f§0§0§0§0Servidor\n   §7línea 2   "})),
            "Servidor\nlínea 2"
        );
        assert_eq!(
            chat_to_plain(&json!({"translate": "chat.hola"})),
            "chat.hola"
        );
        assert_eq!(chat_to_plain(&json!(null)), "");
        assert_eq!(chat_to_plain(&json!({"text": 7})), "7");
        // Anidamiento absurdo: no desborda la pila.
        let mut deep = json!({"text": "fin"});
        for _ in 0..500 {
            deep = json!({"text": "", "extra": [deep]});
        }
        assert_eq!(chat_to_plain(&deep), "");
        // § final sin carácter detrás.
        assert_eq!(strip_legacy_codes("ab§"), "ab");
    }

    #[test]
    fn direcciones() {
        let ok = |s: &str| split_address(s).unwrap();
        assert_eq!(ok("mc.hypixel.net"), ("mc.hypixel.net".into(), None));
        assert_eq!(ok(" mc.org:1234 "), ("mc.org".into(), Some(1234)));
        assert_eq!(ok("[::1]:25566"), ("::1".into(), Some(25566)));
        assert_eq!(ok("[::1]"), ("::1".into(), None));
        assert_eq!(ok("::1"), ("::1".into(), None));
        assert_eq!(ok("127.0.0.1:1"), ("127.0.0.1".into(), Some(1)));
        for bad in [
            "",
            "  ",
            "a b",
            ":25565",
            "host:",
            "host:0",
            "host:70000",
            "host:x",
            "a/b",
            "[::1",
        ] {
            assert!(split_address(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn srv_elige_prioridad_y_peso() {
        let rec = |priority, weight, port, target: &str| SrvRecord {
            priority,
            weight,
            port,
            target: target.into(),
        };
        let picked = pick_srv(vec![
            rec(10, 5, 1, "b.example.com."),
            rec(5, 1, 2, "c.example.com."),
            rec(5, 9, 3, "a.example.com."),
        ]);
        assert_eq!(picked, Some(("a.example.com".into(), 3)));
        assert_eq!(pick_srv(vec![rec(0, 0, 25565, ".")]), None);
        assert_eq!(pick_srv(vec![rec(0, 0, 0, "x.com.")]), None);
        assert_eq!(pick_srv(vec![]), None);
    }

    // -- SRV contra un DNS falso -----------------------------------------------

    fn dns_name(name: &str) -> Vec<u8> {
        let mut out = Vec::new();
        for label in name.trim_end_matches('.').split('.') {
            out.push(label.len() as u8);
            out.extend_from_slice(label.as_bytes());
        }
        out.push(0);
        out
    }

    fn dns_srv(priority: u16, weight: u16, port: u16, target: &str) -> Vec<u8> {
        let mut rdata = Vec::new();
        for n in [priority, weight, port] {
            rdata.extend_from_slice(&n.to_be_bytes());
        }
        rdata.extend(dns_name(target));
        // Nombre por puntero al de la pregunta, tipo SRV, clase IN, TTL 60.
        let mut rr = vec![0xc0, 0x0c, 0x00, 33, 0x00, 0x01, 0, 0, 0, 60];
        rr.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
        rr.extend(rdata);
        rr
    }

    /// DNS UDP que contesta SRV (o NXDOMAIN para nombres con `missing`).
    async fn fake_dns() -> std::net::SocketAddr {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();
        tokio::spawn(async move {
            let mut buf = [0u8; 512];
            while let Ok((n, peer)) = socket.recv_from(&mut buf).await {
                let query = &buf[..n];
                let end = query[12..].iter().position(|b| *b == 0).unwrap() + 12 + 1 + 4;
                let qname = String::from_utf8_lossy(&query[13..end - 5]).into_owned();
                let mut reply = query[..end].to_vec();
                reply[2..4].copy_from_slice(&[0x81, 0x80]);
                reply[6..12].copy_from_slice(&[0; 6]);
                if qname.contains("missing") {
                    reply[3] = 0x83; // NXDOMAIN
                } else {
                    reply[7] = 2; // dos respuestas
                    reply.extend(dns_srv(20, 1, 1, "backup.example.com"));
                    reply.extend(dns_srv(10, 5, 25570, "mc-eu.example.com"));
                }
                let _ = socket.send_to(&reply, peer).await;
            }
        });
        addr
    }

    #[tokio::test]
    async fn srv_con_dns_falso() {
        use hickory_resolver::config::{NameServerConfig, ResolverConfig};
        use hickory_resolver::net::runtime::TokioRuntimeProvider;

        let addr = fake_dns().await;
        let mut ns = NameServerConfig::udp(addr.ip());
        ns.connections[0].port = addr.port();
        let resolver = TokioResolver::builder_with_config(
            ResolverConfig::from_name_servers(vec![ns]),
            TokioRuntimeProvider::default(),
        )
        .build()
        .unwrap();
        assert_eq!(
            srv_with(&resolver, "play.example.com").await,
            Some(("mc-eu.example.com".to_owned(), 25570))
        );
        assert_eq!(srv_with(&resolver, "missing.example.com").await, None);
    }

    // -- Servidor SLP falso ---------------------------------------------------

    #[derive(Clone, Copy, PartialEq)]
    enum Mode {
        Normal,
        /// Anuncia un paquete de 3 MB.
        Oversized,
        /// Acepta y no dice nada.
        Silent,
        /// Responde al estado y cierra sin pong.
        NoPong,
    }

    type Handshake = (i32, String, u16, i32);

    struct Fake {
        address: String,
        connections: Arc<AtomicUsize>,
        handshake: Arc<StdMutex<Option<Handshake>>>,
    }

    fn status_json() -> Value {
        json!({
            "version": {"name": "Paper 1.21.1", "protocol": 767},
            "players": {
                "max": 100,
                "online": 3,
                "sample": [
                    {"name": "Alex", "id": "00000000-0000-0000-0000-000000000001"},
                    {"name": "§aSteve", "id": "00000000-0000-0000-0000-000000000002"}
                ]
            },
            "description": {"text": "", "extra": [
                {"text": "  Mi ", "color": "gold"},
                {"text": "servidor", "bold": true},
                {"text": "\n§7segunda línea"}
            ]},
            "favicon": "data:image/png;base64,iVBORw0KGgo=\nAAAA",
            "enforcesSecureChat": true
        })
    }

    async fn fake_server(mode: Mode, status: Value, pong_delay: Duration) -> Fake {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let connections = Arc::new(AtomicUsize::new(0));
        let handshake = Arc::new(StdMutex::new(None));
        let (conns, hs) = (connections.clone(), handshake.clone());
        let text = status.to_string();
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                conns.fetch_add(1, Ordering::SeqCst);
                let (hs, text) = (hs.clone(), text.clone());
                tokio::spawn(async move {
                    let _ = serve(&mut stream, mode, &text, pong_delay, &hs).await;
                });
            }
        });
        Fake {
            address,
            connections,
            handshake,
        }
    }

    async fn serve(
        s: &mut TcpStream,
        mode: Mode,
        text: &str,
        pong_delay: Duration,
        handshake: &StdMutex<Option<Handshake>>,
    ) -> std::result::Result<(), String> {
        if mode == Mode::Silent {
            tokio::time::sleep(Duration::from_secs(30)).await;
            return Ok(());
        }
        let (id, data) = read_packet(s, 4096).await?;
        assert_eq!(id, 0x00);
        let (protocol, mut at) = decode_varint(&data).ok_or("protocolo")?;
        let (len, used) = decode_varint(&data[at..]).ok_or("longitud")?;
        at += used;
        let host = String::from_utf8(data[at..at + len as usize].to_vec()).unwrap();
        at += len as usize;
        let port = u16::from_be_bytes([data[at], data[at + 1]]);
        let (state, _) = decode_varint(&data[at + 2..]).ok_or("estado")?;
        *handshake.lock().unwrap() = Some((protocol, host, port, state));

        let (id, data) = read_packet(s, 16).await?;
        assert_eq!((id, data.len()), (0x00, 0)); // status request

        if mode == Mode::Oversized {
            s.write_all(&encode_varint(3_000_000)).await.ok();
            s.write_all(&[0u8; 1024]).await.ok();
            tokio::time::sleep(Duration::from_secs(30)).await;
            return Ok(());
        }
        let mut body = vec![0x00];
        body.extend(encode_varint(text.len() as i32));
        body.extend_from_slice(text.as_bytes());
        s.write_all(&frame(&body))
            .await
            .map_err(|e| e.to_string())?;
        if mode == Mode::NoPong {
            return Ok(());
        }
        let (id, ping) = read_packet(s, 16).await?;
        assert_eq!(id, 0x01);
        tokio::time::sleep(pong_delay).await;
        let mut pong = vec![0x01];
        pong.extend(ping);
        s.write_all(&frame(&pong))
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn fast() -> PingOptions {
        PingOptions {
            connect_timeout: Duration::from_secs(2),
            io_timeout: Duration::from_millis(400),
            ..PingOptions::default()
        }
    }

    #[tokio::test]
    async fn ping_completo() {
        let fake = fake_server(Mode::Normal, status_json(), Duration::from_millis(60)).await;
        let status = ping_server_with(&fake.address, &PingOptions::default())
            .await
            .unwrap();
        assert!(status.online, "{:?}", status.error);
        assert_eq!(status.error, None);
        assert_eq!(status.address, fake.address);
        assert_eq!(status.resolved, fake.address);
        assert_eq!(status.version_name, "Paper 1.21.1");
        assert_eq!(status.protocol, 767);
        assert_eq!((status.players_online, status.players_max), (3, 100));
        assert_eq!(status.sample, vec!["Alex", "Steve"]);
        assert_eq!(status.motd_plain, "Mi servidor\nsegunda línea");
        assert_eq!(
            status.favicon_png_base64.as_deref(),
            Some("iVBORw0KGgo=AAAA")
        );
        assert!(
            (60..2000).contains(&status.latency_ms),
            "latencia {}",
            status.latency_ms
        );
        let (protocol, host, port, state) = fake.handshake.lock().unwrap().clone().unwrap();
        assert_eq!((protocol, host.as_str(), state), (-1, "127.0.0.1", 1));
        assert_eq!(port.to_string(), fake.address.rsplit(':').next().unwrap());
    }

    #[tokio::test]
    async fn ping_con_descripcion_de_texto_y_sin_extras() {
        let json = json!({"version": {"name": "1.8.9", "protocol": 47}, "description": "§bHola"});
        let fake = fake_server(Mode::Normal, json, Duration::ZERO).await;
        let status = ping_server_with(&fake.address, &fast()).await.unwrap();
        assert!(status.online);
        assert_eq!(status.motd_plain, "Hola");
        assert_eq!((status.players_online, status.players_max), (0, 0));
        assert!(status.sample.is_empty());
        assert_eq!(status.favicon_png_base64, None);
    }

    #[tokio::test]
    async fn rechaza_respuesta_enorme() {
        let fake = fake_server(Mode::Oversized, status_json(), Duration::ZERO).await;
        let status = ping_server_with(&fake.address, &fast()).await.unwrap();
        assert!(!status.online);
        assert!(
            status
                .error
                .as_deref()
                .unwrap()
                .contains("demasiado grande"),
            "{:?}",
            status.error
        );
    }

    #[tokio::test]
    async fn servidor_mudo_agota_el_tiempo() {
        let fake = fake_server(Mode::Silent, status_json(), Duration::ZERO).await;
        let started = Instant::now();
        let status = ping_server_with(&fake.address, &fast()).await.unwrap();
        assert!(!status.online);
        assert!(status.error.as_deref().unwrap().contains("sin respuesta"));
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[tokio::test]
    async fn sin_pong_usa_la_latencia_de_conexion() {
        let fake = fake_server(Mode::NoPong, status_json(), Duration::ZERO).await;
        let status = ping_server_with(&fake.address, &fast()).await.unwrap();
        assert!(status.online, "{:?}", status.error);
        assert!(status.latency_ms < 400);
    }

    #[tokio::test]
    async fn puerto_cerrado_y_json_roto() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        drop(listener);
        let status = ping_server_with(&address, &fast()).await.unwrap();
        assert!(!status.online);
        assert!(status.error.unwrap().contains("no se pudo conectar"));

        // JSON que no es un objeto.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let _ = read_packet(&mut s, 4096).await;
            let _ = read_packet(&mut s, 16).await;
            let text = "[1,2]";
            let mut body = vec![0x00];
            body.extend(encode_varint(text.len() as i32));
            body.extend_from_slice(text.as_bytes());
            s.write_all(&frame(&body)).await.unwrap();
            tokio::time::sleep(Duration::from_secs(5)).await;
        });
        let status = ping_server_with(&address, &fast()).await.unwrap();
        assert!(!status.online);
        assert!(status.error.unwrap().contains("no es un objeto"));

        assert!(
            ping_server_with("host con espacios", &fast())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn cache_de_30_segundos() {
        let fake = fake_server(Mode::Normal, status_json(), Duration::ZERO).await;
        let a = ping_server(&fake.address).await.unwrap();
        let b = ping_server(&fake.address.to_uppercase()).await.unwrap();
        assert!(a.online);
        assert_eq!(fake.connections.load(Ordering::SeqCst), 1);
        assert_eq!(a.version_name, b.version_name);
        // Caducada: no se usa.
        assert!(cache_get(&cache_key(&fake.address), Duration::ZERO).is_none());
        assert!(cache_get(&cache_key(&fake.address), PING_CACHE_TTL).is_some());
    }

    #[tokio::test]
    async fn varios_a_la_vez_en_orden() {
        let mut fakes = Vec::new();
        for _ in 0..12 {
            fakes.push(fake_server(Mode::Normal, status_json(), Duration::from_millis(20)).await);
        }
        let mut addresses: Vec<String> = fakes.iter().map(|f| f.address.clone()).collect();
        addresses.insert(3, "dirección mala".into());
        let started = Instant::now();
        let list = ping_servers(&addresses).await;
        assert_eq!(list.len(), 13);
        for (address, status) in addresses.iter().zip(&list) {
            assert_eq!(&status.address, address);
        }
        assert!(!list[3].online);
        assert!(list.iter().enumerate().all(|(i, s)| i == 3 || s.online));
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    // -- Lista ----------------------------------------------------------------

    async fn core(dir: &std::path::Path) -> Core {
        Core::open(Options {
            data_dir: Some(dir.to_path_buf()),
            secrets: Some(Arc::new(MemoryStore::default())),
            offline: true,
            ..Options::default()
        })
        .await
        .unwrap()
    }

    fn new(name: &str, address: &str) -> NewServer {
        NewServer {
            name: name.into(),
            address: address.into(),
            ..NewServer::default()
        }
    }

    #[tokio::test]
    async fn crud_de_servidores() {
        let dir = tempfile::tempdir().unwrap();
        let core = core(dir.path()).await;
        assert!(core.list_servers().await.unwrap().is_empty());

        let a = core
            .add_server(new("Uno", "uno.example.com"))
            .await
            .unwrap();
        let b = core
            .add_server(new("", " dos.example.com:1234 "))
            .await
            .unwrap();
        let c = core
            .add_server(NewServer {
                instance_id: Some("mi-mundo".into()),
                account_id: Some(String::new()),
                ..new("Tres", "[::1]:25570")
            })
            .await
            .unwrap();
        assert_eq!(b.name, "dos.example.com:1234");
        assert_eq!(b.address, "dos.example.com:1234");
        assert_eq!(c.instance_id.as_deref(), Some("mi-mundo"));
        assert_eq!(c.account_id, None);
        assert!(uuid::Uuid::parse_str(&a.id).is_ok());
        assert!(core.add_server(new("x", "mala dirección")).await.is_err());

        let ids = |l: &[Server]| l.iter().map(|s| s.id.clone()).collect::<Vec<_>>();
        let list = core.list_servers().await.unwrap();
        assert_eq!(ids(&list), vec![a.id.clone(), b.id.clone(), c.id.clone()]);

        // Persistencia con el esquema esperado.
        let raw: Value =
            serde_json::from_slice(&std::fs::read(core.servers_file()).unwrap()).unwrap();
        assert_eq!(raw["schema"], 1);
        assert_eq!(raw["servers"][2]["instance_id"], "mi-mundo");

        let updated = core
            .update_server(
                &a.id,
                ServerPatch {
                    name: Some("Uno bis".into()),
                    address: Some("nuevo.example.com".into()),
                    instance_id: Some(Some("otra".into())),
                    account_id: Some(Some("cuenta".into())),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            (updated.name.as_str(), updated.address.as_str()),
            ("Uno bis", "nuevo.example.com")
        );
        let cleared = core
            .update_server(
                &a.id,
                ServerPatch {
                    instance_id: Some(None),
                    ..ServerPatch::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(cleared.instance_id, None);
        assert_eq!(cleared.account_id.as_deref(), Some("cuenta"));
        assert!(
            core.update_server(
                &a.id,
                ServerPatch {
                    address: Some("x y".into()),
                    ..ServerPatch::default()
                }
            )
            .await
            .is_err()
        );
        assert!(
            core.update_server("no-existe", ServerPatch::default())
                .await
                .is_err()
        );

        let moved = core.move_server(&c.id, 0).await.unwrap();
        assert_eq!(ids(&moved), vec![c.id.clone(), a.id.clone(), b.id.clone()]);
        let moved = core.move_server(&c.id, 99).await.unwrap();
        assert_eq!(ids(&moved), vec![a.id.clone(), b.id.clone(), c.id.clone()]);
        assert!(core.move_server("no-existe", 0).await.is_err());

        let removed = core.remove_server(&b.id).await.unwrap();
        assert_eq!(removed.id, b.id);
        assert_eq!(core.list_servers().await.unwrap().len(), 2);
        assert!(core.remove_server(&b.id).await.is_err());
    }

    // -- servers.dat -------------------------------------------------------------

    #[derive(Serialize)]
    struct DatOut {
        servers: Vec<DatServerOut>,
    }

    #[derive(Serialize)]
    struct DatServerOut {
        name: String,
        ip: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        icon: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        hidden: Option<bool>,
    }

    fn dat_server(name: &str, ip: &str, icon: bool, hidden: Option<bool>) -> DatServerOut {
        DatServerOut {
            name: name.into(),
            ip: ip.into(),
            icon: icon.then(|| "iVBORw0KGgo=".into()),
            hidden,
        }
    }

    #[tokio::test]
    async fn importa_servers_dat() {
        let dir = tempfile::tempdir().unwrap();
        let core = core(dir.path()).await;
        let inst = core
            .create_instance(NewInstance {
                name: "Con servidores".into(),
                game: GameVersion {
                    mc: "1.21.1".into(),
                    loader: LoaderKind::Vanilla,
                    loader_version: None,
                },
            })
            .await
            .unwrap();
        // Sin archivo: nada que importar.
        assert_eq!(core.import_servers_dat(&inst.id).await.unwrap(), 0);

        let bytes = fastnbt::to_bytes(&DatOut {
            servers: vec![
                dat_server("Hypixel", "mc.hypixel.net", true, None),
                dat_server("Oculto", "oculto.example.com", false, Some(true)),
                dat_server("Visible", "visible.example.com:25570", false, Some(false)),
                dat_server("", "sin-nombre.example.com", false, None),
                dat_server("Mala", "dirección mala", false, None),
                dat_server("Duplicado", "MC.HYPIXEL.NET", false, None),
            ],
        })
        .unwrap();
        std::fs::write(
            core.instance_game_dir(&inst.id)
                .unwrap()
                .join("servers.dat"),
            &bytes,
        )
        .unwrap();
        core.add_server(new("Ya estaba", "visible.example.com:25570"))
            .await
            .unwrap();

        assert_eq!(core.import_servers_dat(&inst.id).await.unwrap(), 2);
        let list = core.list_servers().await.unwrap();
        let by_addr = |a: &str| list.iter().find(|s| s.address == a).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(by_addr("mc.hypixel.net").name, "Hypixel");
        assert_eq!(
            by_addr("mc.hypixel.net").instance_id.as_deref(),
            Some(inst.id.as_str())
        );
        assert_eq!(
            by_addr("sin-nombre.example.com").name,
            "sin-nombre.example.com"
        );
        assert_eq!(by_addr("visible.example.com:25570").name, "Ya estaba");
        assert_eq!(by_addr("visible.example.com:25570").instance_id, None);
        // Idempotente.
        assert_eq!(core.import_servers_dat(&inst.id).await.unwrap(), 0);
        assert!(core.import_servers_dat("no-existe").await.is_err());
    }

    #[test]
    fn servers_dat_comprimido_y_roto() {
        let plain = fastnbt::to_bytes(&DatOut {
            servers: vec![dat_server("A", "a.example.com", false, None)],
        })
        .unwrap();
        let mut gz = Vec::new();
        {
            use std::io::Write;
            let mut enc = flate2::write::GzEncoder::new(&mut gz, flate2::Compression::fast());
            enc.write_all(&plain).unwrap();
            enc.finish().unwrap();
        }
        let expected = vec![("A".to_owned(), "a.example.com".to_owned())];
        assert_eq!(parse_servers_dat(&plain).unwrap(), expected);
        assert_eq!(parse_servers_dat(&gz).unwrap(), expected);
        assert!(parse_servers_dat(b"no es nbt").is_err());
        assert!(parse_servers_dat(&[]).is_err());
        // NBT válido sin la lista: vacío.
        let empty = fastnbt::to_bytes(&json!({})).unwrap();
        assert!(parse_servers_dat(&empty).unwrap().is_empty());
    }
}
