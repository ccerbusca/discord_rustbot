use std::time::Duration;

use poise::serenity_prelude::{CacheHttp, ChannelId};
use songbird::input::restartable::Restartable;
use songbird::input::Input;
use songbird::tracks::TrackQueue;
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

    let (_, _) = manager.join(guild_id, connect_to).await;

    // let mut handler = handle_lock.lock().await;
    //
    // let http = ctx.serenity_context().http.clone();
    //
    // handler.add_global_event(
    //     Event::Track(TrackEvent::End),
    //     utils::TrackEndNotifier {
    //         http,
    //         channel_id: ctx.channel_id(),
    //         handle_lock: handle_lock.clone(),
    //     },
    // );

    utils::check_msg(ctx.send(|m| m.content("Let's jam!")).await);

    Ok(())
}

/// Plays specified song
#[poise::command(slash_command)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "YouTube Song name/URL"] song_url: String,
) -> Result<(), Error> {
    let connect_to = validate_voice_channel(&ctx)
        .await
        .expect("User not in a channel");
    let guild_id = ctx.guild_id().expect("Could not extract guild id");
    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird was not registered with the client builder")
        .clone();

    let handle_mutex = match manager.get(guild_id) {
        Some(handle) => handle,
        None => {
            let (handle, _) = manager.join(guild_id, connect_to).await;
            handle
        }
    };

    let mut handler = handle_mutex.lock().await;

    let _ = ctx.defer().await;

    let source = build_lazy_source(ctx, song_url)
        .await
        .expect("Failed to build source");

    build_embeds_on_play(ctx, handler.queue(), source.metadata.as_ref()).await;

    let track_handle = handler.enqueue_source(source);

    let http = ctx.serenity_context().http.clone();

    let _ = track_handle.add_event(
        Event::Track(TrackEvent::End),
        utils::TrackEndNotifier {
            http,
            channel_id: ctx.channel_id(),
            handle_lock: handle_mutex.clone(),
        },
    );

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

async fn build_lazy_source(
    ctx: Context<'_>,
    song_url: String,
) -> songbird::input::error::Result<Input> {
    let url = url::Url::parse(&song_url);
    let lazy_source = match url {
        Ok(_) => Restartable::ytdl(song_url, true).await,
        Err(_) => Restartable::ytdl_search(song_url, true).await,
    };

    if lazy_source.is_err() {
        println!(
            "Err starting source: {:?}",
            lazy_source.as_ref().unwrap_err()
        );
        utils::check_msg(
            ctx.send(|m| m.content("An error occurred while constructing the source"))
                .await,
        );
    }

    lazy_source.map(|source| source.into())
}

async fn build_embeds_on_play(
    ctx: Context<'_>,
    queue: &TrackQueue,
    metadata: &songbird::input::Metadata,
) {
    let position_in_queue: u64 = (queue.len() + 1) as u64;
    let elapsed = match queue.current() {
        Some(track) => track.get_info().await.unwrap().play_time.as_secs(),
        None => 0,
    };
    let seconds_until = queue
        .current_queue()
        .iter()
        .fold(0, |acc, e| acc + e.metadata().duration.unwrap().as_secs())
        - elapsed;

    utils::check_msg(
        ctx.send(|m| m.build_embed_queued_up(metadata.clone(), position_in_queue, seconds_until))
            .await,
    );
    if queue.is_empty() {
        utils::check_msg(
            ctx.channel_id()
                .send_message(ctx.http(), |m| {
                    m.build_embed_currently_playing(metadata.clone(), Duration::from_secs(0))
                })
                .await,
        )
    }
}
