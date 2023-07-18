use std::ops::Sub;
use std::sync::Arc;
use std::time::Duration;

use poise::async_trait;
use poise::serenity_prelude::{ChannelId, CreateEmbed, CreateMessage, Error, Http};
use songbird::{Call, Event, EventContext, EventHandler as VoiceEventHandler};
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

pub fn format_duration(d: Duration) -> String {
    let minutes = d.as_secs() / 60;
    let seconds = d.as_secs() % 60;
    format!("{:0>2}:{:0>2}", minutes, seconds)
}

pub trait SongEmbedBuilder {
    fn build_embed_queued_up(
        &mut self,
        metadata: songbird::input::Metadata,
        position_in_queue: u64,
        seconds_until: u64,
    ) -> &mut Self;

    fn build_embed_currently_playing(
        &mut self,
        metadata: songbird::input::Metadata,
        seconds_elapsed: Duration,
    ) -> &mut Self;

    fn build_embed_empty_queue(&mut self) -> &mut Self;
}

impl<'a> SongEmbedBuilder for poise::CreateReply<'a> {
    fn build_embed_queued_up(
        &mut self,
        metadata: songbird::input::Metadata,
        position_in_queue: u64,
        seconds_until: u64,
    ) -> &mut Self {
        self.embed(|e| embed_queued_up(e, metadata, position_in_queue, seconds_until))
    }

    fn build_embed_currently_playing(
        &mut self,
        metadata: songbird::input::Metadata,
        seconds_elapsed: Duration,
    ) -> &mut Self {
        self.embed(|e| currently_playing(e, metadata, seconds_elapsed))
    }

    fn build_embed_empty_queue(&mut self) -> &mut Self {
        self.embed(|e| e.color(0xED4245).title("There are no songs in the queue"))
    }
}

impl<'a> SongEmbedBuilder for CreateMessage<'a> {
    fn build_embed_queued_up(
        &mut self,
        metadata: songbird::input::Metadata,
        position_in_queue: u64,
        seconds_until: u64,
    ) -> &mut Self {
        self.embed(|e| embed_queued_up(e, metadata, position_in_queue, seconds_until))
    }

    fn build_embed_currently_playing(
        &mut self,
        metadata: songbird::input::Metadata,
        seconds_elapsed: Duration,
    ) -> &mut Self {
        self.embed(|e| currently_playing(e, metadata, seconds_elapsed))
    }

    fn build_embed_empty_queue(&mut self) -> &mut Self {
        self.embed(|e| {
            e.color(0xED4245)
                .description("There are no songs in the queue")
        })
    }
}

fn embed_queued_up(
    e: &mut CreateEmbed,
    metadata: songbird::input::Metadata,
    position_in_queue: u64,
    seconds_until: u64,
) -> &mut CreateEmbed {
    e.color(0xA877C8)
        .title("New song queued up")
        .description(bold(hyperlink(
            metadata.title.unwrap(),
            metadata.source_url.unwrap(),
        )))
        .fields(vec![
            (
                "Duration",
                format_duration(metadata.duration.unwrap()),
                false,
            ),
            (
                "Position in queue",
                if position_in_queue == 1 {
                    "Next".to_string()
                } else {
                    position_in_queue.to_string()
                },
                false,
            ),
            (
                "Estimated time until song is played",
                format_duration(Duration::from_secs(seconds_until)),
                false,
            ),
        ])
        .thumbnail(metadata.thumbnail.unwrap_or(
            "https://images.pexels.com/photos/11733110/pexels-photo-11733110.jpeg".to_string(),
        ))
}

fn currently_playing(
    e: &mut CreateEmbed,
    metadata: songbird::input::Metadata,
    seconds_elapsed: Duration,
) -> &mut CreateEmbed {
    e.color(0xA877C8)
        .description(format!(
            "Now playing: {}",
            bold(hyperlink(
                metadata.title.unwrap(),
                metadata.source_url.unwrap(),
            ))
        ))
        .fields(vec![(
            "Time remaining",
            format_duration(metadata.duration.unwrap().sub(seconds_elapsed)),
            false,
        )])
        .thumbnail(metadata.thumbnail.unwrap_or(
            "https://images.pexels.com/photos/11733110/pexels-photo-11733110.jpeg".to_string(),
        ))
}

pub struct TrackEndNotifier {
    pub channel_id: ChannelId,
    pub http: Arc<Http>,
    pub handle_lock: Arc<Mutex<Call>>,
}

#[async_trait]
impl VoiceEventHandler for TrackEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(_tracks) = ctx {
            let handler = self.handle_lock.lock().await;
            if let Some(np) = handler.queue().current() {
                let elapsed = np.get_info().await.unwrap().play_time;
                check_msg(
                    self.channel_id
                        .send_message(&self.http, |m| {
                            m.build_embed_currently_playing(np.metadata().clone(), elapsed)
                        })
                        .await,
                );
            } else {
                check_msg(
                    self.channel_id
                        .send_message(&self.http, |m| m.build_embed_empty_queue())
                        .await,
                );
            }
        }
        None
    }
}
