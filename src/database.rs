use nmea_parser::gnss::GgaData;
use nmea_parser::gnss::GgaQualityIndicator;
use nmea_parser::gnss::VtgData;
use surrealdb::engine::remote::ws::Client;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

use crate::actuators::Control;
use crate::sensors::analog::reader::Data as DataAnalog;
use crate::sensors::imu::reader::Data as DataIMU;
use crate::sensors::mag::reader::Data as DataMAG;

pub(crate) struct Database {
    db: Surreal<Client>,
}

impl Database {
    pub(crate) async fn new() -> anyhow::Result<Self> {
        let db = Surreal::new::<Ws>("192.168.1.100:8000").await?;

        db.signin(Root {
            username: "master",
            password: "rootkit",
        }).await?;

        db.use_ns("voiturerc").use_db("voiturerc").await?;
        
        Ok(Self { db })
    }

    // Envoi les données de l'IMU
    pub(crate) async fn send_imu(&self, data: DataIMU) -> anyhow::Result<()> {
        let mut result = self
            .db
            .query("UPDATE imu:realtime SET angles = $angles, temp = $temp;")
            .bind(("angles", data.angles))
            .bind(("temp", data.temp))
            .await?;

        if let Some(e) = result.take_errors().remove(&0) {
            return Err(anyhow::anyhow!(e));
        }

        Ok(())
    }

    // Envoi les données du capteur analogique
    pub(crate) async fn send_analog(&self, data: DataAnalog) -> anyhow::Result<()> {
        let mut result = self
            .db
            .query("UPDATE analog:realtime SET battery = $battery;")
            .bind(("battery", data.battery))
            .await?;

        if let Some(e) = result.take_errors().remove(&0) {
            return Err(anyhow::anyhow!(e));
        }

        Ok(())
    }

    // Envoi les données du capteur magnétique
    pub(crate) async fn send_mag(&self, data: DataMAG) -> anyhow::Result<()> {
        let mut result = self
            .db
            .query("UPDATE mag:realtime SET raw = $raw, heading = $heading;")
            .bind(("raw", data.raw))
            .bind(("heading", data.heading))
            .await?;

        if let Some(e) = result.take_errors().remove(&0) {
            return Err(anyhow::anyhow!(e));
        }

        Ok(())
    }

    // Envoi les données du GPS (Donnée GGA)
    pub(crate) async fn send_gps_gga(&self, data: GgaData) -> anyhow::Result<()> {
        let mut result = self
            .db
            .query("UPDATE gga:realtime SET latitude = $latitude, longitude = $longitude, satellite_count = $satellite_count, fix = $fix;")
            .bind(("latitude", data.latitude.unwrap_or(0.0)))
            .bind(("longitude", data.longitude.unwrap_or(0.0)))
            .bind(("satellite_count", data.satellite_count.unwrap_or(0)))
            .bind(("fix", data.quality == GgaQualityIndicator::GpsFix))
            .await?;

        if let Some(e) = result.take_errors().remove(&0) {
            return Err(anyhow::anyhow!(e));
        }

        Ok(())
    }

    // Envoi les données du GPS (Donnée VTG)
    pub(crate) async fn send_gps_vtg(&self, data: VtgData) -> anyhow::Result<()> {
        let mut result = self
            .db
            .query("UPDATE vtg:realtime SET speed = $speed;")
            .bind(("speed", data.sog_kph.unwrap_or(0.0)))
            .await?;

        if let Some(e) = result.take_errors().remove(&0) {
            return Err(anyhow::anyhow!(e));
        }

        Ok(())
    }

    // Prépare un stream des contrôles.
    pub(crate) async fn live_control(
        &self,
    ) -> anyhow::Result<surrealdb::method::Stream<'_, Client, std::option::Option<Control>>> {
        self.db
            .select(("control", "realtime"))
            .live()
            .await
            .map_err(|x| anyhow::anyhow!(x))
    }
}
