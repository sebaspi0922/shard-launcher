# Hosting y dominio (plan para H5, 2026-10-12)

Investigación completa: `target/agents/hosting/informe.md` (no versionado). Precios en USD, comprobados el 2026-10-11.

## Dominio

`shard.com` está registrado desde 1995 y no aparece a la venta; habría que ofertar sin precio de referencia. `shard.gg`, `shard.app`, `getshard.com` y `shardmc.com` también están ocupados.

Libres:

| Dominio | Cloudflare (1.er año / renovación) | Nota |
|---|---|---|
| shardlauncher.com | 10,46 / 10,46 | El más reconocible. |
| shardlauncher.app | 8,20 / 14,20 | Solo HTTPS (HSTS preload). |
| shardlauncher.dev | 8,20 / 12,20 | Solo HTTPS. |
| shardlauncher.net | 11,86 / 11,86 | |

Cloudflare Registrar vende a coste y no admite `.gg` (en Porkbun, 51,80 al año).

## Arquitectura propuesta (todo en Cloudflare)

| Servicio | Producto | Coste |
|---|---|---|
| Web y `latest.json` del actualizador | Worker con assets estáticos (`web/`) | 0 |
| Descargas (`dl.`) | R2 con dominio propio; egreso gratis, 10 GB | 0 |
| API (`api.`): amigos, presencia, instancias compartidas, informes de cierre | Worker + D1 + R2 + un Durable Object por sala | 0 (plan gratuito) o 5 USD/mes |
| LAN por internet | Prototipo: túnel TCP sobre WebSocket en un Durable Object. Alternativa: relay estilo e4mc en un VPS (~6 EUR/mes) | 0 a 6 EUR/mes |

- **Identidad sin contraseñas propias:** el launcher hace `join` en el sessionserver de Mojang con un `serverId` que da la API. La API comprueba `hasJoined` y emite su propio token. La API nunca recibe el token de Minecraft. Hay que cachear la verificación, porque Mojang limita las peticiones por IP y los Workers comparten IPs.
- **Descartados para la LAN:**
  - Spectrum: TCP arbitrario solo en Enterprise.
  - TURN de Cloudflare: no hace TCP.
  - Tunnel + `cloudflared`: obliga a instalarlo en cada extremo.
- **Actualizaciones:** `latest.json` con versión, URL y firma minisign por plataforma. El launcher verifica la firma con la clave pública incorporada.

## Pasos

1. Elegir el dominio y comprarlo en Cloudflare (necesita la cuenta y el pago del usuario).
2. Publicar la web (`web/`) con `wrangler` y un `latest.json` de prueba.
3. Bucket R2 en `dl.` y clave minisign; subir AppImage, Flatpak y paquete de Arch.
4. Actualizador en la app con verificación de firma.
5. API con identidad por `hasJoined`; luego amigos y presencia.
6. Prototipo de LAN sobre Durable Objects; medir con 3–4 jugadores.
7. Registrar la decisión en `docs/decisiones.md` antes de empezar H5.
