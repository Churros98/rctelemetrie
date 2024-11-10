use surrealdb::engine::remote::ws::Client;
use surrealdb::engine::remote::ws::Wss;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

use crate::actuators::Control;
use crate::actuators::Switch;
use crate::sensors::reader::AnalogData;
use crate::sensors::reader::GpsData;
use crate::sensors::reader::ImuData;
use crate::sensors::reader::MagData;

pub(crate) struct Database {
    db: Surreal<Client>,
}

impl Database {
    pub(crate) async fn new() -> anyhow::Result<Self> {
        let db = Surreal::new::<Wss>("db.theorywrong.me").await?;

        db.signin(Root {
            username: "master",
            password: "Iknowthisfuckingpassword",
        }).await?;

        db.use_ns("voiturerc").use_db("voiturerc").await?;
        
        Ok(Self { db })
    }

    // Envoi les données des différents capteurs analogiques.
    pub(crate) async fn send_analog(&self, data: AnalogData) -> anyhow::Result<()> {
        let mut result = self
            .db
            .query("UPDATE levels:realtime SET battery = $battery;")
            .bind(("battery", data.battery))
            .await?;

        if let Some(e) = result.take_errors().remove(&0) {
            return Err(anyhow::anyhow!(e));
        }

        Ok(())
    }

    // Envoi les données du modem
    pub(crate) async fn send_modem(&self, quality: u32) -> anyhow::Result<()> {
        let mut result = self
            .db
            .query("UPDATE modem:realtime SET quality = $quality;")
            .bind(("quality", quality))
            .await?;

        if let Some(e) = result.take_errors().remove(&0) {
            return Err(anyhow::anyhow!(e));
        }

        Ok(())
    }

    // Envoi les données de navigation.
    pub(crate) async fn send_nav(&self, gps_data: GpsData, mag_data: MagData, imu_data: ImuData) -> anyhow::Result<()> {
        let mut result = self
            .db
            .query("UPDATE nav:realtime SET latitude = $latitude, longitude = $longitude, satellite_count = $satellite_count, fix = $fix, speed = $speed, gps_heading = $gps_heading, mag_raw = $raw, mag_heading = $mag_heading, angles = $angles, temp = $temp;")
            .bind(("latitude", gps_data.latitude))
            .bind(("longitude", gps_data.longitude))
            .bind(("satellite_count", gps_data.satellites))
            .bind(("fix", gps_data.fix))
            .bind(("speed", gps_data.speed_kmh))
            .bind(("gps_heading", gps_data.heading))
            .bind(("mag_raw", mag_data.raw))
            .bind(("mag_heading", mag_data.heading))
            .bind(("angles", imu_data.angles))
            .bind(("temp", imu_data.temp))
            .await?;

        if let Some(e) = result.take_errors().remove(&0) {
            return Err(anyhow::anyhow!(e));
        }

        Ok(())
    }

    // Mets l'intégralité des switchs à 0
    pub(crate) async fn reset_switch(&self) -> anyhow::Result<()> {
        let mut result = self
            .db
            .query("UPDATE switch:realtime SET esc = $esc;")
            .bind(("esc", false))
            .await?;

        if let Some(e) = result.take_errors().remove(&0) {
            return Err(anyhow::anyhow!(e));
        }

        Ok(())
    }

    // Prépare un stream des switchs.
    pub(crate) async fn live_switch(
        &self,
    ) -> anyhow::Result<surrealdb::method::Stream<'_, Client, std::option::Option<Switch>>> {
        self.db
            .select(("switch", "realtime"))
            .live()
            .await
            .map_err(|x| anyhow::anyhow!(x))
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
