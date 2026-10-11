# Discord de Roxy Launcher (leído 2026-10-11)

Servidor "🛞 Roxy Launcher" (id 1022337885241876552). Solo lectura con la sesión de sebaspi. No se pudo leer #🧪︱beta-testing (privado) ni #🧙‍♀️︱changelog (enlazado pero no visible).

## Equipo y estado
- **Sasuke MC** (sasuke_mc): desarrollador principal. **zWallk**: cofundador, hace los anuncios. **Kyxnz**: moderador.
- Proyecto pequeño: "ya casi llegamos a los 100 usuarios" (#showcase, 2026-09-01).
- Cronología:
  - 2026-07-18: primera alpha 0.1.0.
  - 2026-08-02: "migración completada" del backend (login, instancias, leaderboard).
  - 2026-08-05: versión 0.6.0.
  - 2026-08-07: #current-issues ya menciona una 0.7.0 que no se publicó.
- **Caída**: desde 2026-09-30 no funcionan ni la web ni el launcher (reporte de sebaspi en #general).
  - 2026-10-01: el moderador Kyxnz atribuye la caída a "mantenimiento de los servidores donde se encuentra alojado todo nuestro launcher" (#general).
  - 2026-10-06: Sasuke solo escribe "Hola" en #general, sin explicación.
  - 2026-10-07: un usuario sigue sin poder usarlo. No hay anuncios oficiales posteriores.

## Funciones (según anuncios y canales)
**Núcleo de launcher (local, clonable):**
- Login de Microsoft.
- Multi-loader: Vanilla, Forge, Fabric, Quilt y NeoForge.
- Instancias: RAM, argumentos de Java, resolución, icono.
- Mods y datapacks de Modrinth y CurseForge (datapacks desde la 0.6.0).
- Java gestionado.
- Lista de servidores en la Home con estado vía mcsrvstat (caché con TTL).
- Discord Rich Presence.
- Contador de tiempo de juego.
- Interfaz:
  - Tema adaptativo según el wallpaper.
  - Tutorial interactivo.
  - Créditos.
  - Idiomas español e inglés.
- Opción de cerrar o minimizar, y permitir múltiples instancias (en `local_settings.json`).
- Seguridad (0.6.0): protección Zip Slip y validación de rutas.

**Dependen del servidor de Roxy (quitar o reemplazar):**
- Cuenta Roxy (email y contraseña). Ahora mismo bloquea la entrada a todo el launcher.
- "Restaurar cuentas de Microsoft entre dispositivos" con confirmación de contraseña. Indica que guardan los tokens de Microsoft en su servidor.
- Sincronización de instancias. La migración del backend menciona "instancias".
- Leaderboard y niveles.
- Ecosistema 0.6.0:
  - Perfil con Showcase, Wallet, Inventario y Tienda.
  - Logros, Colecciones y Eventos.
  - Pase de batalla.
- Noticias de partners en la Home.
- Referidos (web `roxylauncher.com/dashboard`).
- Amigos.
- **LAN por internet entre amigos**, "tipo Bedrock, sin importar el loader". Se añadió alrededor del 2026-09-02. Necesita un relay o señalización.
- Vinculación con Discord (OAuth con CSRF state; el callback dio problemas).
- Actualizador automático (tauri-plugin-updater).

## Bugs conocidos (#current-issues, #bug-reports)
- Fabric no abría en algunas versiones.
- La 26.2 no cargaba por los flags de JVM.
- El selector de cuentas tapaba el botón Launch.
- La resolución no se guardaba.
- A veces no cargaban Ajustes y Perfil (anotado en #current-issues con la etiqueta 0.7.0).
- Scroll innecesario.
- Ninguno es de Linux/Wayland. El lag en Linux viene de WebKitGTK con NVIDIA (ver 00-contexto.md).

## Sugerencias de la comunidad (#suggestions) útiles para el clon
- Importar instancias desde otra carpeta o launcher.
- Argumentos JVM por defecto que se ajusten a la RAM.
- Nombre de la instancia en el RPC.
- Resolución personalizada.
- Acceso rápido a instancias desde la Home.
- Ocultar el email (dx***@gmail.com).
- Skin real del jugador en la Home en vez de la mascota.
- Recordar la última sesión.
- Servidores de la Home: elegir instancia y cuenta por servidor, y editar o reordenar IPs.
- Pase de batalla: botón para reclamar todo; inventario por secciones.
