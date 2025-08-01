#![allow(unused_variables, unused_mut, dead_code)]
// Import everything from the library crate instead of declaring separate modules
use deemak::DEBUG_MODE;
use deemak::gui_shell::run_gui_loop;
use deemak::metainfo::valid_sekai::validate_or_create_sekai;
use deemak::rns::restore_comp;
use deemak::utils::globals::set_world_dir;
use deemak::utils::{debug_mode, find_root, log};
use raylib::ffi::{SetConfigFlags, SetTargetFPS};
use raylib::prelude::get_monitor_width;

pub const HELP_TXT: &str = r#"
Usage: deemak <sekai_directory> [--debug] [--web]

Options:
  <sekai_directory> [Required]  :   Path to the Sekai directory to parse.
  --debug [Optional]            :   Enable debug mode for more verbose logging.
  --web [Optional]              :   Run the application in web mode (requires a web server).
"#;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // first argument is sekai name to parse
    DEBUG_MODE
        .set(args.iter().any(|arg| arg == "--debug"))
        .expect("DEBUG_MODE already set");
    unsafe {
        if DEBUG_MODE.get().unwrap_or(&false) == &true {
            std::env::set_var("RUST_BACKTRACE", "1");
        }
    }
    log::log_info("Application", "Starting DEEMAK Shell");

    let sekai_dir = if args.len() > 1 {
        // get absolute path to the sekai directory
        let sekai_path = std::env::current_dir().unwrap().join(&args[1]);
        log::log_info(
            "SEKAI",
            &format!("Sekai directory provided: {sekai_path:?}"),
        );

        if !validate_or_create_sekai(&sekai_path, true) {
            log::log_error(
                "SEKAI",
                &format!(
                    "Sekai directory is not valid. Creating default `.dir_info` at {sekai_path:?}"
                ),
            );
        }
        // Just check first for HOME directory validity and create if not.
        let root_dir;
        match find_root::find_home(&sekai_path) {
            Ok(Some(sekai_dir)) => {
                log::log_info(
                    "SEKAI",
                    &format!("Found root directory for Sekai: {}", sekai_dir.display()),
                );
                // Set the global Sekai directory
                root_dir = Some(sekai_dir.clone());
                set_world_dir(sekai_dir);
            }
            Ok(None) => {
                log::log_error(
                    "SEKAI",
                    "Failed to find root directory for Sekai. No HOME location found. Exiting.",
                );
                eprintln!("Error: Failed to find root directory for Sekai. Exiting.");
                return;
            }
            Err(e) => {
                log::log_error(
                    "SEKAI",
                    &format!("Process failed while finding Sekai HOME. Error: {e}. Exiting."),
                );
                eprintln!("Process failed while finding Sekai HOME. Error: {e}. Exiting.");
                return;
            }
        }
        // If not valid, create .dir_info for each of them.
        if !validate_or_create_sekai(&sekai_path, false) {
            log::log_error(
                "SEKAI",
                &format!(
                    "Sekai directory is not valid even after creating default `.dir_info`. Sekai: {sekai_path:?}"
                ),
            );
            eprintln!(
                "Error: Sekai directory is not valid even after creating default `.dir_info`. Please check the sekai validity. Sekai: {sekai_path:?}"
            );
            return;
        } else {
            // sekai is valid
            log::log_info("SEKAI", &format!("Sekai is Valid {sekai_path:?}"));

            // Create the restore file if it doesn't exist, since it is required for restoring. The
            // progress will be saved as `save_me` and will be recreated every run.
            log::log_info(
                "SEKAI",
                &format!(
                    "Creating restore file for Sekai at {:?}",
                    sekai_path.join(".dir_info/restore_me")
                ),
            );
            // restore_me should be made initially if it doesnt exist, else it will not be created
            match restore_comp::backup_sekai("restore", root_dir.as_ref().unwrap()) {
                Err(e) => {
                    log::log_error("SEKAI", &format!("Failed to create restore file: {e}"));
                    eprintln!(
                        "Error: Failed to create restore file: {e}
Continuing..."
                    );
                    return;
                }
                Ok(msg) => {
                    log::log_info("SEKAI", &msg);
                }
            }

            // save_me should be made initially if it doesnt exist, it will be created every run
            log::log_info(
                "SEKAI",
                &format!(
                    "Creating save file for Sekai at {:?}",
                    sekai_path.join(".dir_info/save_me")
                ),
            );
            match restore_comp::backup_sekai("save", root_dir.as_ref().unwrap()) {
                Err(e) => {
                    log::log_error("SEKAI", &format!("Failed to create save file: {e}"));
                    eprintln!(
                        "Error: Failed to create save file: {e}
Continuing..."
                    );
                    return;
                }
                Ok(msg) => {
                    log::log_info("SEKAI", &msg);
                }
            }
        }
        Some(sekai_path)
    } else {
        // args.len() == 1
        log::log_error("Application", "Invalid arguments provided.");
        eprintln!("Error: At least one argument is required.");
        println!("{HELP_TXT}");
        return;
    };

    // If `save_me` already exists, then the sekai will be restored from it.
    match restore_comp::restore_sekai("save", &sekai_dir.clone().unwrap()) {
        Err(err) => {
            log::log_error(
                "SEKAI",
                &format!("Failed to restore Sekai from save file: {err}"),
            );
            eprintln!(
                "Error: Failed to restore Sekai from save file at {sekai_dir:?}
Continuing..."
            );
        }
        Ok(_) => {
            log::log_info("SEKAI", "Sekai restored successfully from save file");
        }
    }

    // NOTE: All Directory operations and variables settings should be done before this point.
    //
    // We have 2 modes, the web and the raylib gui. The web argument runs it on the web, else
    // raylib gui is set by default.
    //
    // NOTE: #############    SERVER USAGE    #############
    //
    // Initialize the server if --web argument is provided
    if args.iter().any(|arg| arg == "--web") {
        log::log_info("Application", "Running in web mode");
        // server::launch_web(sekai_dir.clone().unwrap());
        let _ = deemak::server::server();
        return;
    }

    // NOTE: #############    RAYLIB GUI USAGE    #############
    //
    // Initialize Raylib window
    unsafe {
        SetConfigFlags(4);
        SetTargetFPS(60);
    }
    let loglevel = if !debug_mode() {
        raylib::consts::TraceLogLevel::LOG_ERROR
    } else {
        raylib::consts::TraceLogLevel::LOG_ALL
    };

    let (mut rl, thread) = raylib::init()
        .log_level(loglevel)
        .size(800, 600)
        .title("DEEMAK Shell")
        .build();
    let font_size = get_monitor_width(0) as f32 / 73.5;
    rl.set_trace_log(loglevel);
    // Disable escape key exit to prevent accidental application closure
    unsafe {
        raylib::ffi::SetExitKey(0i32);
    }
    log::log_info("Application", "DEEMAK initialized successfully");

    // Show login screen before menu
    if !deemak::login::show_login(&mut rl, &thread, font_size) {
        log::log_info("Application", "Login aborted by user.");
        return; // Exit if window closed during login
    }

    // Run the GUI loop
    run_gui_loop(&mut rl, &thread, sekai_dir.unwrap(), font_size);
}
