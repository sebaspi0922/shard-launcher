# Guía: app de Azure, acceso a la API de Minecraft y claves opcionales (2026-10-11)

Para iniciar sesión con Microsoft el launcher necesita su **propia** app registrada en Microsoft Entra (Azure). El client ID de Roxy no se reutiliza. Además, Mojang tiene que aprobar esa app para que pueda usar `api.minecraftservices.com`; sin aprobación ese paso responde **403** («Invalid app registration») y solo funcionan las cuentas offline.

El client ID no es secreto (va dentro de cualquier launcher). Puedes pasármelo por el chat o por Telegram. **No hace falta ningún secreto ni contraseña** y no debes compartir ninguno.

Los menús del portal cambian con frecuencia: si algún nombre no coincide, busca el equivalente.

## 1. Registrar la app en Microsoft Entra

1. **Tenant.** Entra en <https://entra.microsoft.com> con tu cuenta de Microsoft (puede ser la misma de Minecraft o otra).
   - Si ves «Entra ID → App registrations», ya tienes directorio: sigue al paso 2.
   - Si te dice que la cuenta no tiene directorio o pide suscripción, crea una cuenta gratuita de Azure en <https://azure.microsoft.com/free>. Pide tarjeta para verificar identidad, pero registrar apps no cuesta nada. No actives servicios de pago.
2. **Nueva app.** Entra ID → App registrations → **New registration**.
   - **Name**: el que quieras, sin «Roxy» (p. ej. el nombre provisional del launcher). Lo verás tú al aceptar permisos.
   - **Supported account types**: **«Personal Microsoft accounts only»**.
   - **Redirect URI**: déjalo vacío.
   - Pulsa **Register**.
3. **Copia el «Application (client) ID»** de la pantalla Overview (un UUID). Ese es el dato que necesito. El «Directory (tenant) ID» no hace falta: el login usa el tenant `consumers`.
4. **Habilita el flujo de cliente público** (lo usa el device code): menú **Authentication** → **Advanced settings** → **Allow public client flows** = **Yes** → **Save**.
   - Si aparece una opción «Live SDK support», actívala también; si no existe, no pasa nada.
5. **(Opcional, para el futuro)** En Authentication → **Add a platform** → **Mobile and desktop applications** → marca o añade `http://localhost`. Permitirá más adelante un login con navegador sin copiar códigos. No es necesario para empezar.
6. **No hagas nada más**: ni «Certificates & secrets», ni «API permissions». El permiso `XboxLive.signin` lo pide el launcher al iniciar sesión; no se añade desde la lista.

## 2. Pedir acceso a la API de Minecraft

1. Abre el formulario de revisión de Mojang/Microsoft: <https://aka.ms/mce-reviewappid> (artículo de ayuda relacionado: <https://help.minecraft.net/hc/en-us/articles/16254801392141>).
2. Rellénalo con datos reales. Lo habitual que piden: correo de contacto, el **Application (client) ID** del paso 1.3, nombre de la app y una descripción. Una descripción honesta y suficiente:
   > Launcher de Minecraft: Java Edition de uso personal y no comercial. Inicia sesión con cuentas Microsoft que poseen el juego mediante el flujo device code (tenant consumers, scope XboxLive.signin) para obtener el token de Minecraft y lanzar el juego oficial. No se distribuye públicamente ni almacena credenciales fuera del equipo del usuario.
3. Envíalo y guarda el correo de confirmación si llega.
4. **Plazo**: no hay uno publicado. Hay reportes de días a semanas, y de solicitudes sin respuesta; algún foro de Microsoft sugiere además el programa de Xbox para desarrolladores, pero eso no es una respuesta oficial. Mientras tanto:
   - El launcher se desarrolla con pruebas simuladas y se prueba en real hasta el paso de Xbox (XSTS), que no depende de la aprobación.
   - Las partidas se prueban con cuentas offline (un jugador y servidores en modo offline).
5. Cuando llegue la aprobación (o un rechazo), avísame. Con la app aprobada, `lcore account login` debe completar la cadena entera.

**Errores que puedes ver después:**
- `403 Invalid app registration` en `login_with_xbox`: la app aún no está aprobada.
- XErr `2148916233`: la cuenta no tiene perfil de Xbox (se crea entrando una vez en xbox.com).
- XErr `2148916238`: cuenta de menor que debe estar en una familia de Microsoft (con apps propias pasa más que con el launcher oficial).

## 3. (Opcional) Discord Rich Presence

1. <https://discord.com/developers/applications> → **New Application** con el nombre que quieres que aparezca como «Jugando a …».
2. Copia el **Application ID** (público) y pásamelo.
3. En **Rich Presence → Art Assets** sube imágenes propias (p. ej. `launcher`, `minecraft`); no uses el arte de Roxy.

## 4. (Opcional) Clave de API de CurseForge

1. <https://console.curseforge.com> → inicia sesión → solicita una API key para un proyecto personal (también pasa por revisión).
2. Esta clave **sí es secreta**: no la pegues en el chat. El launcher te la pedirá en Ajustes (o con `lcore settings set-curseforge-key`) y la guardará en el keyring.

Sin clave, el launcher funciona solo con Modrinth.
