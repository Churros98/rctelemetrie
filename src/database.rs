use surrealdb::engine::remote::ws::Client;
use surrealdb::engine::remote::ws::Wss;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

use crate::actuators::Control;
use crate::actuators::Switch;
use crate::cli::Cli;
use crate::sensors::reader::ModemData;
use crate::sensors::reader::SensorsData;

use crate::config::Config;

pub(crate) struct Database {
    db: Surreal<Client>,
    uuid: String,
}

impl Database {
    pub(crate) async fn new(args: Cli) -> anyhow::Result<Self> {
        let db = Surreal::new::<Wss>(&args.db_url).await?;

        db.signin(Root {
            username: &args.db_username.as_str(),
            password: &args.db_password.as_str(),
        }).await?;

        db.use_ns("voiturerc").use_db("voiturerc").await?;
        Ok(Self { db, uuid: args.uuid.replace("-", "") })
    }

    // Récupére la configuration de la voiture.
    pub(crate) async fn get_config(&self) -> anyhow::Result<Config> {
        let config: Option<Config> = self.db.select(("config", self.uuid.clone())).await?;
        match config {
            Some(cfg) => Ok(cfg),
            None => {
                println!("[DATABASE] Aucune configuration trouvée pour la voiture, création d'une configuration par défaut ...");
                self.db.insert::<Option<Config>>(("config", self.uuid.clone())).content(Config::new()).await?;
                Ok(Config::new())
            },
        }
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
            .update(("sensors", self.uuid.clone()))
            .content(data)
            .await?;
        
        Ok(())
    }

    // Mets l'intégralité des switchs à 0
    pub(crate) async fn reset_switchs(&self) -> anyhow::Result<()> {
        let _: Option<Switch> = self.db
        .update(("switch", self.uuid.as_str()))
        .content(Switch::empty())
        .await?;

        Ok(())
    }

    // Prépare un stream des switchs.
    pub(crate) async fn live_switch(
        &self,
    ) -> anyhow::Result<surrealdb::method::Stream<std::option::Option<Switch>>> {
        self.db
            .select(("switch", self.uuid.clone()))
            .live()
            .await
            .map_err(|x| anyhow::anyhow!(x))
    }

    // Prépare un stream des contrôles.
    pub(crate) async fn live_control(
        &self,
    ) -> anyhow::Result<surrealdb::method::Stream<std::option::Option<Control>>> {
        self.db
            .select(("control", self.uuid.clone()))
            .live()
            .await
            .map_err(|x| anyhow::anyhow!(x))
    }
}
