use poise::futures_util::TryFutureExt;
use crate::{Context, Error};

#[poise::command(slash_command)]
pub async fn inspireme(ctx: Context<'_>) -> Result<(), Error> {
    let body = reqwest::get("https://inspirobot.me/api?generate=true")
        .and_then(|res| res.text())
        .await
        .map_err(|e| {
            println!("Error getting image: {}", e);
            "Something went wrong while contacting inspirobot".to_string()
        });

    match body {
        Ok(url) =>
            ctx.send(|m| {
                m.embed(|e| {
                    e.color(0xA877C8)
                        .image(url)
                })
            }).await,
        Err(e) =>
            ctx.send(|m| {
                m.content(e)
            }).await
    }?;


    Ok(())
}