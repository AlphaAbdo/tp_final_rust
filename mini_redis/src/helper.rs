use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Instant};
use tokio::fs;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::Mutex,
    time::{Duration},
};

pub type Store = Arc<Mutex<HashMap<String, Entry>>>;

#[derive(Debug, Clone)]
pub struct Entry {
    value: String,
    expires_at: Option<Instant>,
}


// TODO modifiying reuquest
#[derive(Debug, Deserialize)]
pub struct Request {
    pub cmd: String,
    pub key: Option<String>,
    pub value: Option<String>,
    pub seconds: Option<u64>,
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
            // Nouvelle clé-valeur ajoutée au store
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
            map.insert(
                key,
                Entry {
                    value,
                    expires_at: None,
                },
            );

            json!({ "status": "ok" })
        }
        "GET" => {
            let Some(key) = request.key else {
                return json!({
                    "status": "error",
                    "message": "missing key"
                });
            };

            let mut map = store.lock().await;

            // Si la clé est périmée, pouf, elle disparaît.
            if is_key_expired(&map, &key) {
                map.remove(&key);
            }

            let value = map.get(&key).map(|entry| entry.value.clone());

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
        "KEYS" => {
            // Retour toutes les clés non-expirées
            let map = store.lock().await;
            let now = Instant::now();
            let keys: Vec<String> = map
                .iter()
                .filter(|(_, entry)| {
                    match entry.expires_at {
                        Some(expires_at) => expires_at > now,
                        None => true,
                    }
                })
                .map(|(k, _)| k.clone())
                .collect();

            json!({
                "status": "ok",
                "keys": keys
            })
        }
        "EXPIRE" => {
            // Fixer une deadline d'expiration pour la key
            let Some(key) = request.key else {
                return json!({
                    "status": "error",
                    "message": "missing key"
                });
            };

            let Some(seconds) = request.seconds else {
                return json!({
                    "status": "error",
                    "message": "missing seconds"
                });
            };

            let mut map = store.lock().await;
            if let Some(entry) = map.get_mut(&key) {
                entry.expires_at = Some(Instant::now() + Duration::from_secs(seconds));
                json!({ "status": "ok" })
            } else {
                json!({
                    "status": "error",
                    "message": "key not found"
                })
            }
        }
        "TTL" => {
            let Some(key) = request.key else {
                return json!({
                    "status": "error",
                    "message": "missing key"
                });
            };

            let map = store.lock().await;
            match map.get(&key) {
                Some(entry) => {
                    let ttl = match entry.expires_at {
                        Some(expires_at) => {
                            let now = Instant::now();
                            if expires_at > now {
                                (expires_at - now).as_secs() as i64
                            } else {
                                -2 // expired
                            }
                        }
                        None => -1, // no expiration
                    };
                    json!({
                        "status": "ok",
                        "ttl": ttl
                    })
                }
                None => json!({
                    "status": "ok",
                    "ttl": -2
                }),
            }
        }
        "INCR" => {
            let Some(key) = request.key else {
                return json!({
                    "status": "error",
                    "message": "missing key"
                });
            };

            let mut map = store.lock().await;
            
            if let Some(entry) = map.get_mut(&key) {
                match entry.value.parse::<i64>() {
                    Ok(num) => {
                        let new_val = num + 1;
                        entry.value = new_val.to_string();
                        json!({
                            "status": "ok",
                            "value": new_val
                        })
                    }
                    Err(_) => json!({
                        "status": "error",
                        "message": "not an integer"
                    }),
                }
            } else {
                map.insert(
                    key,
                    Entry {
                        value: "1".to_string(),
                        expires_at: None,
                    },
                );
                json!({
                    "status": "ok",
                    "value": 1
                })
            }
        }
        "DECR" => {
            let Some(key) = request.key else {
                return json!({
                    "status": "error",
                    "message": "missing key"
                });
            };

            let mut map = store.lock().await;
            
            if let Some(entry) = map.get_mut(&key) {
                match entry.value.parse::<i64>() {
                    Ok(num) => {
                        let new_val = num - 1;
                        entry.value = new_val.to_string();
                        json!({
                            "status": "ok",
                            "value": new_val
                        })
                    }
                    Err(_) => json!({
                        "status": "error",
                        "message": "not an integer"
                    }),
                }
            } else {
                map.insert(
                    key,
                    Entry {
                        value: "-1".to_string(),
                        expires_at: None,
                    },
                );
                json!({
                    "status": "ok",
                    "value": -1
                })
            }
        }
        "SAVE" => {
            let map = store.lock().await;
            let mut dump = serde_json::Map::new();
            
            for (key, entry) in map.iter() {
                dump.insert(key.clone(), Value::String(entry.value.clone()));
            }
            
            let json_data = serde_json::to_string_pretty(&dump).unwrap_or_default();
            
            // Écriture asynchrone du fichier (marche)
            match fs::write("dump.json", json_data).await {
                Ok(_) => json!({ "status": "ok" }),
                Err(_) => json!({
                    "status": "error",
                    "message": "failed to save"
                }),
            }
        }
        
        _ => json!({
            "status": "error",
            "message": "une commande Inconnu!"
        }),
    }
}


pub async fn clean_expired_keys(store: &Store) {
    // Tâche de nettoyage périodique - remover les keys périmées
    let mut map = store.lock().await;
    remove_expired_entries(&mut map);
}

pub fn remove_expired_entries(map: &mut HashMap<String, Entry>) {
    let now = Instant::now();
    map.retain(|_, entry| match entry.expires_at {
        Some(expires_at) => expires_at > now,
        None => true,
    });
}

pub fn is_key_expired(map: &HashMap<String, Entry>, key: &str) -> bool {
    match map.get(key).and_then(|entry| entry.expires_at) {
        Some(expires_at) => expires_at <= Instant::now(),
        None => false,
    }
}