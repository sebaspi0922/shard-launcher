# Cuenta Shard en el launcher

El alta «Cuenta Shard» usa el Yggdrasil configurado en `settings.json`. El valor efectivo cuando `yggdrasil_base_url` está ausente o es `null` sale de `DEFAULT_YGGDRASIL_BASE_URL` en `core/src/settings.rs`. El dominio `https://auth.shard.SEBAS_DOMINIO` está pendiente de confirmar. Para probar otro proveedor, se puede añadir `"yggdrasil_base_url": "https://account.ely.by/api/authlib-injector"` al archivo de ajustes antes de iniciar el launcher.

## Alta y sesión

- El core envía `POST {base}/authserver/authenticate` con el usuario, la contraseña y el agente de protocolo `Minecraft` versión 1. El keyring guarda `accessToken`, `clientToken`, `selectedProfile` y la base normalizada en el servicio `shard.yggdrasil`. La contraseña se usa durante el alta y queda fuera de la persistencia. Eliminar la cuenta borra su entrada local del keyring.
- El identificador local de la cuenta Shard se deriva de la base y el UUID. Así pueden coexistir una cuenta Offline y una Shard con el mismo UUID de juego, como puede ocurrir con Drasl en modo UUID offline. Repetir el alta del mismo perfil en la misma base reemplaza los tokens. Cambiar el ajuste afecta a las próximas altas y cada cuenta existente conserva la base donde se autenticó.
- El lanzamiento usa `selectedProfile.name` y `selectedProfile.id` sin guiones como nombre y UUID, y el token de acceso guardado. En las versiones que reciben `--userType`, su valor es `mojang`. Si el servidor revoca la sesión o el token caduca, se repite el alta. No se añadió renovación automática ni selección entre perfiles sin `selectedProfile`.
- Microsoft conserva su autenticación y los argumentos de su carril. Offline conserva su sesión local y omite la red de autenticación. La instalación y actualización de archivos del juego siguen el comportamiento existente del launcher y su opción `--offline`.

## Argumentos JVM

Antes de lanzar con una cuenta Shard, el core pide `GET {base}` y comprueba que la respuesta sea un objeto JSON. Codifica los bytes originales en base64 estándar. Los dos argumentos se pasan como elementos individuales, después de los argumentos JVM personalizados y antes de la clase principal:

    -javaagent:<directorio de datos de Shard>/cache/authlib-injector/1.2.8/authlib-injector.jar={base}
    -Dauthlibinjector.yggdrasil.prefetched=<base64 de los bytes de GET {base}>

La base va sin slash final. Si falla la preparación de los metadatos o del JAR, el lanzamiento Shard se interrumpe antes de arrancar Java.

El core consulta `artifact/latest.json`, conserva el build 56 si cambia la versión más reciente y valida el JAR de caché y el recién descargado. El manifiesto oficial de 1.2.8 publica SHA-256, de modo que el código exige una suma publicada, valida SHA-256 cuando figura y valida SHA-1 cuando figura. Sin SHA-1 en el manifiesto de 1.2.8, la verificación de esa versión usa SHA-256. [Manifiesto oficial del build 56](https://github.com/yushijinhun/authlib-injector.yushi.moe/blob/master/artifact/56.json).

## Archivos

| Área | Archivos |
| --- | --- |
| Core | `Cargo.lock`, `core/Cargo.toml`, `core/src/accounts.rs`, `core/src/auth/mod.rs`, `core/src/auth/yggdrasil.rs`, `core/src/bin/shard.rs`, `core/src/launch.rs`, `core/src/settings.rs`, `core/tests/auth_flow.rs` |
| Puente Rust y Dart | `app/rust/src/api/accounts.rs`, `app/rust/src/api/types.rs`, `app/rust/src/frb_generated.rs`, `app/lib/src/rust/api/accounts.dart`, `app/lib/src/rust/api/types.dart`, `app/lib/src/rust/frb_generated.dart`, `app/lib/core/bridge.dart`, `app/lib/core/providers.dart`, `app/test/support/fake_bridge.dart` |
| Interfaz | `app/lib/ui/accounts/account_dialogs.dart`, `app/lib/ui/accounts/account_panel.dart`, `app/lib/ui/home/home_page.dart`, `app/lib/ui/first_run/first_run_page.dart`, `app/lib/ui/servers/server_dialog.dart`, `app/lib/ui/skins/skins_page.dart`, `app/lib/l10n/app_en.arb`, `app/lib/l10n/app_es.arb`, `app/lib/l10n/app_localizations.dart`, `app/lib/l10n/app_localizations_en.dart`, `app/lib/l10n/app_localizations_es.dart` |
| Drasl e informe | `deploy/drasl/docker-compose.yml`, `deploy/drasl/Caddyfile`, `deploy/drasl/config.toml`, `CODEX-IMPLEMENTACION.md` |

## Drasl

Compose usa la imagen del proyecto `unmojang/drasl:latest`, con SQLite en el volumen `drasl_state` y `PlayerUUIDGeneration = "offline"`. Caddy expone HTTPS y traduce la raíz JSON y las rutas Yggdrasil hacia `/authlib-injector` en Drasl. Antes de un despliegue posterior, fija el mismo dominio en `SHARD_AUTH_DOMAIN`, `Domain` y `BaseURL`, y reemplaza `DefaultAdmins` por el usuario que administrará el registro por invitación. [Compose de referencia](https://github.com/unmojang/drasl/blob/master/example/docker/docker-compose.yaml) y [almacenamiento SQLite de Drasl](https://github.com/unmojang/drasl/blob/master/db.go).

## Verificación en el VPS

Pasaron `git diff --check`, la validación de Compose y la lectura de los archivos ARB y TOML. Los tests añadidos cubren el formato de metadatos y los argumentos JVM, el checksum, el POST con wiremock, respuestas rechazadas o incompletas, la persistencia de tokens y la convivencia de cuentas con el mismo UUID.

No se pudieron ejecutar Cargo, Clippy, rustfmt ni los tests del core porque el entorno de trabajo carece de Cargo. Tampoco estaban Flutter, Dart ni `flutter_rust_bridge_codegen`. El puente generado y las localizaciones se actualizaron manualmente y queda pendiente comprobarlos con la toolchain. La descarga real del JAR, el keyring real, el arranque del juego y la compatibilidad con una instancia Drasl o Ely.by quedan sin comprobar. La sesión no pudo resolver el host del JAR y el dominio final está pendiente. Se validó el modelo de Compose sin arrancar contenedores. No hubo push ni cambios en `research/`.

Desde la raíz del repo, con las toolchains instaladas:

```sh
cargo fmt --manifest-path core/Cargo.toml -- --check
cargo clippy -p lcore --all-targets --locked -- -D warnings
cargo test -p lcore --locked
cargo check --locked -p lcore -p shard_bridge
tools/flutter.sh analyze
tools/flutter.sh test
docker compose -f deploy/drasl/docker-compose.yml config --quiet
git diff --check
```

Para la prueba de integración cuando exista el authserver, comprueba que `GET {base}` devuelve los metadatos JSON. Después añade una cuenta Shard de prueba en Flutter y ejecuta una instancia ya instalada con esa cuenta, con una Microsoft y con una Offline. Comprueba que solo Shard añade los dos argumentos de authlib-injector y que el perfil cargado coincide con la cuenta elegida.

El sandbox montó `~/work/shard-offline` como solo lectura, por lo que el informe se guardó en la raíz del repositorio.
