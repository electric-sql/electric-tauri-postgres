// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
extern crate lazy_static;

mod tauri_postgres;
mod utils;

use tauri::{Manager, State};
// Tauri plug-ins
use tauri_plugin_log::{
    fern::colors::{Color, ColoredLevelConfig},
    LogTarget,
};

use std::{mem::{MaybeUninit, replace}, sync::Arc};

use pg_embed::postgres::PgEmbed;
use sqlx::PgConnection;
use tauri_postgres::{tauri_pg_connect, tauri_pg_query, tauri_pg_fill_example_data_sync};
use tauri_postgres::{tauri_pg_setup, tauri_pg_init_database, tauri_pg_query_sync};

// #[tokio::main]
// async fn create_database_connection() -> (PgEmbed, PgConnection) {
//     return tauri_pg_connect().await;
// }

// static mut PG: Arc<PgEmbed>;

// struct PgSingleton {
//     pg_embed: Option<PgEmbed>,
// }
// impl PgSingleton {
//     fn take_pg_singleton(&mut self) -> Option<PgEmbed> {
//         let p = replace(&mut self.pg_embed, None);
//         p
//     }
// }
// static mut PG: PgEmbed = tauri_pg_init_database_sync();
// static mut CONN: PgConnection = tauri_pg_connect_sync(unsafe { PG }, "test");

struct GlobalPG(PgEmbed);

#[tokio::main]
pub async fn tauri_pg_init_database_sync() -> PgEmbed {
    return tauri_pg_init_database().await;
}

#[tokio::main]
pub async fn tauri_pg_connect_sync(pg: &PgEmbed, db_name: &str) -> PgConnection {
    return tauri_pg_connect(pg, db_name).await;
}

// lazy_static! {
//     static ref PG: PgEmbed = tauri_pg_init_database_sync();
//     static ref CONN: PgConnection = tauri_pg_connect_sync(PG, "test");
// }

//  PG: (mut PgEmbed, mut PgConnection) = create_database_connection();
// static mut CONN: Option<PgConnection> = None;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tokio::main]
#[tauri::command]
async fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// #[tokio::main]
// #[tauri::command]
// async fn send_recv_postgres<'r>(state: State<'r, GlobalPG>, data: &str) -> String {
//     println!("{}", data);
//     // let (mut pg, mut conn) = connect_postgres().await;
//     // return send_to_postgres(&mut conn, data).await;
//     let pg = &state.inner().0;
//     let conn = tauri_pg_connect_sync(pg, "test");
//     return tauri_pg_query(conn, data).await;
// }

#[tokio::main]
#[tauri::command]
async fn send_recv_postgres<'r>(state: State<'r, GlobalPG>, data: &str) -> String {
    println!("{}", data);
    // let (mut pg, mut conn) = connect_postgres().await;
    // return send_to_postgres(&mut conn, data).await;
    let pg = &state.inner().0;
    let conn = tauri_pg_connect(pg, "test").await;

    tauri_pg_query(conn, data).await
}

fn main() {
    // unsafe {
    // (PG, CONN) = Some(connect_postgres().await);
    // }
    let mut log = tauri_plugin_log::Builder::default()
        .targets([
            // LogTarget::LogDir,
            // LOG PATH: ~/.chatgpt/ChatGPT.log
            LogTarget::Folder(utils::app_root()),
            LogTarget::Stdout,
            LogTarget::Webview,
        ])
        .level(log::LevelFilter::Debug);

    let mut builder = tauri::Builder::default()
      .setup(|app| {
        app.manage(GlobalPG(tauri_pg_init_database_sync()));
        // read the `GlobalPG` managed state with the turbofish syntax
        // let pg = app.state::<GlobalPG>();
        // let conn = tauri_pg_connect_sync(&pg.inner().0, "test");
        // tauri_pg_fill_example_data_sync(conn);
        // read the `GlobalPG` managed state with the `State` guard
        // let pg: State<GlobalPG> = app.state();
        Ok(())
      })
        .plugin(log.build())
        .invoke_handler(tauri::generate_handler![greet, send_recv_postgres])
        .on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                std::process::exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
