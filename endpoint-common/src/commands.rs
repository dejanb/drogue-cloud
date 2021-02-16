use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::Id;
use tokio::sync::mpsc::{channel, Sender, Receiver};

/// Represents command message passed to the actors
#[derive(Clone)]
pub struct Command {
    pub device_id: Id,
    pub command: String,
}

#[derive(Clone)]
pub struct Commands {
    pub devices: Arc<Mutex<HashMap<Id, Sender<String>>>>,
}

impl Commands {

    pub fn new() -> Self {
        Self {
            devices: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn send(&self, msg: Command) -> Result<(), String> {
        let device = { self.devices.lock().unwrap().get(&msg.device_id).cloned() };
        if let Some(sender) = device {
            match sender.send(msg.command).await {
                Ok(_) => {
                    log::debug!("Command sent to device {:?}", msg.device_id);
                    Ok(())
                }
                Err(e) => {
                    log::error!("Failed to send a command {:?}", e);
                    Err(e.to_string())
                }
            }
        } else {
            log::debug!(
                "Failed to route command: No device {:?} found on this endpoint!",
                msg.device_id
            );
            Err("Device not found".to_string())
        }
    }

    pub fn subscribe(&self, device_id: Id) -> Receiver<String>{
        let (tx, rx) = channel(32);
        let mut devices = self.devices.lock().unwrap();
        devices.insert(
            device_id.clone(),
            tx.clone(),
        );
        log::debug!("Device {:?} subscribed to receive commands",  device_id);
        rx
    }

    pub fn unsubscribe(&self, device_id: Id) {
        let mut devices = self.devices.lock().unwrap();
        devices.remove(&device_id.clone());
        log::debug!("Device {:?} unsubscribed from receiving commands",  device_id);
    }

}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_timeout() {
        env_logger::init();
        let id = Id::new("test", "test");

        let commands = Commands::new();
        let msg = Command {
            device_id: id.clone(),
            command: "test".to_string(),
        };

        let mut receiver = commands.subscribe(id.clone());

        let success_handle = tokio::spawn(async move {
            let cmd = timeout(Duration::from_secs(1), receiver.recv()).await;
            assert_eq!(cmd, Ok(Some("test".to_string())));
            let cmd2 = timeout(Duration::from_secs(1), receiver.recv()).await;
            assert_eq!(cmd2.is_err(), true);
        });

        commands.clone().send(msg.clone()).await.ok();

        success_handle.await.unwrap();

    }

}

