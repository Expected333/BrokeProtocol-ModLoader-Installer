// Masque la console en release (mais la garde en debug pour les logs).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use eframe::egui;
use mlkit::{installer, steam};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([680.0, 520.0])
            .with_min_inner_size([520.0, 420.0])
            .with_title("ModLoader Setup — BROKE PROTOCOL"),
        ..Default::default()
    };
    eframe::run_native(
        "ModLoader Setup",
        options,
        Box::new(|_cc| Ok(Box::new(SetupApp::new()))),
    )
}

/// Issue de la dernière opération, pour colorer le message final.
enum Outcome {
    Success(String),
    Failure(String),
}

struct SetupApp {
    game_dir: Option<PathBuf>,
    log: Vec<String>,
    outcome: Option<Outcome>,
}

impl SetupApp {
    fn new() -> Self {
        Self {
            game_dir: steam::detect_game_dir(),
            log: Vec::new(),
            outcome: None,
        }
    }

    fn installed(&self) -> bool {
        self.game_dir
            .as_deref()
            .is_some_and(installer::is_installed)
    }

    fn run_install(&mut self) {
        let Some(dir) = self.game_dir.clone() else {
            return;
        };
        self.log.clear();
        match installer::install(&dir) {
            Ok(steps) => {
                self.log = steps;
                self.outcome = Some(Outcome::Success(
                    "ModLoader installé ! Lance BROKE PROTOCOL pour en profiter.".into(),
                ));
            }
            Err(e) => {
                self.log.push(e.to_string());
                self.outcome = Some(Outcome::Failure("L'installation a échoué.".into()));
            }
        }
    }

    fn run_uninstall(&mut self) {
        let Some(dir) = self.game_dir.clone() else {
            return;
        };
        self.log.clear();
        match installer::uninstall(&dir) {
            Ok(steps) => {
                self.log = steps;
                self.outcome = Some(Outcome::Success("ModLoader désinstallé.".into()));
            }
            Err(e) => {
                self.log.push(e.to_string());
                self.outcome = Some(Outcome::Failure("La désinstallation a échoué.".into()));
            }
        }
    }

    fn browse(&mut self) {
        if let Some(picked) = rfd::FileDialog::new()
            .set_title("Sélectionne le dossier d'installation de BROKE PROTOCOL")
            .pick_folder()
        {
            if steam::is_valid_game_dir(&picked) {
                self.game_dir = Some(picked);
                self.outcome = None;
                self.log.clear();
            } else {
                self.outcome = Some(Outcome::Failure(
                    "Ce dossier ne contient pas BrokeProtocol_Data.".into(),
                ));
            }
        }
    }
}

impl eframe::App for SetupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading("Installeur du ModLoader");
            ui.label("BROKE PROTOCOL — ajoute ModLoader.dll et Harmony au jeu.");
            ui.separator();

            // --- Dossier du jeu ---
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Dossier du jeu").strong());
            match &self.game_dir {
                Some(dir) => {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(60, 180, 90), "✔");
                        ui.monospace(dir.display().to_string());
                    });
                    if self.installed() {
                        ui.colored_label(
                            egui::Color32::from_rgb(90, 160, 230),
                            "ModLoader déjà installé sur cette copie.",
                        );
                    }
                }
                None => {
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 90, 90),
                        "Installation introuvable automatiquement — sélectionne-la manuellement.",
                    );
                }
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("📁 Parcourir…").clicked() {
                    self.browse();
                }
                if ui.button("🔄 Re-détecter").clicked() {
                    self.game_dir = steam::detect_game_dir();
                    self.outcome = None;
                    self.log.clear();
                }
            });

            // --- Actions ---
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(6.0);
            let ready = self.game_dir.is_some();
            ui.horizontal(|ui| {
                let install_btn = egui::Button::new(
                    egui::RichText::new("  Installer  ").size(16.0),
                );
                if ui.add_enabled(ready, install_btn).clicked() {
                    self.run_install();
                }

                let uninstall_btn = egui::Button::new("Désinstaller");
                if ui
                    .add_enabled(ready && self.installed(), uninstall_btn)
                    .clicked()
                {
                    self.run_uninstall();
                }
            });

            // --- Résultat ---
            if let Some(outcome) = &self.outcome {
                ui.add_space(10.0);
                match outcome {
                    Outcome::Success(msg) => {
                        ui.colored_label(egui::Color32::from_rgb(60, 180, 90), msg);
                    }
                    Outcome::Failure(msg) => {
                        ui.colored_label(egui::Color32::from_rgb(220, 90, 90), msg);
                    }
                }
            }

            // --- Journal détaillé ---
            if !self.log.is_empty() {
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Détails").strong());
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for line in &self.log {
                            ui.monospace(line);
                        }
                    });
            }

            // --- Pied de page ---
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(6.0);
                ui.weak("Astuce : si l'écriture échoue, lance l'installeur en tant qu'administrateur.");
            });
        });
    }
}
