//! Errores de Xbox Live devueltos por el flujo SISU de `minecraft-auth`.

/// Explicación de los XErr conocidos (minecraft.wiki, «Microsoft authentication»).
pub fn xerr_message(xerr: u64) -> &'static str {
    match xerr {
        2148916227 => "la cuenta está bloqueada en Xbox",
        2148916233 => "la cuenta no tiene perfil de Xbox; créalo entrando una vez en xbox.com",
        2148916235 => "Xbox Live no está disponible en el país de la cuenta",
        2148916236 | 2148916237 => "la cuenta necesita verificación de adulto en xbox.com",
        2148916238 => "cuenta de menor: un adulto debe añadirla a una familia de Microsoft",
        _ => "error de Xbox Live",
    }
}
