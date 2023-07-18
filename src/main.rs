mod commands;
mod utils;

use dotenv::dotenv;
use poise::serenity_prelude as serenity;
use songbird::SerenityInit;

pub struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![commands::inspireme::inspireme()],
            pre_command: |ctx| {
                Box::pin(async move {
                    println!("Executing command: '{}'", ctx.command().qualified_name);
                })
            },
            ..Default::default()
        })
        .token(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
        .client_settings(|client_builder| client_builder.register_songbird())
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let manager = songbird::get(ctx.serenity_context())
                    .await
                    .expect("Songbird was not registered with the client builder")
                    .clone();

                manager.Ok(Data {})
            })
        });

    framework.run().await.unwrap();
}
