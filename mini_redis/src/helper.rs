use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};


use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::Mutex,
};

pub type Store = Arc<Mutex<HashMap<String, String>>>;

// TODO modifiying reuquest
#[derive(Debug, Deserialize)]
pub struct Request {
    pub cmd: String,
    pub key: Option<String>,
    pub value: Option<String>,
}

pub async fn handle_client(socket: TcpStream, store: Store) -> std::io::Result<()> {
    let (reader_half, mut writer_half) = socket.into_split();
    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();

    loop {
        line.clear();

        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }

        let response = match serde_json::from_str::<Request>(line.trim()) {
            Ok(request) => process_request(request, &store).await,
            Err(_) => json!({
                "status": "error",
                "message": "invalid json"
            }),
        };

        writer_half.write_all(response.to_string().as_bytes()).await?;
        writer_half.write_all(b"\n").await?;
    }

    Ok(())
}

pub async fn process_request(request: Request, store: &Store) -> Value {
    match request.cmd.as_str() {
        // PING et EXpire partagera les structures
        "PING" => json!({ "status": "ok" }),
        "SET" => {
            let Some(key) = request.key else {
                return json!({
                    "status": "error",
                    "message": "missing key"
                });
            };

            let Some(value) = request.value else {
                return json!({
                    "status": "error",
                    "message": "missing value"
                });
            };

            let mut map = store.lock().await;
            map.insert(key, value);

            json!({ "status": "ok" })
        }
        "GET" => {
            let Some(key) = request.key else {
                return json!({
                    "status": "error",
                    "message": "missing key"
                });
            };

            let map = store.lock().await;
            let value = map.get(&key).cloned();

            json!({
                "status": "ok",
                "value": value
            })
        }
        "DEL" => {
            
            // Provisoir
            let Some(key) = request.key else {
                return json!({
                    "status": "error",
                    "message": "missing key"
                });
            };

            let mut map = store.lock().await;
            let mut count:usize = 0;
            let value = map.remove(&key) ;
            if let Some(_) = value {
                count = 1
            }
            //let value = map.get(&key).cloned();
            json!({
                "status": "ok",
                "count": count
            })
        }
        
        _ => json!({
            "status": "error",
            "message": "une commande Inconnu!"
        }),
    }
}