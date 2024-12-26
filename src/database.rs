use surrealdb::engine::remote::ws::Client;
use surrealdb::engine::remote::ws::Wss;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use uuid::Uuid;

use crate::actuators::Control;
use crate::actuators::Switch;
use crate::sensors::reader::ModemData;
use crate::sensors::reader::SensorsData;

pub(crate) struct Database {
    db: Surreal<Client>,
    uuid: String,
}

impl Database {
    pub(crate) async fn new(uuid: Uuid) -> anyhow::Result<Self> {
        let db = Surreal::new::<Wss>(env!("DB_URL")).await?;

        db.signin(Root {
            username: env!("DB_USERNAME"),
            password: env!("DB_PASSWORD"),
        }).await?;

        db.use_ns("voiturerc").use_db("voiturerc").await?;
        
        Ok(Self { db, uuid: uuid.to_string().replace("-", "") })
    }

    // Envoi les données du modem
    pub(crate) async fn send_modem(&self, quality: u32) -> anyhow::Result<()> {
        let _: Option<ModemData> = self
            .db
            .update(("modem", self.uuid.clone()))
            .content(ModemData { quality })
            .await?;

        Ok(())
    }

    // Envoi les données des capteurs.
    pub(crate) async fn send_sensors(&self, data: SensorsData) -> anyhow::Result<()> {
        let _: Option<SensorsData> = self
            .db
            .update(("nav", self.uuid.clone()))
            .content(data)
            .await?;
        
        Ok(())
    }

    // Mets l'intégralité des switchs à 0
    pub(crate) async fn reset_switchs(&self) -> anyhow::Result<()> {
        let _: Option<Switch> = self.db
        .update(("switch", self.uuid.as_str()))
        .content(Switch { esc: false })
        .await?;

        Ok(())
    }

    // Prépare un stream des switchs.
    pub(crate) async fn live_switch(
        &self,
    ) -> anyhow::Result<surrealdb::method::Stream<'_, Client, std::option::Option<Switch>>> {
        self.db
            .select(("switch", self.uuid.clone()))
            .live()
            .await
            .map_err(|x| anyhow::anyhow!(x))
    }

    // Prépare un stream des contrôles.
    pub(crate) async fn live_control(
        &self,
    ) -> anyhow::Result<surrealdb::method::Stream<'_, Client, std::option::Option<Control>>> {
        self.db
            .select(("control", self.uuid.clone()))
            .live()
            .await
            .map_err(|x| anyhow::anyhow!(x))
    }
}
