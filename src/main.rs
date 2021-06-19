use futures::StreamExt;
use regex::RegexBuilder;
use std::convert::TryFrom;
use std::time::Instant;
use std::{env, error::Error};
use twilight_gateway::{
  cluster::{Cluster, ShardScheme},
  Event,
};
use twilight_http::{request::AuditLogReason, Client as HttpClient};
use twilight_model::gateway::{
  payload::update_presence::UpdatePresencePayload,
  presence::{ActivityType, MinimalActivity, Status},
  Intents,
};
use twilight_util::snowflake::Snowflake;
use dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
  dotenv::dotenv().ok();

  let scheme = ShardScheme::Auto;
  let token = env::var("DISCORD_TOKEN")?;
  let intents = Intents::GUILD_MEMBERS;

  let (cluster, mut events) = Cluster::builder(&token, intents)
    .shard_scheme(scheme)
    .presence(UpdatePresencePayload::new(
      vec![MinimalActivity {
        kind: ActivityType::Listening,
        name: "idiots".into(),
        url: None,
      }
      .into()],
      false,
      None,
      Status::Offline,
    )?)
    .build()
    .await?;

  let cluster_spawn = cluster.clone();

  tokio::spawn(async move {
    cluster_spawn.up().await;
  });

  let http = HttpClient::new(&token);

  while let Some((shard_id, event)) = events.next().await {
    tokio::spawn(handle_event(shard_id, event, http.clone()));
  }

  Ok(())
}

async fn handle_event(
  shard_id: u64, event: Event, http: HttpClient,
) -> Result<(), Box<dyn Error + Send + Sync>> {
  match event {
    Event::MemberAdd(member) => {
      let regex = RegexBuilder::new("/token|john f|motion")
        .case_insensitive(true)
        .build()?;

      let username = member.user.name.to_lowercase();
      let reason = "User%20is%20most%20likely%20a%20spam%20account."; // https://github.com/twilight-rs/twilight/pull/803
      let now = Instant::now().elapsed().as_millis();

      if regex.is_match(&username) || now + 60000 > u128::try_from(member.user.id.timestamp())? {
        let ban = http
          .create_ban(member.guild_id, member.user.id)
          .delete_message_days(7)?
          .reason(reason)?
          .await;

        match ban {
          Ok(()) => println!(
            "Banned {}",
            format!("{}#{}", member.user.name, member.user.discriminator)
          ),
          Err(err) => println!("{}", err),
        }

        return Ok(());
      }
    }
    Event::ShardConnected(_) => {
      let user = http.current_user().await?;

      println!(
        "Shard {} connected with user {}",
        shard_id,
        format!("{}#{}", user.name, user.discriminator)
      );
    }
    _ => {}
  }

  Ok(())
}
