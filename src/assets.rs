//! DLL embarquées directement dans le binaire de l'installeur, pour qu'il soit
//! autonome et distribuable en un seul fichier .exe.

/// Le ModLoader lui-même.
pub const MODLOADER_DLL: &[u8] = include_bytes!("../assets/ModLoader.dll");

/// La dépendance Harmony (patching runtime). Le `!` au début force le tri
/// alphabétique en premier côté Unity.
pub const HARMONY_DLL: &[u8] = include_bytes!("../assets/!_0Harmony.dll");

/// Les fichiers (nom, contenu) à déposer dans le dossier Managed du jeu.
pub const MANAGED_DLLS: &[(&str, &[u8])] = &[
    ("ModLoader.dll", MODLOADER_DLL),
    ("!_0Harmony.dll", HARMONY_DLL),
];
