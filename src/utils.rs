use crate::Context;
use poise::async_trait;
use poise::serenity_prelude::Error;
use songbird::{Call, Event, EventContext, EventHandler as VoiceEventHandler};
use std::ops::Sub;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub fn check_msg<T>(result: Result<T, Error>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

pub fn bold(s: String) -> String {
    format!("**{}**", s)
}

pub fn hyperlink(text: String, url: String) -> String {
    format!("[{}]({})", text, url)
}

pub trait SongEmbedBuilder<'a> {
    fn build_embed_queued_up(
        &mut self,
        metadata: &songbird::input::Metadata,
        position_in_queue: u32,
        seconds_until: u32,
    ) -> &mut Self;

    fn build_embed_currently_playing(
        &mut self,
        metadata: songbird::input::Metadata,
        seconds_elapsed: Duration,
    ) -> &mut Self;

    fn build_embed_empty_queue(&mut self) -> &mut Self {
        self.embed(|e| e.color(0xED4245).title("There are no songs in the queue"))
    }
}

impl<'a> SongEmbedBuilder<'a> for poise::CreateReply {
    fn build_embed_queued_up(
        &mut self,
        metadata: songbird::input::Metadata,
        position_in_queue: u32,
        seconds_until: u32,
    ) -> &mut Self {
        self.embed(|e| {
            e.color(0xA877C8)
                .title("New song queued up")
                .description(bold(hyperlink(
                    metadata.title.unwrap(),
                    metadata.source_url.unwrap(),
                )))
                .fields(vec![
                    ("Duration", metadata.duration.unwrap(), false),
                    ("Position in queue", position_in_queue, false),
                    ("Estimated time until song is played", seconds_until, false),
                ])
                .thumbnail(
                    metadata.thumbnail.unwrap_or(
                        "https://images.pexels.com/photos/11733110/pexels-photo-11733110.jpeg"
                            .to_string(),
                    ),
                )
        })
    }

    fn build_embed_currently_playing(
        &mut self,
        metadata: songbird::input::Metadata,
        seconds_elapsed: Duration,
    ) -> &mut Self {
        self.embed(|e| {
            e.color(0xA877C8)
                .description(bold(hyperlink(
                    metadata.title.unwrap(),
                    metadata.source_url.unwrap(),
                )))
                .fields(vec![(
                    "Time remaining",
                    metadata.duration.unwrap().sub(seconds_elapsed),
                    false,
                )])
                .thumbnail(
                    metadata.thumbnail.unwrap_or(
                        "https://images.pexels.com/photos/11733110/pexels-photo-11733110.jpeg"
                            .to_string(),
                    ),
                )
        })
    }
}

pub struct TrackEndNotifier {
    pub ctx: Context<'_>,
    pub handle_lock: Arc<Mutex<Call>>,
}

#[async_trait]
impl VoiceEventHandler for TrackEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(tracks) = ctx {
            let handler = self.handler_lock.lock().await;
            if let Some(np) = handler.queue().current() {
                let elapsed = np.get_info().await.unwrap().play_time;
                self.ctx
                    .send(|m| m.build_embed_currently_playing(np.metadata().clone(), elapsed));
            } else {
                self.ctx.send(|m| m.build_embed_empty_queue());
            }
        }
        None
    }
}

// pub trait CtxExtensions {
//     fn songbird_manager
// }
