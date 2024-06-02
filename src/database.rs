use surrealdb::engine::remote::ws::Client;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

use crate::sensors::analog::reader::Data as DataAnalog;
use crate::sensors::gps::DataGGA;
use crate::sensors::imu::reader::Data as DataIMU;
use crate::sensors::mag::reader::Data as DataMAG;

pub (crate) struct Database {
    db: Surreal<Client>,
}

impl Database {
    pub (crate) async fn new() -> anyhow::Result<Self> {
        let db = Surreal::new::<Ws>("127.0.0.1:8000").await?;

        db.signin(Root {
            username: "root",
            password: "root",
        })
        .await?;

        db.use_ns("voiturerc").use_db("voiturerc").await?;

        Ok(Self {
            db
        })
    }

    // Envoi les données de l'IMU
    pub (crate) async fn send_imu(&self, data: DataIMU) -> anyhow::Result<()> {
        self.db.create::<Option<DataIMU>>(("sensor", "imu")).content(data).await?;
        Ok(())
    }

    // Envoi les données du capteur analogique
    pub (crate) async fn send_analog(&self, data: DataAnalog) -> anyhow::Result<()> {
        self.db.create::<Option<DataAnalog>>(("sensor", "analog")).content(data).await?;
        Ok(())
    }

    // Envoi les données du capteur magnétique
    pub (crate) async fn send_mag(&self, data: DataMAG) -> anyhow::Result<()> {
        self.db.create::<Option<DataMAG>>(("sensor", "mag")).content(data).await?;
        Ok(())
    }

    // Envoi les données du GPS (Donnée GGA)
    pub (crate) async fn send_gps_gga(&self, data: DataGGA) -> anyhow::Result<()> {
        self.db.create::<Option<DataGGA>>(("sensor", "gps")).content(data).await?;
        Ok(())
    }
}