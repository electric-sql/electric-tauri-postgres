use env_logger::Env;
use pg_embed::pg_enums::PgAuthMethod;
use pg_embed::pg_errors::{PgEmbedError, PgEmbedErrorType};
use pg_embed::pg_fetch::{PgFetchSettings, PG_V15};
use pg_embed::postgres::{PgEmbed, PgSettings};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use sqlx::postgres::PgRow;
use sqlx::{Column, Row, ValueRef};
use sqlx::{Connection, PgConnection};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

pub async fn tauri_pg_setup(
    port: u16,
    database_dir: PathBuf,
    persistent: bool,
    migration_dir: Option<PathBuf>,
) -> Result<PgEmbed, PgEmbedError> {
    let _ = env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .is_test(true)
        .try_init();
    let pg_settings = PgSettings {
        database_dir,
        port,
        user: "postgres".to_string(),
        password: "password".to_string(),
        auth_method: PgAuthMethod::MD5,
        persistent,
        timeout: Some(Duration::from_secs(10)),
        migration_dir,
    };
    let fetch_settings = PgFetchSettings {
        version: PG_V15,
        ..Default::default()
    };
    let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;
    pg.setup().await?;
    Ok(pg)
}

pub async fn tauri_pg_init_database() -> PgEmbed {
    let database_dir = PathBuf::from("/home/iib/data_test/db");
    let mut pg: PgEmbed = tauri_pg_setup(5432, database_dir, true, None)
        .await
        .expect("PgEmbed should not fail here");

    pg.start_db().await.expect("start_db should not fail here");
    let db_name = "test";
    if !pg
        .database_exists(&db_name)
        .await
        .expect("The check should not fail here")
    {
        pg.create_database(&db_name)
            .await
            .expect("create_database should not fail here");
    };
    pg.migrate(&db_name)
        .await
        .expect("migrate should not fail here");

    pg
}

pub async fn tauri_pg_connect(pg: &PgEmbed, db_name: &str) -> PgConnection {
    // let _ = pg.setup().await;
    let db_uri = pg.full_db_uri(&db_name);
    let mut conn = PgConnection::connect(&db_uri)
        .await
        .map_err(|_| PgEmbedError {
            error_type: PgEmbedErrorType::SqlQueryError,
            source: None,
            message: None,
        })
        .expect("PgConnection should not fail here");

    // let _ = sqlx::query("CREATE TABLE IF NOT EXISTS testing (id BIGSERIAL PRIMARY KEY, description TEXT NOT NULL, done BOOLEAN NOT NULL DEFAULT FALSE)")
    // .execute(&mut conn)
    // .await
    // .map_err(|_| PgEmbedError {
    //     error_type: PgEmbedErrorType::SqlQueryError,
    //     source: None,
    //     message: None,
    // }).expect("This query should not fail");

    // let _ = sqlx::query("INSERT INTO testing (description) VALUES ('Hello')")
    //     .execute(&mut conn)
    //     .await
    //     .map_err(|_| PgEmbedError {
    //         error_type: PgEmbedErrorType::SqlQueryError,
    //         source: None,
    //         message: None,
    //     }).expect("This query should not fail");

    conn
}

#[tokio::main]
pub async fn tauri_pg_fill_example_data_sync(mut conn: PgConnection) {
    let _ = sqlx::query("CREATE TABLE IF NOT EXISTS testing (id BIGSERIAL PRIMARY KEY, description TEXT NOT NULL, done BOOLEAN NOT NULL DEFAULT FALSE)")
    .execute(&mut conn)
    .await
    .map_err(|_| PgEmbedError {
        error_type: PgEmbedErrorType::SqlQueryError,
        source: None,
        message: None,
    }).expect("This query should not fail");
}

pub async fn send_recv_postgres_loop(pg: &mut PgConnection) {
    let mut rl = DefaultEditor::new().expect("DefaultEditor to be instantiated");
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }
    loop {
        let readline = rl.readline("postgres> ");
        match readline {
            Ok(line) => {
                if rl.add_history_entry(line.as_str()).is_err() {
                    eprintln!("Adding to history failed")
                };
                let rows = match sqlx::query(line.as_ref())
                    .fetch_all(&mut *pg)
                    .await
                    .map_err(|_| PgEmbedError {
                        error_type: PgEmbedErrorType::SqlQueryError,
                        source: None,
                        message: None,
                    }) {
                    Ok(rows) => rows,
                    Err(error) => {
                        eprintln!("Problem with the statement: {:?}", error);
                        let rows: Vec<PgRow> = vec![];

                        rows
                    }
                };
                for row in rows {
                    println!("{:?}", row_to_json(row));
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    if rl.save_history("history.txt").is_err() {
        println!("Saving history should not fail");
    }
}

// https://stackoverflow.com/a/72904564
fn row_to_json(row: PgRow) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for col in row.columns() {
        let value = row.try_get_raw(col.ordinal()).unwrap();
        let value = match value.is_null() {
            true => "NULL".to_string(),
            false => value.as_str().unwrap().to_string(),
        };
        result.insert(col.name().to_string(), value);
    }

    result
}

pub async fn tauri_pg_query(mut conn: PgConnection, line: &str) -> String {
    let rows = match sqlx::query(line.as_ref())
        .fetch_all(&mut conn)
        .await
        .map_err(|_| PgEmbedError {
            error_type: PgEmbedErrorType::SqlQueryError,
            source: None,
            message: None,
        }) {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("Problem with the statement: {:?}", error);
            return error.to_string();
        }
    };

    let mut result = String::new().to_owned();
    for row in rows {
        let row_column = row_to_json(row);
        for (key, value) in row_column {
            result.push_str(key.as_str());
            result.push_str("|".as_ref());
            result.push_str(value.as_str());
        }
    }

    return result;
}

#[tokio::main]
pub async fn tauri_pg_query_sync(mut conn: PgConnection, line: &str) -> String {
    let rows = match sqlx::query(line.as_ref())
        .fetch_all(&mut conn)
        .await
        .map_err(|_| PgEmbedError {
            error_type: PgEmbedErrorType::SqlQueryError,
            source: None,
            message: None,
        }) {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("Problem with the statement: {:?}", error);
            return error.to_string();
        }
    };

    let mut result = String::new().to_owned();
    for row in rows {
        let row_column = row_to_json(row);
        for (key, value) in row_column {
            result.push_str(key.as_str());
            result.push_str("|".as_ref());
            result.push_str(value.as_str());
        }
    }

    return result;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let (mut pg, mut conn) = tauri_pg_connect().await;
    // send_recv_postgres_loop(&mut conn).await;
    // pg.pg_settings.timeout = Some(Duration::from_millis(10));
    // pg.start_db().await.err().map(|e| e.message).flatten();
    Ok(())
}
