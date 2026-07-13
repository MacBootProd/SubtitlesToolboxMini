// --- Subtitles Toolbox mini - main.rs ---
// --- CONFIGURATION OS ---
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(windows)] const LINE_ENDING: &str = "\r\n";
#[cfg(not(windows))] const LINE_ENDING: &str = "\n";

slint::include_modules!();
use rfd::FileDialog;
use std::{fs, rc::Rc, cell::RefCell, path::{Path, PathBuf}};
use encoding_rs::WINDOWS_1252;

// use slint::language::ColorScheme;
// --- SETTINGS ---
use serde::{Serialize, Deserialize};
#[derive(Serialize, Deserialize, Clone, Debug)]
struct AppSettings { is_dark: bool, merge_gap: String, shift_val: String, save_mode: i32, custom_path: String, lang: String }
impl Default for AppSettings { fn default() -> Self { Self { is_dark: true, merge_gap: "010".into(), shift_val: "+2.0".into(), save_mode: 0, custom_path: "".into(), lang: "en".into() } } }
    fn get_settings_path() -> PathBuf { let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")); p.push("SubtitlesToolboxMini"); let _ = fs::create_dir_all(&p); p.push("settings.json"); p }
    fn load_settings() -> AppSettings { fs::read_to_string(get_settings_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default() }
    fn save_settings(s: &AppSettings) { if let Ok(j) = serde_json::to_string_pretty(s) { let _ = fs::write(get_settings_path(), j); } }
// --- STRUCTURES ---
#[derive(Clone, Debug)] 
struct SubInfo { index: i32, start_ms: i64, end_ms: i64, text: String, source: i32 }
struct PendingSave { path: PathBuf, content: String }
// --- TIME UTILITIES ---
fn ms_to_srt_time(ms: i64) -> String { 
    let t = ms.max(0); 
    format!("{:02}:{:02}:{:02},{:03}", t/3600000, (t/60000)%60, (t/1000)%60, t%1000) 
}
fn srt_time_to_ms(s: &str) -> i64 {
    let cleaned = s.replace(',', ":").replace('.', ":");
    let parts: Vec<&str> = cleaned.split(':').collect();
    let len = parts.len();
    if len < 2 { return 0; }
    let ms = parts[len - 1].trim().parse::<i64>().unwrap_or(0);
    let sec = parts[len - 2].trim().parse::<i64>().unwrap_or(0) * 1000;
    let min = parts[len - 3].trim().parse::<i64>().unwrap_or(0) * 60000;
    let hrs = if len > 3 { parts[len - 4].trim().parse::<i64>().unwrap_or(0) * 3600000 } else { 0 };
    hrs + min + sec + ms}
// --- READING ET PARSING ---
fn read_file_smart(p: &str) -> Result<String, std::io::Error> { 
    let b = fs::read(p)?; 
    String::from_utf8(b.clone()).or_else(|_| Ok(WINDOWS_1252.decode(&b).0.into_owned())) 
}
fn parse_first_sub(content: &str) -> Option<SubInfo> {
    let l: Vec<&str> = content.lines().collect();
    for (i, line) in l.iter().enumerate() {
        if line.contains(" --> ") && i > 0 {
            let idx = l[i-1].trim().trim_start_matches('\u{feff}').parse::<i32>().unwrap_or(0);
            let mut txt = Vec::new(); let mut j = i + 1;
            while j < l.len() && !l[j].trim().is_empty() { txt.push(l[j].trim()); j += 1; }
            return Some(SubInfo { index: idx, start_ms: srt_time_to_ms(line.split(" --> ").next().unwrap()), end_ms: 0, text: txt.join("\n"), source: 0 });
        }}
        None}
#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    let init_settings = load_settings();
    let settings = Rc::new(RefCell::new(init_settings.clone()));
    let ui = AppWindow::new()?;
    let ui_h = ui.as_weak();

             // Define sorted languages. English locked at index 0.
        let languages_display = ["English", "Français", "Italiano"];
        let languages_codes = ["en", "fr", "it"];

        // Send the flat text array directly to Slint AppState component
        let lang_model = std::rc::Rc::new(slint::VecModel::from(
            languages_display.iter().map(|&s| slint::SharedString::from(s)).collect::<Vec<_>>()
        ));
        ui.global::<AppState>().set_available_languages(lang_model.into());

        // Resolve current index based on user configuration
        let current_idx = languages_codes.iter().position(|&c| c == init_settings.lang).unwrap_or(0) as i32;
        ui.global::<AppState>().set_s_lang_index(current_idx);
        ui.global::<AppState>().set_tmp_lang_index(current_idx);

        // Rust listens to Slint UI selection and resolves the index dynamically
        ui.on_language_changed_in_ui({
            let h = ui_h.clone();
            move |val| {
                let ui = h.unwrap();
                // Search the clicked string position inside our official Rust array
                let idx = languages_display.iter().position(|&l| l == val.as_str()).unwrap_or(0);
                // Instantly update Slint's temporary index mirror
                ui.global::<AppState>().set_tmp_lang_index(idx as i32);
            }
        });

        if init_settings.lang == "en" { let _ = slint::select_bundled_translation("en"); }
        else if slint::select_bundled_translation(&init_settings.lang).is_err() { let _ = slint::select_bundled_translation("en"); }

    {   
        let s = settings.borrow();
        ui.global::<Theme>().set_is_dark(s.is_dark);
        ui.set_s_lang(s.lang.clone().into()); // Send selected language to UI
        ui.set_m_gap(s.merge_gap.clone().into());
        ui.set_offset_val(s.shift_val.clone().into());
        ui.set_default_shift_seconds(s.shift_val.clone().into());
        ui.set_s_save_mode(s.save_mode);
        ui.set_s_custom_path(s.custom_path.clone().into());
        ui.set_mode_same(s.save_mode == 0);
        ui.set_mode_ask(s.save_mode == 1);
        ui.set_mode_custom(s.save_mode == 2);
    }
    ui.on_prepare_settings({
        let h = ui_h.clone(); let s_rc = settings.clone();
        move || {
            let ui = h.unwrap(); let s = s_rc.borrow();
            ui.set_tmp_dark(s.is_dark);
            ui.global::<AppState>().set_tmp_lang_index(ui.global::<AppState>().get_s_lang_index());
            ui.set_tmp_offset(s.shift_val.clone().into());
            ui.set_tmp_gap(s.merge_gap.clone().into());
            ui.set_tmp_save_mode(s.save_mode);
            ui.set_tmp_custom_path(s.custom_path.clone().into());}
    });
    ui.on_save_settings({
        let h = ui_h.clone(); let s_rc = settings.clone();
        move || {
            let ui = h.unwrap(); let mut s = s_rc.borrow_mut();
            s.is_dark = ui.global::<Theme>().get_is_dark();
            s.lang = ui.get_s_lang().to_string(); // set choices
            s.merge_gap = ui.get_m_gap().to_string();
            s.shift_val = ui.get_offset_val().to_string();
            s.save_mode = ui.get_s_save_mode();
            s.custom_path = ui.get_s_custom_path().to_string();
            let selected_idx = ui.global::<AppState>().get_tmp_lang_index() as usize;
            s.lang = languages_codes[selected_idx].to_string();
            ui.global::<AppState>().set_s_lang_index(selected_idx as i32);
            ui.set_s_lang(s.lang.clone().into());
            
            save_settings(&s);

            // Instant and seamless translation update at runtime
            let _ = slint::select_bundled_translation(&s.lang);}
    });
    ui.on_pick_custom_folder({
    let h = ui_h.clone();
    move || {
        let ui = h.unwrap();
        if let Some(p) = FileDialog::new().pick_folder() {
            ui.set_tmp_custom_path(p.to_string_lossy().to_string().into());
        }}});
    let all_subs = Rc::new(RefCell::new(Vec::<SubInfo>::new()));
    let pending_save = Rc::new(RefCell::new(None::<PendingSave>));
ui.on_open_extlink(|| { let _ = webbrowser::open("https://github.com/MacBootProd/SubtitlesToolboxMini"); });
// --- LOAD FILE 1 ---
    ui.on_open_file1_clicked({ 
        let h=ui_h.clone(); let subs=all_subs.clone(); 
        move |tab, conv_idx| { 
            let ui=h.unwrap(); let mut dialog = FileDialog::new();
            match tab {
                0 | 2 => dialog = dialog.add_filter("SRT Subtitles", &["srt"]),
                1 => match conv_idx { 0 => dialog = dialog.add_filter("ASS Subtitles", &["ass"]), 1 => dialog = dialog.add_filter("VobSub / MicroDVD Subtitles", &["sub"]), 2 => dialog = dialog.add_filter("Text Files", &["txt"]), 3 => dialog = dialog.add_filter("WebVTT Subtitles", &["vtt"]), 4 => dialog = dialog.add_filter("SRT Subtitles", &["srt"]), _ => {} },
                _ => {}
            }
            if let Some(p)=dialog.pick_file() {
                *subs.borrow_mut() = Vec::new();
                let ps = p.to_string_lossy().to_string(); ui.set_file1_path(ps.clone().into()); ui.set_status_text(p.file_name().unwrap().to_string_lossy().to_string().into());
                if tab == 1 && conv_idx == 1 {
    if p.with_extension("idx").exists() {
        *subs.borrow_mut() = Vec::new(); // Clear global subtitles vector
        ui.set_file1_path("".into()); // Clear path to prevent EXECUTE ACTION
        let err_msg = ui.global::<AppState>().invoke_get_vobsub_error();
        ui.set_status_text(err_msg);
        return;
    }
}

                if let Ok(c)=read_file_smart(&ps) {
                    let mut v = Vec::new(); let ls: Vec<&str> = c.lines().collect();
                    for (i, l) in ls.iter().enumerate() { 
                        if l.contains(" --> ") && i > 0 { 
                            let mut t=Vec::new(); let mut j=i+1; while j<ls.len() && !ls[j].trim().is_empty() { t.push(ls[j].trim()); j+=1; } 
                            v.push(SubInfo { index: ls[i-1].trim().trim_start_matches('\u{feff}').parse().unwrap_or(0), start_ms: srt_time_to_ms(l.split(" --> ").next().unwrap()), end_ms: srt_time_to_ms(l.split(" --> ").last().unwrap()), text: t.join("\n"), source: 1 }); }  }
                    *subs.borrow_mut() = v.clone();
                    if let (Some(f), Some(l)) = (v.first(), v.last()) { 
                        ui.set_resync_idx_a(f.index.to_string().into()); ui.set_resync_time_a(ms_to_srt_time(f.start_ms).into()); ui.set_resync_text_a(f.text.clone().into()); 
                        ui.set_resync_idx_b(l.index.to_string().into()); ui.set_resync_time_b(ms_to_srt_time(l.start_ms).into()); ui.set_resync_text_b(l.text.clone().into()); 
                 } }}}
            });
// --- LOAD FILE 2 ---
    ui.on_open_file2_clicked({ 
        let h=ui_h.clone(); let subs=all_subs.clone(); 
        move || { 
            let ui=h.unwrap(); 
            if let Some(p)=FileDialog::new().add_filter("SRT", &["srt"]).pick_file() {
                let ps = p.to_string_lossy().to_string();
                ui.set_file2_path(ps.clone().into()); 
                ui.set_file2_status(p.file_name().unwrap().to_string_lossy().to_string().into());
                if let Ok(c)=read_file_smart(&ps) { 
                    if let Some(s)=parse_first_sub(&c) { ui.set_resync_text_b(s.text.into()); } 
                    let next = subs.borrow().last().map(|x| x.index + 1).unwrap_or(1); 
                    ui.set_resync_idx_b(next.to_string().into()); 
                    // LOGIQUE CORRIGÉE : Dernier timing de fin du Fichier 1 + 10ms
                    let target_ms = subs.borrow().last().map(|x| x.end_ms + 10).unwrap_or(0);
                    ui.set_offset_val(ms_to_srt_time(target_ms).into()); } } }
    });
    // --- NAVIGATE A/B ---
    let upd = |ui: AppWindow, s: SubInfo, is_a: bool| { 
        if is_a { ui.set_resync_idx_a(s.index.to_string().into()); ui.set_resync_time_a(ms_to_srt_time(s.start_ms).into()); ui.set_resync_text_a(s.text.into()); } 
        else { ui.set_resync_idx_b(s.index.to_string().into()); ui.set_resync_time_b(ms_to_srt_time(s.start_ms).into()); ui.set_resync_text_b(s.text.into()); }  };
    ui.on_prev_sub_a_clicked({ let h=ui_h.clone(); let s=all_subs.clone(); move || { let ui=h.unwrap(); let idx=ui.get_resync_idx_a().parse::<i32>().unwrap_or(0); let subs=s.borrow(); if let Some(p)=subs.iter().position(|x| x.index==idx) { if p>0 { upd(ui, subs[p-1].clone(), true); } } } });
    ui.on_next_sub_a_clicked({ let h=ui_h.clone(); let s=all_subs.clone(); move || { let ui=h.unwrap(); let idx=ui.get_resync_idx_a().parse::<i32>().unwrap_or(0); let subs=s.borrow(); if let Some(p)=subs.iter().position(|x| x.index==idx) { if p+1<subs.len() { upd(ui, subs[p+1].clone(), true); } } } });
    ui.on_prev_sub_b_clicked({ let h=ui_h.clone(); let s=all_subs.clone(); move || { let ui=h.unwrap(); let idx=ui.get_resync_idx_b().parse::<i32>().unwrap_or(0); let subs=s.borrow(); if let Some(p)=subs.iter().position(|x| x.index==idx) { if p>0 { upd(ui, subs[p-1].clone(), false); } } } });
    ui.on_next_sub_b_clicked({ let h=ui_h.clone(); let s=all_subs.clone(); move || { let ui=h.unwrap(); let idx=ui.get_resync_idx_b().parse::<i32>().unwrap_or(0); let subs=s.borrow(); if let Some(p)=subs.iter().position(|x| x.index==idx) { if p+1<subs.len() { upd(ui, subs[p+1].clone(), false); } } } });
    // --- CONFLICT POPUP ---
        ui.on_conflict_resolved({ 
        let h=ui_h.clone(); let ps=pending_save.clone(); 
        move |action| { 
            let ui=h.unwrap(); 
            if let Some(d)=ps.borrow_mut().take() { 
                match action.as_str() {
                    "overwrite" => { 
                        let _=fs::write(&d.path, &d.content); 
                        let filename = d.path.file_name().unwrap().to_string_lossy().to_string();
                        let success_msg = ui.global::<AppState>().invoke_get_save_success(filename.into());
                        ui.set_final_result_message(success_msg); 
                    }
                    "rename" => {
                        if let Some(np)=FileDialog::new().set_file_name(d.path.file_name().unwrap().to_string_lossy().as_ref()).set_directory(d.path.parent().unwrap()).add_filter("SRT", &["srt"]).save_file() { 
                            let _=fs::write(&np, &d.content); 
                            let filename = np.file_name().unwrap().to_string_lossy().to_string();
                            let success_msg = ui.global::<AppState>().invoke_get_save_success(filename.into());
                            ui.set_final_result_message(success_msg); 
                        }  
                    }
                    _ => { 
                        let cancel_msg = ui.global::<AppState>().invoke_get_cancelled();
                        ui.set_final_result_message(cancel_msg); 
                        }
                    }
                }
            }
        });
// --- ACTION MAIN ---
        ui.on_process_clicked({ 
        let h=ui_h.clone(); let ps=pending_save.clone(); 
                move |off_s, mode, t_a, t_b, i_a, i_b, ovl, dur, keep_meta, keep_style, fps_str, m_mode, l_mode, src_fps_idx, tgt_fps_idx, p_style, p_format| {
            let ui=h.unwrap(); let p1=ui.get_file1_path(); let p2=ui.get_file2_path();
            
            // 1. SAFETY VERIFICATIONS
            if p1.is_empty() { 
                let err_msg = ui.global::<AppState>().invoke_get_error_load_first();
                ui.set_final_result_message(err_msg); 
                return; 
            }
            let ext = Path::new(p1.as_str()).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if (mode.starts_with("Clean") || ["Shift", "Re-sync", "Repair index", "Append 2 files", "Merge 2 files", "Change_Framerate"].contains(&mode.as_str())) && ext != "srt" { 
                let err_msg = ui.global::<AppState>().invoke_get_error_require_srt();
                ui.set_final_result_message(err_msg); 
                return; 
            }
            if mode.starts_with("Convert_") {
                let req = match mode.as_str() { "Convert_ASS" => "ass", "Convert_SUB" => "sub", "Convert_TXT" => "txt", "Convert_VTT" => "vtt", "Convert_PlainText" => "srt", _ => "" };
                if ext != req { 
                    let err_msg = ui.global::<AppState>().invoke_get_extension_error(req.into());
                    ui.set_final_result_message(err_msg); 
                return; }
            }
            if (mode == "Merge 2 files" || mode == "Append 2 files") && p2.is_empty() { 
                let err_msg = ui.global::<AppState>().invoke_get_error_load_file2();
                ui.set_final_result_message(err_msg); 
                return; 
            }
            let c1=match read_file_smart(p1.as_str()) { Ok(c)=>c, Err(_)=>return };
            let (mut fl, mut cnt, mut last, mut suf) = (Vec::new(), 0, -1, "_mod");
            let off_ms = if ui.get_shift_mode_hms() { if off_s.starts_with('-') { -srt_time_to_ms(&off_s[1..]) } else { srt_time_to_ms(&off_s) } } else { (off_s.parse::<f64>().unwrap_or(0.0)*1000.0) as i64 };
            let (mut rat, mut oa_ms, ta_ms, tb_ms) = (1.0, 0, srt_time_to_ms(&t_a), srt_time_to_ms(&t_b));
            
            // --- RE-SYNC ---
            if mode=="Re-sync" { 
                suf="_resync"; 
                let ia=i_a.parse::<i32>().unwrap_or(1); let ib=i_b.parse::<i32>().unwrap_or(1); 
                let (mut oa, mut ob)=(None, None); let lines: Vec<&str>=c1.lines().collect(); 
                for (i,l) in lines.iter().enumerate() { 
                    if l.contains(" --> ") && i>0 { 
                        let idx=lines[i-1].trim().trim_start_matches('\u{feff}').parse::<i32>().unwrap_or(0); 
                        if idx==ia {oa=Some(srt_time_to_ms(l.split(" --> ").next().unwrap()));} 
                        if idx==ib {ob=Some(srt_time_to_ms(l.split(" --> ").next().unwrap()));} 
                    }  } 
                if let (Some(v_oa), Some(v_ob))=(oa, ob) { oa_ms=v_oa; if v_ob>v_oa && tb_ms>ta_ms { rat=(tb_ms-ta_ms) as f64 / (v_ob-v_oa) as f64; } } 
            }
            // --- CHANGE FRAMERATE ---
            if mode == "Change_Framerate" {
                suf = "_fps";
                let f_list = ["23.976", "24", "25", "29.97", "30", "50", "59.94", "60"];
                let src_str = f_list.get(src_fps_idx as usize).unwrap_or(&"23.976");
                let tgt_str = f_list.get(tgt_fps_idx as usize).unwrap_or(&"25");
                let src_fps: f64 = src_str.parse().unwrap_or(23.976);
                let tgt_fps: f64 = tgt_str.parse().unwrap_or(25.0);
                rat = src_fps / tgt_fps;
            }
            // --- PROCESS FILE 1 ---
            for (i,l) in c1.lines().enumerate() {
                let mut tr=l.trim(); if i==0 && tr.starts_with('\u{feff}') {tr=&tr[3..];} if tr.is_empty() {continue;}
                if tr.chars().all(|c| c.is_ascii_digit()) { if mode == "Repair index" || mode == "Append 2 files" { cnt += 1; fl.push(cnt.to_string()); } else { fl.push(tr.to_string()); } }
                else if tr.contains(" --> ") {
                    let sp: Vec<&str>=tr.split(" --> ").collect(); let (s, e)=(srt_time_to_ms(sp[0]), srt_time_to_ms(sp[1]));
                    let mut ns = match mode.as_str() { "Shift"=>{suf="_shifted"; s+off_ms}, "Re-sync"=>ta_ms+((s-oa_ms) as f64 * rat) as i64, "Change_Framerate"=>(s as f64 * rat) as i64, _=>s };
                    let mut ne = if dur && mode=="Re-sync" {ns+(e-s)} else { match mode.as_str() { "Shift"=>e+off_ms, "Re-sync"=>ta_ms+((e-oa_ms) as f64 * rat) as i64, "Change_Framerate"=>(e as f64 * rat) as i64, "Repair index"=>{suf="_repair"; e}, _=>e } };
                    if ovl && last != -1 && ns < (last+10) { let d=(last+10)-ns; ns+=d; ne+=d; } last=ne; fl.push(format!("{} --> {}", ms_to_srt_time(ns), ms_to_srt_time(ne)));
                } else { fl.push(tr.to_string()); }
            }
    // --- APPEND ---
            if mode=="Append 2 files" { 
                suf="_append"; 
                if let Ok(c2)=read_file_smart(ui.get_file2_path().as_str()) {
                    let (target, mut orig) = (srt_time_to_ms(&off_s), 0); 
                    for l in c2.lines() { if l.contains(" --> ") { orig=srt_time_to_ms(l.split(" --> ").next().unwrap()); break; } }
                    let sft = target-orig; 
                    for l in c2.lines() { 
                        let t=l.trim(); if t.is_empty() {continue;} 
                        if t.chars().all(|c| c.is_ascii_digit()) { cnt+=1; fl.push(cnt.to_string()); } 
                        else if t.contains(" --> ") { 
                            let p: Vec<&str>=t.split(" --> ").collect(); 
                            fl.push(format!("{} --> {}", ms_to_srt_time(srt_time_to_ms(p[0])+sft), ms_to_srt_time(srt_time_to_ms(p[1])+sft))); 
                        } else { fl.push(t.to_string()); } 
            }}}
    // --- MERGE 2 FILES ---
        if mode == "Merge 2 files" {
            suf = "_merged";
            if let Ok(c2) = read_file_smart(ui.get_file2_path().as_str()) {
                let mut all: Vec<SubInfo> = Vec::new();
                let parse_to_vec = |content: String, src: i32, target: &mut Vec<SubInfo>| {
                    let ls: Vec<&str> = content.lines().collect();
                    for (i, l) in ls.iter().enumerate() { if l.contains(" --> ") && i > 0 {
                        let sp: Vec<&str> = l.split(" --> ").collect();
                        target.push(SubInfo { index: 0, start_ms: srt_time_to_ms(sp[0]), end_ms: srt_time_to_ms(sp[1]), text: {let mut t=Vec::new(); let mut j=i+1; while j<ls.len() && !ls[j].trim().is_empty() {t.push(ls[j].trim()); j+=1;} t.join("\n")}, source: src });
                    }}
                };
                parse_to_vec(c1.clone(), 1, &mut all); parse_to_vec(c2, 2, &mut all);

                let mut filtered: Vec<SubInfo> = Vec::new();
                let gap = ui.get_m_gap().parse::<i64>().unwrap_or(10);
                // On pré-sépare le favori pour comparer
                let f1: Vec<SubInfo> = all.iter().filter(|s| s.source == 1).cloned().collect();
                let f2: Vec<SubInfo> = all.iter().filter(|s| s.source == 2).cloned().collect();

                for s in &all {
                    let mut is_duplicate = false;
                    // On ne teste l'exclusion que si 's' provient du fichier NON-favori
                    let is_non_fav = (ui.get_fav_f2() && s.source == 1) || (!ui.get_fav_f2() && s.source == 2);
                    
                    if is_non_fav {
                        let favorites = if ui.get_fav_f2() { &f2 } else { &f1 };
                        for fav in favorites {
                            let sd = (s.start_ms - fav.start_ms).abs();
                            let ed = (s.end_ms - fav.end_ms).abs();
                            let match_s = ui.get_m_start() && sd <= gap;
                            let match_d = ui.get_m_dur() && (sd <= gap || ed <= gap);
                            
                            if match_s || match_d { is_duplicate = true; break; }
                        }
                    }
                    if !is_duplicate { filtered.push(s.clone()); }
                }
                filtered.sort_by_key(|s| s.start_ms);
                fl.clear(); cnt = 0;
                for s in filtered { cnt += 1; fl.push(cnt.to_string()); fl.push(format!("{} --> {}", ms_to_srt_time(s.start_ms), ms_to_srt_time(s.end_ms))); fl.push(s.text); }
            }
        }
// --- CONVERT LOGIC ---
        if mode.starts_with("Convert_") {
            suf = "_converted"; fl.clear(); cnt = 0;
                let fps: f64 = match fps_str.as_str() {
                    "23.976" => 23.976,
                    "24" => 24.0,
                    "25" => 25.0,
                    "29.97" => 29.97,
                    "30" => 30.0,
                    "50" => 50.0,
                    "59.94" => 59.94,
                    "60" => 60.0,
                    _ => 25.0,
                };
                let ms_per_frame = 1000.0 / fps;
            
            if mode == "Convert_ASS" {
                for l in c1.lines() { if l.starts_with("Dialogue:") {
                    let parts: Vec<&str> = l.splitn(10, ',').collect(); if parts.len() >= 10 { cnt += 1;
                        let parse_ass_time = |s: &str| -> String { let p: Vec<&str> = s.split(':').collect(); if p.len() < 3 { return "00:00:00,000".into(); } let h = p[0].trim().parse::<i64>().unwrap_or(0); let m = p[1].trim().parse::<i64>().unwrap_or(0); let sec_parts: Vec<&str> = p[2].split('.').collect(); let s_sec = sec_parts.get(0).unwrap_or(&"0").trim().parse::<i64>().unwrap_or(0); let c_sec = sec_parts.get(1).unwrap_or(&"0").trim().parse::<i64>().unwrap_or(0); format!("{:02}:{:02}:{:02},{:03}", h, m, s_sec, c_sec * 10) };
                        let start_srt = parse_ass_time(parts[1]); let end_srt = parse_ass_time(parts[2]);
                        let raw_txt = parts[9].to_string(); let mut clean_txt = String::new(); let mut inside = false; let mut tag_buf = String::new();
                        let (mut op_i, mut op_b, mut op_u) = (false, false, false);
                        for c in raw_txt.chars() { if c == '{' { inside = true; tag_buf.clear(); } else if c == '}' { inside = false; if keep_style { if tag_buf.contains("\\i1") { clean_txt.push_str("<i>"); op_i = true; } if tag_buf.contains("\\i0") { clean_txt.push_str("</i>"); op_i = false; } if tag_buf.contains("\\b1") { clean_txt.push_str("<b>"); op_b = true; } if tag_buf.contains("\\b0") { clean_txt.push_str("</b>"); op_b = false; } if tag_buf.contains("\\u1") { clean_txt.push_str("<u>"); op_u = true; } if tag_buf.contains("\\u0") { clean_txt.push_str("</u>"); op_u = false; } } } else if inside { tag_buf.push(c); } else { clean_txt.push(c); } }
                        if op_i { clean_txt.push_str("</i>"); } if op_b { clean_txt.push_str("</b>"); } if op_u { clean_txt.push_str("</u>"); }
                        fl.push(cnt.to_string()); fl.push(format!("{} --> {}", start_srt, end_srt)); fl.push(clean_txt.replace("\\N", LINE_ENDING).replace("\\n", LINE_ENDING));
                    }
                }}
            }
            if mode == "Convert_SUB" {
                for l in c1.lines() { if l.starts_with('{') { if let Some(e1) = l.find('}') { if let Some(s2) = l[e1+1..].find('{') { if let Some(e2) = l[e1+1+s2..].find('}') { cnt += 1;
                    let f1 = l[1..e1].parse::<i64>().unwrap_or(0); let f2 = l[e1+2+s2..e1+1+s2+e2].parse::<i64>().unwrap_or(0);
                    let s_ms = (f1 as f64 * ms_per_frame) as i64; let e_ms = (f2 as f64 * ms_per_frame) as i64;
                    let raw_txt = l[e1+1+s2+e2+1..].trim().to_string(); let mut clean_txt = String::new(); let mut inside = false; let mut tag_buf = String::new();
                    let (mut op_i, mut op_b, mut op_u, mut op_c) = (false, false, false, false);
                    for c in raw_txt.chars() { if c == '{' { inside = true; tag_buf.clear(); } else if c == '}' { inside = false; if keep_style { 
                        if tag_buf.starts_with("y:") { let tags = &tag_buf[2..]; if tags.contains('i') { clean_txt.push_str("<i>"); op_i = true; } if tags.contains('b') { clean_txt.push_str("<b>"); op_b = true; } if tags.contains('u') { clean_txt.push_str("<u>"); op_u = true; } }
                        if tag_buf.starts_with("c:$") && tag_buf.len() == 9 { let bgr = &tag_buf[3..9]; if let (Some(b), Some(g), Some(r)) = (bgr.get(0..2), bgr.get(2..4), bgr.get(4..6)) { clean_txt.push_str(&format!("<font color=\"#{}{}{}\">", r, g, b)); op_c = true; } }
                    } } else if inside { tag_buf.push(c); } else { clean_txt.push(c); } }
                    if op_c { clean_txt.push_str("</font>"); } if op_u { clean_txt.push_str("</u>"); } if op_b { clean_txt.push_str("</b>"); } if op_i { clean_txt.push_str("</i>"); }
                    fl.push(cnt.to_string()); fl.push(format!("{} --> {}", ms_to_srt_time(s_ms), ms_to_srt_time(e_ms))); fl.push(clean_txt.replace('|', LINE_ENDING));
                }}}}}
            }
            if mode == "Convert_TXT" {
                let sample = c1.lines().map(|l| l.trim()).find(|l| !l.is_empty()).unwrap_or("");
                if sample.starts_with('{') {
                    for l in c1.lines() { if l.starts_with('{') { if let Some(e1) = l.find('}') { if let Some(s2) = l[e1+1..].find('{') { if let Some(e2) = l[e1+1+s2..].find('}') {
                        let f1 = l[1..e1].parse::<i64>().unwrap_or(0); let f2 = l[e1+2+s2..e1+1+s2+e2].parse::<i64>().unwrap_or(0);
                        if f1 == 1 && f2 == 1 { continue; } cnt += 1;
                        let s_ms = (f1 as f64 * ms_per_frame) as i64; let e_ms = (f2 as f64 * ms_per_frame) as i64;
                        let txt = l[e1+1+s2+e2+1..].trim().to_string();
                        fl.push(cnt.to_string()); fl.push(format!("{} --> {}", ms_to_srt_time(s_ms), ms_to_srt_time(e_ms))); fl.push(txt.replace('|', LINE_ENDING));
                    }}}}}
                } else {
                    let ls: Vec<&str> = c1.lines().collect();
                    for (i, l) in ls.iter().enumerate() { if l.contains(" --> ") && i > 0 { cnt += 1;
                        let p: Vec<&str> = l.split(" --> ").collect();
                        let s = srt_time_to_ms(p[0]);
                        let mut ep = p[1].split_whitespace();
                        let e = srt_time_to_ms(ep.next().unwrap_or(""));
                        let mut t = Vec::new(); let mut j = i + 1;
                        while j < ls.len() && !ls[j].trim().is_empty() && !ls[j].contains(" --> ") {
                            let tr = ls[j].trim(); if !tr.chars().all(|c| c.is_ascii_digit()) { t.push(tr); } j += 1;
                        }
                        fl.push(cnt.to_string()); fl.push(format!("{} --> {}", ms_to_srt_time(s), ms_to_srt_time(e))); fl.push(t.join("\n"));
                    }}
                }
            }
            if mode == "Convert_VTT" {
                let lines: Vec<&str> = c1.lines().collect();
                for (i, line) in lines.iter().enumerate() { let tr = line.trim(); if tr.contains(" --> ") { cnt += 1;
                    let p: Vec<&str> = tr.split(" --> ").collect(); let s = srt_time_to_ms(p[0]);
                    let mut ep = p[1].split_whitespace(); let e = srt_time_to_ms(ep.next().unwrap_or(""));
                    let mut meta = String::new(); if keep_meta && i > 0 { let pr = lines[i-1].trim(); if !pr.is_empty() && pr != "WEBVTT" && !pr.chars().all(|c| c.is_ascii_digit()) { if let Some(sp) = pr.find(' ') { let m_txt = pr[sp..].trim(); if !m_txt.is_empty() { meta = format!("[{}]", m_txt); } } else { meta = format!("[{}]", pr); } } }
                    let mut txts = Vec::new(); if !meta.is_empty() { txts.push(meta); } let mut j = i + 1;
                    while j < lines.len() && !lines[j].trim().is_empty() && !lines[j].contains(" --> ") { let txt_tr = lines[j].trim(); if !txt_tr.chars().all(|c| c.is_ascii_digit()) { txts.push(txt_tr.to_string()); } j += 1; }
                    fl.push(cnt.to_string()); fl.push(format!("{} --> {}", ms_to_srt_time(s), ms_to_srt_time(e))); fl.push(txts.join("\n"));
                }}
            }
// --- CONVERT : SRT TO LITERARY DOCUMENT (TXT / RTF) ---
    if mode == "Convert_PlainText" {
    suf = if p_format == 1 { "_plain.rtf" } else { "_plain.txt" }; fl.clear();

    struct SrtBlock { text_pure: String, start_ms: i64, end_ms: i64, color: Option<String>, is_bold: bool, is_italic: bool, is_underlined: bool, is_dialogue: bool }
    let (mut blocks, mut current_s_ms, mut current_e_ms, mut current_block_lines) = (Vec::<SrtBlock>::new(), 0i64, 0i64, Vec::<String>::new());

    // Helper to process individual sub-lines or full blocks to preserve dashes
    let finalize_block = |lines: &mut Vec<String>, start: i64, end: i64, blocks_vec: &mut Vec<SrtBlock>| {
        if lines.is_empty() { return; }
        let (mut sub_groups, mut current_sub) = (Vec::new(), String::new());
        
        for l in lines.iter() {
            let clean_l = l.trim_start_matches('\u{feff}').trim().to_string(); if clean_l.is_empty() { continue; }
            if clean_l.starts_with('-') || clean_l.starts_with('—') {
                if !current_sub.is_empty() { sub_groups.push(current_sub.trim().to_string()); }
                sub_groups.push(clean_l); current_sub = String::new();
            } else { if !current_sub.is_empty() { current_sub.push(' '); } current_sub.push_str(&clean_l); }
        }
        if !current_sub.is_empty() { sub_groups.push(current_sub.trim().to_string()); } lines.clear();

        for full_text in sub_groups {
            if full_text.is_empty() { continue; }
            let is_dialogue = full_text.starts_with('-') || full_text.starts_with('—');
            let (is_bold, is_italic, is_underlined) = (full_text.contains("<b>") || full_text.contains("<B>"), full_text.contains("<i>") || full_text.contains("<I>"), full_text.contains("<u>") || full_text.contains("<U>"));
            let mut color = None;
            if let Some(s_idx) = full_text.find("<font color=\"#") { if full_text[s_idx..].find("\">").is_some() { color = Some(full_text[s_idx + 14..s_idx + 20].to_uppercase()); } }

            let mut clean_text = full_text;
            while let Some(s) = clean_text.find('<') { if let Some(e) = clean_text[s..].find('>') { clean_text.replace_range(s..=s + e, ""); } else { break; } }
            clean_text = clean_text.trim().to_string();
            if !clean_text.is_empty() { blocks_vec.push(SrtBlock { text_pure: clean_text, start_ms: start, end_ms: end, color, is_bold, is_italic, is_underlined, is_dialogue }); }
        }
    };

    // STEP 1: PARSE AND GROUP SRT LINES INTO STRUCTURAL BLOCKS
    for line in c1.lines() {
        let tr = line.trim(); if tr.is_empty() { continue; }
        if tr.chars().all(|c| c.is_ascii_digit()) { finalize_block(&mut current_block_lines, current_s_ms, current_e_ms, &mut blocks); continue; }
        if tr.contains(" --> ") {
            let parts: Vec<&str> = tr.split(" --> ").collect();
            if parts.len() == 2 { current_s_ms = srt_time_to_ms(parts[0].trim()); current_e_ms = srt_time_to_ms(parts[1].trim()); }
        } else { current_block_lines.push(tr.to_string()); }
    }
    finalize_block(&mut current_block_lines, current_s_ms, current_e_ms, &mut blocks);

    // STEP 2: MERGE BLOCKS LITERALLY & APPLY STYLES WITHOUT DUPLICATION
    let (mut doc_body, mut color_table) = (String::new(), Vec::<String>::new());
    if !blocks.is_empty() {
        let (mut acc_text, mut acc_bold, mut acc_italic, mut acc_underlined, mut acc_color, mut acc_dialogue, mut last_end_ms) = 
            (blocks[0].text_pure.clone(), blocks[0].is_bold, blocks[0].is_italic, blocks[0].is_underlined, blocks[0].color.clone(), blocks[0].is_dialogue, blocks[0].end_ms);
        
        // Helper to format and append accumulated text to the final body
        let flush_accumulator = |text: &str, bold: bool, italic: bool, underlined: bool, color: &Option<String>, is_diag: bool, body: &mut String, c_table: &mut Vec<String>, force_newline: bool| {
            if text.trim().is_empty() { return; }
            let mut styled = text.trim().to_string();

            if p_style == 1 {
                if p_format == 1 {
                    if italic { styled = format!(r"\i {}\i0 ", styled); } if bold { styled = format!(r"\b {}\b0 ", styled); } if underlined { styled = format!(r"\ul {}\ul0 ", styled); }
                    if let Some(ref hex) = color {
                        if let (Ok(r), Ok(g), Ok(b)) = (i64::from_str_radix(&hex[0..2], 16), i64::from_str_radix(&hex[2..4], 16), i64::from_str_radix(&hex[4..6], 16)) {
                            let rtf_color = format!(r"\red{}\green{}\blue{};", r, g, b);
                            let c_idx = match c_table.iter().position(|c| c == &rtf_color) { Some(pos) => pos + 1, None => { c_table.push(rtf_color); c_table.len() } };
                            styled = format!(r"\cf{} {}\cf0 ", c_idx, styled);
                        }
                    }
                } else {
                    if underlined { styled = format!("_{}_", styled); } if bold { styled = format!("**{}**", styled); } if italic { styled = format!("*{}*", styled); }
                }
            }
            
            // Safe spacing logic: prevent text from gluing together across style boundaries
            if !body.is_empty() {
                if force_newline || is_diag || color.is_some() { 
                    if !body.ends_with('\n') { body.push_str("\n"); }
                } else if !body.ends_with(' ') && !body.ends_with('\n') {
                    body.push_str(" ");
                }
            }
            body.push_str(&styled);
        };

        for i in 1..blocks.len() {
            let next = &blocks[i];
            let time_break = (next.start_ms - last_end_ms) >= 3000;
            let color_break = next.color != acc_color;
            let dialogue_break = next.is_dialogue || acc_dialogue;
            let style_break = next.is_bold != acc_bold || next.is_italic != acc_italic || next.is_underlined != acc_underlined;

            if time_break || color_break || dialogue_break || style_break {
                let force_nl = color_break || dialogue_break;
                flush_accumulator(&acc_text, acc_bold, acc_italic, acc_underlined, &acc_color, acc_dialogue, &mut doc_body, &mut color_table, force_nl);
                
                if time_break && !doc_body.is_empty() { doc_body.push_str("\n\n"); }
                else if acc_color.is_some() && !doc_body.is_empty() && !doc_body.ends_with('\n') { doc_body.push_str("\n"); }
                
                acc_text = next.text_pure.clone(); acc_bold = next.is_bold; acc_italic = next.is_italic; acc_underlined = next.is_underlined; acc_color = next.color.clone(); acc_dialogue = next.is_dialogue;
            } else { 
                acc_text.push(' '); acc_text.push_str(&next.text_pure); 
            }
            last_end_ms = next.end_ms;
        }
        
        flush_accumulator(&acc_text, acc_bold, acc_italic, acc_underlined, &acc_color, acc_dialogue, &mut doc_body, &mut color_table, false);
        if acc_color.is_some() && !doc_body.ends_with('\n') { doc_body.push_str("\n"); }
    }
    doc_body = doc_body.replace("\n\n\n", "\n\n").trim().to_string();

    // STEP 3: FORMAT OUTPUT CONTENT & SECURE RTF UNICODE ENCODING (i16)
    let out_content = if p_format == 1 {
        let mut rtf_escaped = String::new();
        for c in doc_body.chars() {
            if c.is_ascii() { rtf_escaped.push(c); } 
            else { let cp = c as u32; let rtf_code = if cp <= 0x7FFF { cp as i16 } else { (cp as i32 - 0x10000) as i16 }; rtf_escaped.push_str(&format!(r"\u{}?", rtf_code)); }
        }
        let rtf_body = rtf_escaped.replace("\n\n", r"\par\par ").replace("\n", r"\par ");
        let color_tbl_str = if color_table.is_empty() { String::new() } else { format!(r"{{\colortbl;{}}}", color_table.join("")) };
        format!(r"{{\rtf1\ansi\deff0{{\fonttbl{{\f0\fnil\fcharset0 Arial;}}}}{}\f0\fs24 {}}}", color_tbl_str, rtf_body)
    } else { doc_body };
    // STEP 4: NATIVE SAVE DIALOG AND SYSTEM WRITE
    let base = Path::new(p1.as_str());
    let out_p = match ui.get_s_save_mode() {
        1 => FileDialog::new().set_file_name(&format!("{}{}", base.file_stem().unwrap().to_string_lossy(), suf)).set_directory(base.parent().unwrap()).add_filter(if p_format == 1 { "RTF" } else { "TXT" }, &[if p_format == 1 { "rtf" } else { "txt" }]).save_file(),
        2 => { let mut p = PathBuf::from(ui.get_s_custom_path().as_str()); p.push(format!("{}{}", base.file_stem().unwrap().to_string_lossy(), suf)); Some(p) },
        _ => Some(base.with_file_name(format!("{}{}", base.file_stem().unwrap().to_string_lossy(), suf))),
    }.unwrap_or_default();

    if out_p.as_os_str().is_empty() { return; }
    if out_p.exists() && ui.get_s_save_mode() != 1 { *ps.borrow_mut() = Some(PendingSave { path: out_p, content: out_content }); ui.set_show_conflict(true); } 
        else { 
        let _ = fs::write(&out_p, out_content.as_bytes()); 
        // 1. Extract the file name string securely
        let filename = out_p.file_name().unwrap().to_string_lossy().to_string();
        // 2. Call Slint's global AppState to get the localized string
        let success_msg = ui.global::<AppState>().invoke_get_save_success(filename.into());
        // 3. Update the final UI result component
        ui.set_final_result_message(success_msg); }
    return; }
    }
// --- CLEAN LOGIC ---
        if mode.starts_with("Clean_") {
            suf = "_cleaned"; fl.clear(); cnt = 0;
            if mode == "Clean_style" {
                for l in c1.lines() { let tr = l.trim(); if tr.is_empty() { continue; }
                    if tr.chars().all(|c| c.is_ascii_digit()) { cnt += 1; fl.push(cnt.to_string()); }
                    else if tr.contains(" --> ") { fl.push(tr.to_string()); }
                    else {
                        let mut clean = String::new(); let (mut in_html, mut in_curl) = (false, false);
                        for c in tr.chars() {
                            if c == '<' { in_html = true; } else if c == '>' { in_html = false; }
                            else if c == '{' { in_curl = true; } else if c == '}' { in_curl = false; }
                            else if !in_html && !in_curl { clean.push(c); } }
                        let final_txt = clean.trim().to_string(); if !final_txt.is_empty() { fl.push(final_txt); } }
            }   }
            if mode == "Clean_metadata" {
                for l in c1.lines() { let tr = l.trim(); if tr.is_empty() { continue; }
                    if tr.chars().all(|c| c.is_ascii_digit()) { cnt += 1; fl.push(cnt.to_string()); }
                    else if tr.contains(" --> ") { fl.push(tr.to_string()); }
                    else {
                        let mut clean = String::new(); let (mut in_p, mut in_b) = (false, false);
                        for c in tr.chars() {
                            if c == '(' && (m_mode == 0 || m_mode == 2) { in_p = true; }
                            else if c == ')' && (m_mode == 0 || m_mode == 2) { in_p = false; }
                            else if c == '[' && (m_mode == 1 || m_mode == 2) { in_b = true; }
                            else if c == ']' && (m_mode == 1 || m_mode == 2) { in_b = false; }
                            else if !in_p && !in_b { clean.push(c); }
                        }
                        let final_txt = clean.trim().to_string(); if !final_txt.is_empty() { fl.push(final_txt); }
            }   }   }
            if mode == "Clean_SDH" {
                let ls: Vec<&str> = c1.lines().collect();
                for (i, l) in ls.iter().enumerate() { if l.contains(" --> ") && i > 0 {
                    let mut t = Vec::new(); let mut j = i + 1;
                    while j < ls.len() && !ls[j].trim().is_empty() && !ls[j].contains(" --> ") {
                        let tr = ls[j].trim(); if !tr.chars().all(|c| c.is_ascii_digit()) {
                            let mut clean = String::new(); let (mut in_p, mut in_b, mut in_h) = (false, false, false);
                            for c in tr.chars() {
                                if c == '(' { in_p = true; } else if c == ')' { in_p = false; }
                                else if c == '[' { in_b = true; } else if c == ']' { in_b = false; }
                                else if c == '<' { in_h = true; clean.push(c); } else if c == '>' { in_h = false; clean.push(c); }
                                else if !in_p && !in_b && !in_h { clean.push(c); }
                            }
                            let mut txt = clean.trim().to_string();
                            while let Some(s_idx) = txt.find('<') { if let Some(e_idx) = txt.find('>') { if e_idx > s_idx { txt.drain(s_idx..=e_idx); } else { break; } } else { break; } }
                            let mut final_txt = txt.trim().to_string();
                            if let Some(col_idx) = final_txt.find(':') {
                                let prefix = &final_txt[..col_idx];
                                if prefix.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c == '_') { final_txt = final_txt[col_idx + 1..].trim().to_string(); }
                            }
                            if !final_txt.is_empty() { t.push(final_txt); }
                        } j += 1;
                    }
                    if !t.is_empty() { cnt += 1; fl.push(cnt.to_string()); fl.push(l.trim().to_string()); fl.push(t.join("\n")); }
            }   }   }
            if mode == "Clean_Lyrics" {
                let ls: Vec<&str> = c1.lines().collect();
                for (i, l) in ls.iter().enumerate() { if l.contains(" --> ") && i > 0 {
                    let mut t = Vec::new(); let mut j = i + 1;
                    while j < ls.len() && !ls[j].trim().is_empty() && !ls[j].contains(" --> ") {
                        let tr = ls[j].trim(); if !tr.chars().all(|c| c.is_ascii_digit()) {
                            let has_note = (l_mode == 0 || l_mode == 2) && (tr.contains('♪') || tr.contains('♫'));
                            if !has_note {
                                let mut clean = tr.to_string();
                                if (l_mode == 1 || l_mode == 2) && clean.contains('#') && !clean.contains("color=\"#") && !clean.contains("color=#") {
                                    let parts: Vec<&str> = clean.split('#').collect();
                                    let mut rebuilt = String::new();
                                    for (idx, part) in parts.iter().enumerate() {
                                        if idx % 2 == 0 { rebuilt.push_str(part); } }
                                    clean = rebuilt;}
                                let final_txt = clean.trim().to_string(); if !final_txt.is_empty() { t.push(final_txt); }
                            }   }
                        j += 1; }
                    if !t.is_empty() { cnt += 1; fl.push(cnt.to_string()); fl.push(l.trim().to_string()); fl.push(t.join("\n"));  } }
            } }
        }
// --- S_SAVE_MODE ---
            let base = Path::new(p1.as_str());
            let out_n = if mode == "Convert_PlainText" {
                format!("{}{}", base.file_stem().unwrap().to_string_lossy(), suf)
            }
            else {
                format!("{}{}.srt", base.file_stem().unwrap().to_string_lossy(), suf)};
            let out_p = match ui.get_s_save_mode() {
                1 => {
                    let mut dialog = FileDialog::new().set_file_name(&out_n).set_directory(base.parent().unwrap());
                    if mode == "Convert_PlainText" {
                        if suf.ends_with(".rtf") { dialog = dialog.add_filter("RTF Document", &["rtf"]); }
                        else { dialog = dialog.add_filter("Text File", &["txt"]); }
                    } else {
                        dialog = dialog.add_filter("SRT", &["srt"]);}
                    dialog.save_file()},
                2 => { let mut p = PathBuf::from(ui.get_s_custom_path().as_str()); p.push(&out_n); Some(p) },
                _ => Some(base.with_file_name(&out_n)),
            }.unwrap_or_default();
            if out_p.as_os_str().is_empty() { return; }

            let mut out_s = String::new();
            if mode == "Convert_PlainText" {
                // special for Convert Plaintext
                if let Some(content) = fl.first() {
                    out_s.push_str(content);}
            } else {
                // for all srt
                for (i, l) in fl.iter().enumerate() { 
                    if i > 0 && l.chars().all(|c| c.is_ascii_digit()) { out_s.push_str(LINE_ENDING); } 
                    out_s.push_str(l); out_s.push_str(LINE_ENDING);}
            }
            if out_p.exists() && ui.get_s_save_mode() != 1 { 
                *ps.borrow_mut() = Some(PendingSave { path: out_p, content: out_s }); 
                ui.set_show_conflict(true); 
            } else { 
                let _ = fs::write(&out_p, out_s); 
                // 1. Extract the file name as a clean string
                let filename = out_p.file_name().unwrap().to_string_lossy().to_string();
                // 2. Query Slint's translation engine dynamically
                let success_msg = ui.global::<AppState>().invoke_get_save_success(filename.into());
                // 3. Display the localized text
                ui.set_final_result_message(success_msg);
            }
        }
    });
    ui.run()
}
