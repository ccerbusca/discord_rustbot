use std::time::Duration;

use poise::serenity_prelude::ChannelId;
use songbird::input::restartable::Restartable;
use songbird::input::Input;
use songbird::{Event, TrackEvent};

use crate::utils::SongEmbedBuilder;
use crate::{utils, Context, Error};

/// Join voice channel
#[poise::command(slash_command)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().expect("Could not extract guild id");
    let connect_to = validate_voice_channel(&ctx)
        .await
        .expect("Not in a voice channel");

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird was not registered with the client builder")
        .clone();

    let (handle_lock, _) = manager.join(guild_id, connect_to).await;

    let mut handler = handle_lock.lock().await;

    let http = ctx.serenity_context().http.clone();

    handler.add_global_event(
        Event::Track(TrackEvent::End),
        utils::TrackEndNotifier {
            http,
            channel_id: ctx.channel_id(),
            handle_lock: handle_lock.clone(),
        },
    );

    utils::check_msg(ctx.send(|m| m.content("Let's jam!")).await);

    Ok(())
}

/// Plays specified song
#[poise::command(slash_command)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "YouTube Song name/URL"] song_url: String,
) -> Result<(), Error> {
    validate_voice_channel(&ctx).await;

    let url = url::Url::parse(&song_url).expect("Invalid URL");
    let guild_id = ctx.guild_id().expect("Could not extract guild id");
    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird was not registered with the client builder")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let _ = ctx.defer().await;

        let mut handler = handler_lock.lock().await;

        let lazy_source = match Restartable::ytdl(url.to_string(), true).await {
            Ok(source) => source,
            Err(why) => {
                println!("Err starting source: {:?}", why);
                utils::check_msg(ctx.send(|m| m.content("Error sourcing ffmpeg")).await);
                return Ok(());
            }
        };

        let source: Input = lazy_source.into();

        utils::check_msg(
            ctx.send(|m| m.build_embed_queued_up(source.metadata.as_ref().clone(), 1, 0))
                .await,
        );
        if handler.queue().is_empty() {
            utils::check_msg(
                ctx.send(|m| {
                    m.reply(false).build_embed_currently_playing(
                        source.metadata.as_ref().clone(),
                        Duration::from_secs(0),
                    )
                })
                .await,
            )
        }

        handler.enqueue_source(source);
    } else {
        utils::check_msg(ctx.send(|m| m.content("I'm not in a voice channel")).await);
    }

    Ok(())
}

/// Displays information about the currently played song
#[poise::command(slash_command)]
pub async fn current(ctx: Context<'_>) -> Result<(), Error> {
    validate_voice_channel(&ctx).await;

    let guild_id = ctx.guild_id().expect("Could not extract guild id");
    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird was not registered with the client builder")
        .clone();

    let handle = manager.get(guild_id).expect("Not in a voice channel");

    let handler = handle.lock().await;

    if let Some(track_handle) = handler.queue().current() {
        let elapsed = track_handle.get_info().await.unwrap().play_time;
        utils::check_msg(
            ctx.send(|m| m.build_embed_currently_playing(track_handle.metadata().clone(), elapsed))
                .await,
        )
    } else {
        utils::check_msg(ctx.send(|m| m.build_embed_empty_queue()).await)
    }
    Ok(())
}

/// Skip current song
#[poise::command(slash_command)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    validate_voice_channel(&ctx).await;

    let guild_id = ctx.guild_id().expect("Could not extract guild id");
    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird was not registered with the client builder")
        .clone();

    let handle = manager.get(guild_id).expect("Not in a voice channel");

    let handler = handle.lock().await;

    let _ = handler.queue().skip();
    Ok(())
}

/// Leave voice channel
#[poise::command(slash_command)]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    validate_voice_channel(&ctx).await;

    let guild_id = ctx.guild_id().expect("Could not extract guild id");
    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird was not registered with the client builder")
        .clone();

    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            println!("Failed leaving channel: '{:?}'", e);
            utils::check_msg(
                ctx.send(|m| m.content("Failed to leave voice channel"))
                    .await,
            );
        }

        utils::check_msg(ctx.send(|m| m.content("Bye 👋")).await);
    } else {
        utils::check_msg(ctx.send(|m| m.content("I'm not in a voice channel")).await);
    }

    Ok(())
}

async fn validate_voice_channel(ctx: &Context<'_>) -> Option<ChannelId> {
    let guild = ctx.guild().expect("Could not get the guild from context");
    let channel_id = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|voice_state| voice_state.channel_id);

    if channel_id.is_none() {
        utils::check_msg(
            ctx.send(|m| m.content("You have to be in a voice channel to execute commands!"))
                .await,
        );
    }

    channel_id
}
