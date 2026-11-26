//! Datos de ejemplo de fake_bridge.dart y goldens_test.dart para Registros
//! (`sampleLog`) y Ajustes de la instancia (RAM del equipo y ajustes
//! globales del `FakeBridge`). Fase 3.

use crate::console::RawLine;
use crate::inst_settings::Globals;

/// `sampleLog` (test «detalle: registros»): (texto, stderr).
pub const SAMPLE_LOG: [(&str, bool); 21] = [
    ("[18:02:11] [main/INFO]: Loading Minecraft 1.21.4 with Fabric Loader 0.16.10", false),
    ("[18:02:11] [main/INFO]: Loading 87 mods:", false),
    ("\t- fabric-api 0.119.2+1.21.4", false),
    ("\t- sodium 0.6.13+mc1.21.4", false),
    ("\t- iris 1.8.8+mc1.21.4", false),
    ("[18:02:12] [main/WARN]: Mod resolution found an incompatible version of cloth-config", false),
    ("[18:02:14] [Render thread/INFO]: Setting user: NadiaK", false),
    ("[18:02:15] [Render thread/INFO]: Backend library: LWJGL version 3.3.3-snapshot", false),
    ("[18:02:16] [Render thread/DEBUG]: Created GL debug context", false),
    ("[18:02:18] [Render thread/INFO]: Reloading ResourceManager: vanilla, fabric, sodium", false),
    ("[18:02:19] [Worker-Main-4/ERROR]: Failed to load texture minecraft:textures/entity/cape.png", true),
    ("java.io.FileNotFoundException: minecraft:textures/entity/cape.png", true),
    ("\tat net.minecraft.class_3300.method_14486(class_3300.java:41)", true),
    ("\tat net.minecraft.class_1049.method_4545(class_1049.java:64)", true),
    ("[18:02:21] [Render thread/INFO]: Sound engine started", false),
    ("[18:02:21] [Render thread/INFO]: Created: 1024x512x4 minecraft:textures/atlas/blocks.png-atlas", false),
    ("[18:02:24] [Server thread/INFO]: Preparing level \"Mundo de verano\"", false),
    ("[18:02:26] [Server thread/INFO]: Preparing spawn area: 83%", false),
    ("[18:02:27] [Server thread/WARN]: Can't keep up! Is the server overloaded? Running 2140ms or 42 ticks behind", false),
    ("[18:02:28] [Render thread/INFO]: Loaded 1290 advancements", false),
    ("[18:02:29] [Server thread/INFO]: NadiaK joined the game", false),
];

pub fn sample_log() -> Vec<RawLine> {
    SAMPLE_LOG.iter().map(|&(t, e)| RawLine::new(t, e)).collect()
}

/// `getSystemMemoryMb` (16 GB) y `getSettings` del `FakeBridge`.
pub fn sample_globals() -> Globals {
    Globals { memory_mb: 4096, jvm_args: vec!["-XX:+UseG1GC".into()], total_mb: 16384 }
}
