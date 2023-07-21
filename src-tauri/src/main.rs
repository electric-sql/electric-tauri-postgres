// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tauri_postgres;
mod utils;

// General
use log::debug;
use pg_embed::postgres::PgEmbed;
use sqlx::PgConnection;

// Tauri
use tauri::{Manager, State};

// Tauri plug-ins
use tauri_plugin_log::{
    fern::colors::{Color, ColoredLevelConfig},
    LogTarget,
};

// This package
use tauri_postgres::{
    tauri_pg_connect, tauri_pg_fill_example_data_sync, tauri_pg_init_database, tauri_pg_query,
};

// Postgres console
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::{
    io::{BufRead, BufReader, Write},
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::Duration,
};
use tauri::async_runtime::Mutex as AsyncMutex;

// Tauri globals
/// This is the global connection to Postgres
struct GlobalPG(PgEmbed);

/// App state for the terminal
struct AppState {
    pty_pair: Arc<AsyncMutex<PtyPair>>,
    writer: Arc<AsyncMutex<Box<dyn Write + Send>>>,
}

// Tauri commands
#[tauri::command]
async fn async_write_to_pty(data: &str, state: State<'_, AppState>) -> Result<(), ()> {
    write!(state.writer.lock().await, "{}", data).map_err(|_| ())
}

#[tauri::command]
async fn async_resize_pty(rows: u16, cols: u16, state: State<'_, AppState>) -> Result<(), ()> {
    state
        .pty_pair
        .lock()
        .await
        .master
        .resize(PtySize {
            rows,
            cols,
            ..Default::default()
        })
        .map_err(|_| ())
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tokio::main]
#[tauri::command]
async fn send_recv_postgres(state: State<GlobalPG>, data: &str) -> String {
    debug!("{}", data);

    let pg = &state.inner().0;
    let conn = tauri_pg_connect(pg, "test").await;

    tauri_pg_query(conn, data).await
}

/// TODO: A special method to test sending to the terminal. This should use the normal method.
#[tokio::main]
#[tauri::command]
async fn send_recv_postgres_terminal(state: State<GlobalPG>, data: &str) -> String {
    debug!("From the terminal, {}", data);

    let pg = &state.inner().0;
    let conn = tauri_pg_connect(pg, "test").await;

    let result = tauri_pg_query(conn, data).await;

    result.into()
}

#[tokio::main]
pub async fn tauri_pg_init_database_sync() -> PgEmbed {
    return tauri_pg_init_database().await;
}

#[tokio::main]
pub async fn tauri_pg_connect_sync(pg: &PgEmbed, db_name: &str) -> PgConnection {
    return tauri_pg_connect(pg, db_name).await;
}

fn main() {
    let log = tauri_plugin_log::Builder::default()
        .targets([
            // LogTarget::LogDir,
            // LOG PATH: ~/.chatgpt/ChatGPT.log
            LogTarget::Folder(utils::app_root()),
            LogTarget::Stdout,
            LogTarget::Webview,
        ])
        .level(log::LevelFilter::Debug);

    // Setup the postgres terminal
    let pty_system = native_pty_system();

    let pty_pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();

    #[cfg(target_os = "windows")]
    let cmd = CommandBuilder::new("powershell.exe");
    #[cfg(not(target_os = "windows"))]
    let cmd = CommandBuilder::new("bash");

    let mut child = pty_pair.slave.spawn_command(cmd).unwrap();

    thread::spawn(move || {
        child.wait().unwrap();
    });

    let reader = pty_pair.master.try_clone_reader().unwrap();
    let writer = pty_pair.master.take_writer().unwrap();

    let reader = Arc::new(Mutex::new(Some(BufReader::new(reader))));

    // Start the app
    tauri::Builder::default()
        .on_page_load(move |window, _| {
            let window = window.clone();
            let reader = reader.clone();

            thread::spawn(move || {
                let reader = reader.lock().unwrap().take();
                if let Some(mut reader) = reader {
                    loop {
                        sleep(Duration::from_millis(1));
                        let data = reader.fill_buf().unwrap().to_vec();
                        reader.consume(data.len());
                        if data.len() > 0 {
                            window.emit("data", data).unwrap();
                        }
                    }
                }
            });
        })
        .setup(|app| {
            app.manage(GlobalPG(tauri_pg_init_database_sync()));

            // terminal
            tauri::WindowBuilder::new(
                app,
                "postgresterminal", /* must be unique */
                tauri::WindowUrl::App("debug.html".into()),
            )
            .build()?;

            // read the `GlobalPG` managed state with the turbofish syntax
            // let pg = app.state::<GlobalPG>();
            // let conn = tauri_pg_connect_sync(&pg.inner().0, "test");
            // tauri_pg_fill_example_data_sync(conn);
            // read the `GlobalPG` managed state with the `State` guard
            // let pg: State<GlobalPG> = app.state();
            Ok(())
        })
        .plugin(log.build())
        .manage(AppState {
            pty_pair: Arc::new(AsyncMutex::new(pty_pair)),
            writer: Arc::new(AsyncMutex::new(writer)),
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            send_recv_postgres,
            async_write_to_pty,
            async_resize_pty,
            send_recv_postgres_terminal
        ])
        .on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                std::process::exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
