mod helper;
mod verificator;

use std::{collections::HashMap, sync::Arc};
use helper::{handle_client , Store,  clean_expired_keys};

use tokio::{
    net::{TcpListener},
    sync::Mutex,
    time::{interval, Duration},
};
use tracing::{error, info};

//pour les requestses

#[tokio::main]
async fn main() {
    // Initialiser tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    //consignes
    {
        //strategy pour PINg et SET, avoir le meme struct tuple
        //expire semble close to PING et set

        // TODO: Implémenter le serveur MiniRedis sur 127.0.0.1:7878
        //
        // Étapes suggérées :
        // 1. Créer le store partagé (Arc<Mutex<HashMap<String, ...>>>)
        // 2. Bind un TcpListener sur 127.0.0.1:7878
        // 3. Accept loop : pour chaque connexion, spawn une tâche
        // 4. Dans chaque tâche : lire les requêtes JSON ligne par ligne,
        //    traiter la commande, envoyer la réponse JSON + '\n'
    }   
    
    println!("MiniRedis - En");

    let store: Store = Arc::new(Mutex::new(HashMap::new()));

    // Petit robot ménager : il enlève les clés expirées de temps en temps.
    let cleanup_store = Arc::clone(&store);
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));

        loop {
            ticker.tick().await;
            clean_expired_keys(&cleanup_store).await;
        }
    });
    
    
    let listener = TcpListener::bind("127.0.0.1:7878")
        .await
        .expect("impossible de démarrer le serveur sur 127.0.0.1:7878; port utilisé ??");

    info!("Serveur MiniRedis démare sur 127.0.0.1:7878");

    // Getrstion dur workflow
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                let store = Arc::clone(&store);

                tokio::spawn(async move {
                    info!("Nouveau client connecté: {}", addr);

                    if let Err(err) = handle_client(socket, store).await {
                        error!("Erreur avec le client {}: {}", addr, err);
                    }

                    info!("Client déconnecté: {}", addr);
                });
            }
            Err(err) => {
                error!("Erreur pendant accept(): {}", err);
            }
        }
    }
}
