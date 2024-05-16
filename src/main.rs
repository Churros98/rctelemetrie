use futures::join;
use tokio::signal::unix::SignalKind;
use tokio::sync::watch;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal::unix::signal;
use std::error::Error;

use crate::commande::Commande;
use crate::sensors::Sensors;
use crate::actuator::Acurator;
use crate::telemetrie::Telemetrie;

mod actuator;
mod sensors;
mod telemetrie;
mod commande;
mod i2c;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Démarre les différentes tâches
    println!("[CORE] Démarrage ...");

    // Interupteur général d'arrêt
    let general_stop = Arc::new(AtomicBool::new(false));

    // Permet le passage des message entre le client de télémétrie et les capteurs/actionneurs
    let (sensors_tx, sensors_rx) = watch::channel::<sensors::SensorsData>(Sensors::empty());
    let (actuator_tx, actuator_rx) = watch::channel::<actuator::ActuatorData>(Acurator::empty());

    let sensors_tx = Arc::new(Mutex::new(sensors_tx));
    let actuator_tx = Arc::new(Mutex::new(actuator_tx));

    // IP de connexion au serveur
    let ip = String::from("192.168.1.102");

    // Prépare les différentes tâches
    let actuator = Acurator::new(general_stop.clone(), actuator_rx)?;
    let sensors = Sensors::new(general_stop.clone(), sensors_tx).await?;
    let telemetrie = Telemetrie::new(&ip, 1111, general_stop.clone(), sensors_rx)?;
    let control = Commande::new(&ip, 1112, general_stop.clone(), actuator_tx)?;

    // Démarre toutes les tâches
    let telemetrie_thread = tokio::spawn(async move {
        let telemetrie = telemetrie;
        let _ = telemetrie.update().await;
    });
    
    let control_thread = tokio::spawn(async move {
        let control = control;
        let _ = control.update().await;
    });
    
    let sensors_thread = tokio::spawn(async move {
        let mut sensors = sensors;
        let _ = sensors.update().await;
    });

    let actuator_thread = tokio::spawn(async move {
        let mut actuator = actuator;
        if actuator.update().await.is_err() {
            actuator.safe_stop();
        }
    });

    // Attend un SIGTERM pour executer le process de fermeture
    let mut sigint_event = signal(SignalKind::terminate()).expect("[CORE] Impossible d'enregister l'événement SIGINT");
    let mut sigterm_event = signal(SignalKind::terminate()).expect("[CORE] Impossible d'enregister l'événement SIGTERM");

    tokio::select! {
        _ = sigint_event.recv() => println!("[CORE] Arrêt demandé: SIGINT"),
        _ = sigterm_event.recv() => println!("[CORE] Arrêt demandé: SIGTERM"),
    }

    // Passe le "STOP" à True pour l'ensemble des boucles
    general_stop.store(true, Ordering::Relaxed);

    // Attend la fin de l'ensemble des tâches
    let _ = join!(sensors_thread, actuator_thread, telemetrie_thread, control_thread);
    println!("[CORE] Arrêt.");
    Ok(())
}
