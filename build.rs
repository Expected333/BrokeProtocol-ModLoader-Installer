use std::path::Path;

fn main() {
    // À chaque build, on tente de rafraîchir les DLL embarquées avec la dernière
    // version compilée du mod. Si le dossier bin/Debug n'existe pas (ex : build sur
    // une autre machine), on garde les copies déjà présentes dans assets/.
    let debug_dir = Path::new("../ModLoader/ModLoader/bin/Debug");
    for dll in ["ModLoader.dll", "!_0Harmony.dll"] {
        let src = debug_dir.join(dll);
        if src.exists() {
            let dst = Path::new("assets").join(dll);
            let _ = std::fs::copy(&src, &dst);
            println!("cargo:rerun-if-changed={}", src.display());
        }
    }

    // Embarque le manifest Windows (élévation UAC "requireAdministrator" + DPI).
    // embed-manifest n'agit que sur les binaires (rustc-link-arg-bins) et gère le
    // conflit avec le manifest par défaut de rustc — les tests de la lib restent
    // exécutables sans élévation.
    #[cfg(windows)]
    {
        use embed_manifest::manifest::{DpiAwareness, ExecutionLevel};
        use embed_manifest::{embed_manifest, new_manifest};

        embed_manifest(
            new_manifest("Yanis.ModLoaderSetup")
                .requested_execution_level(ExecutionLevel::RequireAdministrator)
                .dpi_awareness(DpiAwareness::PerMonitorV2),
        )
        .expect("échec de l'embarquement du manifest Windows");
    }
}
