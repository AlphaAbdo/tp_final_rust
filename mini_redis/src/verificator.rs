#[cfg(test)]
mod tests {
    use crate::helper::{process_request, Request};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_set_get() {
        let store = Arc::new(Mutex::new(HashMap::new()));
        let req = Request {
            cmd: "SET".to_string(),
            key: Some("test_key".to_string()),
            value: Some("test_value".to_string()),
            seconds: None,
        };
        
        let resp = process_request(req, &store).await;
        assert_eq!(resp["status"], "ok");

        let req = Request {
            cmd: "GET".to_string(),
            key: Some("test_key".to_string()),
            value: None,
            seconds: None,
        };
        
        let resp = process_request(req, &store).await;
        assert_eq!(resp["status"], "ok");
        assert_eq!(resp["value"], "test_value");
    }

    #[tokio::test]
    async fn test_del() {
        let store = Arc::new(Mutex::new(HashMap::new()));
        
        // Set a value
        let req = Request {
            cmd: "SET".to_string(),
            key: Some("del_key".to_string()),
            value: Some("val".to_string()),
            seconds: None,
        };
        process_request(req, &store).await;

        // Delete it
        let req = Request {
            cmd: "DEL".to_string(),
            key: Some("del_key".to_string()),
            value: None,
            seconds: None,
        };
        
        let resp = process_request(req, &store).await;
        assert_eq!(resp["status"], "ok");
        assert_eq!(resp["count"], 1);
    }

    #[tokio::test]
    async fn test_keys() {
        let store = Arc::new(Mutex::new(HashMap::new()));
        
        // Add some keys
        for i in 0..3 {
            let req = Request {
                cmd: "SET".to_string(),
                key: Some(format!("key{}", i)),
                value: Some("val".to_string()),
                seconds: None,
            };
            process_request(req, &store).await;
        }

        let req = Request {
            cmd: "KEYS".to_string(),
            key: None,
            value: None,
            seconds: None,
        };
        
        let resp = process_request(req, &store).await;
        assert_eq!(resp["status"], "ok");
        assert_eq!(resp["keys"].as_array().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn test_ttl() {
        let store = Arc::new(Mutex::new(HashMap::new()));
        
        let req = Request {
            cmd: "SET".to_string(),
            key: Some("ttl_key".to_string()),
            value: Some("val".to_string()),
            seconds: None,
        };
        process_request(req, &store).await;

        // Check TTL on key without expiration
        let req = Request {
            cmd: "TTL".to_string(),
            key: Some("ttl_key".to_string()),
            value: None,
            seconds: None,
        };
        
        let resp = process_request(req, &store).await;
        assert_eq!(resp["status"], "ok");
        assert_eq!(resp["ttl"], -1); // No expiration

        // Check TTL on nonexistent key
        let req = Request {
            cmd: "TTL".to_string(),
            key: Some("nonexistent".to_string()),
            value: None,
            seconds: None,
        };
        
        let resp = process_request(req, &store).await;
        assert_eq!(resp["ttl"], -2); // Nonexistent
    }

    #[tokio::test]
    async fn test_ping() {
        let store = Arc::new(Mutex::new(HashMap::new()));
        let req = Request {
            cmd: "PING".to_string(),
            key: None,
            value: None,
            seconds: None,
        };
        
        let resp = process_request(req, &store).await;
        assert_eq!(resp["status"], "ok");
    }
}
