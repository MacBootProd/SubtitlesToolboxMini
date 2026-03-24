#![windows_subsystem = "windows"]

slint::include_modules!();
use rfd::FileDialog;
use std::fs;
use std::path::Path;
use encoding_rs::WINDOWS_1252;

// --- STRUCTURES & UTILITAIRES ---

#[derive(Clone, Debug)]
struct SubInfo {
    index: i32,
    start_ms: i64,
}

// Convertit les millisecondes en format SRT (HH:MM:SS,ms)
fn ms_to_srt_time(total_ms: i64) -> String {
    let t = total_ms.max(0);
    let ms = t % 1000;
    let s = (t / 1000) % 60;
    let m = (t / 60000) % 60;
    let h = t / 3600000;
    format!("{:02}:{:02}:{:02},{:03}", h, m, s, ms)
}

// Convertit un format SRT en millisecondes
fn srt_time_to_ms(s: &str) -> i64 {
    let cleaned = s.replace(',', ":");
    let parts: Vec<&str> = cleaned.split(':').collect();
    if parts.len() < 3 { return 0; }
    
    let h = parts[0].trim().parse::<i64>().unwrap_or(0);
    let m = parts[1].trim().parse::<i64>().unwrap_or(0);
    let s_val = parts[2].trim().parse::<i64>().unwrap_or(0);
    let ms = if parts.len() > 3 { parts[3].trim().parse::<i64>().unwrap_or(0) } else { 0 };
    
    (h * 3600000) + (m * 60000) + (s_val * 1000) + ms
}

// Lecture intelligente : tente l'UTF-8, puis Windows-1252 (ANSI)
fn read_file_smart(path: &str) -> Result<String, std::io::Error> {
    let bytes = fs::read(path)?;
    if let Ok(content) = String::from_utf8(bytes.clone()) {
        return Ok(content);
    }
    let (decoded, _, _) = WINDOWS_1252.decode(&bytes);
    Ok(decoded.into_owned())
}

// Scanne le contenu pour trouver les infos du premier et dernier sous-titre
fn scan_subtitles(content: &str) -> (Option<SubInfo>, Option<SubInfo>) {
    let mut subs = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains(" --> ") && i > 0 {
            // Nettoyage du BOM UTF-8 sur l'index si présent
            let idx_str = lines[i-1].trim().trim_start_matches('\u{feff}');
            let idx = idx_str.parse::<i32>().unwrap_or(0);
            let start_time = line.split(" --> ").next().unwrap_or("");
            subs.push(SubInfo { index: idx, start_ms: srt_time_to_ms(start_time) });
        }
    }
    (subs.first().cloned(), subs.last().cloned())
}

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    let ui_handle = ui.as_weak();

    // --- CALLBACK : CHARGER FICHIER 1 ---
    ui.on_open_file1_clicked({
        let h = ui_handle.clone();
        move || {
            let ui = h.unwrap();
            if let Some(p) = FileDialog::new().add_filter("Subtitles", &["srt"]).pick_file() {
                let path_str = p.display().to_string();
                ui.set_file1_path(path_str.clone().into());
                ui.set_status_text(p.file_name().unwrap().to_string_lossy().to_string().into());
                ui.set_final_result_message("".into()); // Reset message précédent

                if let Ok(content) = read_file_smart(&path_str) {
                    let (first, last) = scan_subtitles(&content);
                    if let (Some(f), Some(l)) = (first, last) {
                        ui.set_resync_idx_a(f.index.to_string().into());
                        ui.set_resync_time_a(ms_to_srt_time(f.start_ms).into());
                        ui.set_resync_idx_b(l.index.to_string().into());
                        ui.set_resync_time_b(ms_to_srt_time(l.start_ms).into());
                    }
                }
            }
        }
    });

    // --- CALLBACK : CHARGER FICHIER 2 ---
    ui.on_open_file2_clicked({
        let h = ui_handle.clone();
        move || {
            let ui = h.unwrap();
            if let Some(p) = FileDialog::new().add_filter("Subtitles", &["srt"]).pick_file() {
                ui.set_file2_path(p.display().to_string().into());
                ui.set_file2_status(p.file_name().unwrap().to_string_lossy().to_string().into());
            }
        }
    });

    // --- CALLBACK : EXECUTER ACTION ---
    ui.on_process_clicked(move |offset_str, mode, t_a, t_b, idx_a_str, idx_b_str| {
        let ui = ui_handle.unwrap();
        let p1 = ui.get_file1_path();
        if p1.is_empty() { return; }

        let mut input_content = match read_file_smart(p1.as_str()) {
            Ok(c) => c,
            Err(_) => {
                ui.set_final_result_message("❌ Error: File not found".into());
                return;
            }
        };

        let mut suffix = "_mod";

        // 1. GESTION DU MERGE
        if mode == "Merge 2 files" {
            let p2 = ui.get_file2_path();
            if let Ok(c2) = read_file_smart(p2.as_str()) {
                input_content.push_str("\n\n");
                input_content.push_str(&c2);
                suffix = "_merged";
            }
        }

        // 2. PARAMÈTRES RESYNCH / OFFSET
        let offset_ms = (offset_str.parse::<f64>().unwrap_or(0.0) * 1000.0) as i64;
        let mut ratio = 1.0;
        let mut original_a_ms = 0;
        let target_a_ms = srt_time_to_ms(&t_a);
        let target_b_ms = srt_time_to_ms(&t_b);

        if mode == "Re-synch" {
            suffix = "_resynced";
            let idx_a_user = idx_a_str.parse::<i32>().unwrap_or(1);
            let idx_b_user = idx_b_str.parse::<i32>().unwrap_or(1);
            
            let mut oa = None; 
            let mut ob = None;
            let lines: Vec<&str> = input_content.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if line.contains(" --> ") && i > 0 {
                    let idx = lines[i-1].trim().trim_start_matches('\u{feff}').parse::<i32>().unwrap_or(0);
                    if idx == idx_a_user { oa = Some(srt_time_to_ms(line.split(" --> ").next().unwrap())); }
                    if idx == idx_b_user { ob = Some(srt_time_to_ms(line.split(" --> ").next().unwrap())); }
                }
            }
            if let (Some(v_oa), Some(v_ob)) = (oa, ob) {
                original_a_ms = v_oa;
                if v_ob > v_oa && target_b_ms > target_a_ms {
                    ratio = (target_b_ms - target_a_ms) as f64 / (v_ob - v_oa) as f64;
                }
            }
        }

        // 3. TRAITEMENT LIGNE PAR LIGNE
        let mut final_lines = Vec::new();
        let mut counter = 0;
        for (i, line) in input_content.lines().enumerate() {
            let mut trimmed = line.trim();
            // Nettoyage première ligne
            if i == 0 && trimmed.starts_with('\u{feff}') { trimmed = &trimmed[3..]; }
            if trimmed.is_empty() { continue; }

            // Cas Index
            if trimmed.chars().all(|c| c.is_ascii_digit()) {
                counter += 1;
                final_lines.push(counter.to_string());
            } 
            // Cas Temps
            else if trimmed.contains(" --> ") {
                let parts: Vec<&str> = trimmed.split(" --> ").collect();
                if parts.len() < 2 { continue; }
                let s_ms = srt_time_to_ms(parts[0]);
                let e_ms = srt_time_to_ms(parts[1]);

                let (ns, ne) = match mode.as_str() {
                    "Shift" => { suffix = "_shifted"; (s_ms + offset_ms, e_ms + offset_ms) },
                    "Re-synch" => (
                        target_a_ms + ((s_ms - original_a_ms) as f64 * ratio) as i64,
                        target_a_ms + ((e_ms - original_a_ms) as f64 * ratio) as i64
                    ),
                    "Repair index" => { suffix = "_repaired"; (s_ms, e_ms) },
                    _ => (s_ms, e_ms),
                };
                final_lines.push(format!("{} --> {}", ms_to_srt_time(ns), ms_to_srt_time(ne)));
            } 
            // Cas Texte
            else {
                final_lines.push(trimmed.to_string());
            }
        }

        // 4. SAUVEGARDE
        let base_path = Path::new(p1.as_str());
        let out_filename = format!("{}{}.srt", base_path.file_stem().unwrap().to_string_lossy(), suffix);
        let out_path = base_path.with_file_name(&out_filename);
        
        let mut output_string = String::new();
        for (i, line) in final_lines.iter().enumerate() {
            // Ligne vide avant chaque nouvel index (sauf le premier)
            if i > 0 && line.chars().all(|c| c.is_ascii_digit()) {
                output_string.push_str("\r\n");
            }
            output_string.push_str(line);
            output_string.push_str("\r\n");
        }

        match fs::write(&out_path, output_string) {
            Ok(_) => {
                ui.set_final_result_message(format!("✅ Done!\nSaved as: {}", out_filename).into());
            },
            Err(_) => {
                ui.set_final_result_message("❌ Error saving file".into());
            }
        }
    });

    ui.run()
}
